use crate::{
  frame_drawer::FrameDrawer, logger::Logger, scaling_util::ScalingUtil,
  window_corner_util::WindowCornerUtil, Hinstance, Hwnd, CLASS_NAME,
};
use winapi::{
  ctypes::c_void,
  shared::{
    minwindef::LPARAM,
    minwindef::{LPVOID, LRESULT, WPARAM},
    windef::{COLORREF, HWND, RECT},
    winerror::SUCCEEDED,
  },
  um::{
    dwmapi::{
      DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_EXCLUDED_FROM_PEEK,
      DWMWA_EXTENDED_FRAME_BOUNDS,
    },
    winuser::{
      CreateWindowExW, DefWindowProcW, DestroyWindow, GetForegroundWindow, GetWindowLongPtrW,
      KillTimer, LoadCursorW, RegisterClassExW, SetLayeredWindowAttributes, SetTimer,
      SetWindowLongPtrW, SetWindowPos, GWLP_USERDATA, IDC_ARROW, LWA_COLORKEY, SWP_NOACTIVATE,
      SWP_NOMOVE, SWP_NOREDRAW, SWP_NOSIZE, WM_ERASEBKGND, WM_NCDESTROY, WM_SETCURSOR, WM_TIMER,
      WNDCLASSEXW, WS_DISABLED, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP,
    },
  },
};

const REFRESH_BORDER_TIMER_ID: usize = 123;
const REFRESH_BORDER_INTERVAL: u32 = 100;

pub struct WindowBorder {
  pub window: Option<Hwnd>,
  hinstance: Hinstance,
  tracking_window: Hwnd,
  frame_drawer: Option<FrameDrawer>,
}

impl Drop for WindowBorder {
  fn drop(&mut self) {
    self.frame_drawer = None;

    if let Some(window) = self.window.as_ref() {
      unsafe {
        SetWindowLongPtrW(window.0, GWLP_USERDATA, 0);
        DestroyWindow(window.0);
      }
    }
  }
}

impl WindowBorder {
  pub fn new(window: Hwnd, hinstance: Hinstance) -> Option<Self> {
    let mut border = Self {
      window: None,
      hinstance,
      tracking_window: window,
      frame_drawer: None,
    };
    if border.init() {
      return Some(border);
    }

    None
  }

  fn init(&mut self) -> bool {
    if self.tracking_window.0.is_null() {
      return false;
    }

    let window_rect_opt = get_frame_rect(&self.tracking_window);
    if window_rect_opt.is_none() {
      return false;
    }
    let window_rect = window_rect_opt.unwrap();

    let mut wcex: WNDCLASSEXW = unsafe { std::mem::zeroed() };
    wcex.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
    wcex.lpfnWndProc = Some(wnd_proc);
    wcex.hInstance = self.hinstance.0;
    wcex.lpszClassName = CLASS_NAME.as_ptr();
    wcex.hCursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
    unsafe { RegisterClassExW(&wcex) };

    let window_hwnd = unsafe {
      CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOOLWINDOW,
        CLASS_NAME.as_ptr(),
        std::ptr::null_mut(),
        WS_POPUP | WS_DISABLED,
        window_rect.left,
        window_rect.top,
        window_rect.right - window_rect.left,
        window_rect.bottom - window_rect.top,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        self.hinstance.0,
        self as *mut WindowBorder as LPVOID,
      )
    };

    if window_hwnd.is_null() {
      return false;
    }

    self.window = Some(Hwnd(window_hwnd));
    let window = self.window.as_ref().unwrap();

    if unsafe { SetLayeredWindowAttributes(window.0, 0, 0, LWA_COLORKEY) } == 0 {
      return false;
    }

    unsafe {
      SetWindowPos(
        self.tracking_window.0,
        window.0,
        window_rect.left,
        window_rect.top,
        window_rect.right - window_rect.left,
        window_rect.bottom - window_rect.top,
        SWP_NOMOVE | SWP_NOSIZE,
      );
    }

    let val = true;
    unsafe {
      DwmSetWindowAttribute(
        window.0,
        DWMWA_EXCLUDED_FROM_PEEK,
        &val as *const _ as *const c_void,
        std::mem::size_of::<bool>() as u32,
      );
    }

    if let Some(frame_drawer) = FrameDrawer::new(window.clone()) {
      self.frame_drawer = Some(frame_drawer);
      self.frame_drawer.as_mut().unwrap().show();
      unsafe {
        SetTimer(
          window.0,
          REFRESH_BORDER_TIMER_ID,
          REFRESH_BORDER_INTERVAL,
          None,
        );
      }
      self.update_border_position();
      self.update_border_properties();

      true
    } else {
      false
    }
  }

  pub fn update_border_position(&self) {
    if self.tracking_window.0.is_null() {
      return;
    }

    let rect_opt = get_frame_rect(&self.tracking_window);
    if rect_opt.is_none() {
      if let Some(frame_drawer) = self.frame_drawer.as_ref() {
        frame_drawer.hide();
      }
      return;
    }

    let rect = rect_opt.unwrap();
    if let Some(window) = self.window.as_ref() {
      unsafe {
        SetWindowPos(
          window.0,
          self.tracking_window.0,
          rect.left,
          rect.top,
          rect.right - rect.left,
          rect.bottom - rect.top,
          SWP_NOREDRAW | SWP_NOACTIVATE,
        );
      }
    }
  }

  pub fn update_border_properties(&mut self) {
    if self.tracking_window.0.is_null() || self.frame_drawer.is_none() || self.window.is_none() {
      return;
    }
    let frame_drawer = self.frame_drawer.as_mut().unwrap();
    let window = self.window.as_ref().unwrap();

    let window_rect_opt = get_frame_rect(&self.tracking_window);
    if window_rect_opt.is_none() {
      return;
    }

    let window_rect = window_rect_opt.unwrap();
    unsafe {
      SetWindowPos(
        window.0,
        self.tracking_window.0,
        window_rect.left,
        window_rect.top,
        window_rect.right - window_rect.left,
        window_rect.bottom - window_rect.top,
        SWP_NOREDRAW | SWP_NOACTIVATE,
      );

      let frame_rect = RECT {
        left: 0,
        top: 0,
        right: window_rect.right - window_rect.left,
        bottom: window_rect.bottom - window_rect.top,
      };

      let fg = GetForegroundWindow();
      let color = if fg == self.tracking_window.0 {
        hex_to_rgb("#c6a0f6")
      } else {
        hex_to_rgb("#ffffff")
      };

      let scaling_factor = ScalingUtil::scaling_factor(&self.tracking_window);
      let thickness = 4.0 * scaling_factor;
      let square_borders = false;
      let corner_radius = if square_borders {
        0.0
      } else {
        WindowCornerUtil::corners_radius(self.tracking_window.0) as f32 * scaling_factor
      };

      frame_drawer.set_border_rect(frame_rect, color, thickness as u32, corner_radius);
    }
  }

  fn wnd_proc(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
      WM_TIMER => match wparam {
        REFRESH_BORDER_TIMER_ID => {
          self.update_border_position();
          self.update_border_properties();
          unsafe {
            KillTimer(hwnd, REFRESH_BORDER_TIMER_ID);
            SetTimer(hwnd, REFRESH_BORDER_TIMER_ID, REFRESH_BORDER_INTERVAL, None);
          }
          0
        }
        _ => 0,
      },
      WM_NCDESTROY => {
        unsafe {
          KillTimer(hwnd, REFRESH_BORDER_TIMER_ID);
        }
        unsafe {
          DefWindowProcW(hwnd, msg, wparam, lparam);
          SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        }
        0
      }
      WM_ERASEBKGND => 1,
      WM_SETCURSOR => 1,
      _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
  }
}

unsafe extern "system" fn wnd_proc(
  hwnd: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  let self_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut c_void;

  /* if self_ptr.is_null() && msg == WM_CREATE {
    let create_struct = lparam as *const CREATESTRUCTW;
    let ptr = (*create_struct).lpCreateParams;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
  } */

  if !self_ptr.is_null() {
    let window_border = &mut *(self_ptr as *mut WindowBorder);
    window_border.wnd_proc(hwnd, msg, wparam, lparam)
  } else {
    DefWindowProcW(hwnd, msg, wparam, lparam)
  }
}

fn get_frame_rect(window: &Hwnd) -> Option<RECT> {
  let mut rect: RECT = RECT {
    bottom: 0,
    left: 0,
    right: 0,
    top: 0,
  };
  unsafe {
    if !SUCCEEDED(DwmGetWindowAttribute(
      window.0,
      DWMWA_EXTENDED_FRAME_BOUNDS,
      &mut rect as *mut _ as *mut c_void,
      std::mem::size_of::<RECT>() as u32,
    )) {
      return None;
    }
  }

  let thickness = 4;
  rect.top -= thickness;
  rect.left -= thickness;
  rect.right += thickness;
  rect.bottom += thickness;

  Some(rect)
}

pub fn hex_to_rgb(hex: &str) -> COLORREF {
  if hex.len() != 7 || !hex.starts_with('#') {
    Logger::log(&format!("[ERROR] Invalid hex: {}", hex));
    return 0xFFFFFFFF;
  }

  let r = u8::from_str_radix(&hex[1..3], 16);
  let g = u8::from_str_radix(&hex[3..5], 16);
  let b = u8::from_str_radix(&hex[5..7], 16);

  match (r, g, b) {
    (Ok(r), Ok(g), Ok(b)) => (b as u32) << 16 | (g as u32) << 8 | r as u32,
    _ => {
      Logger::log(&format!("[ERROR] Invalid hex: {}", hex));
      0xFFFFFFFF
    }
  }
}
