// #![windows_subsystem = "windows"]

use cleanup::Cleanup;
use lazy_static::lazy_static;
use std::{
  collections::HashMap,
  ffi::{OsStr, OsString},
  os::windows::prelude::{OsStrExt, OsStringExt},
  sync::Mutex,
};
use tray_util::TrayUtil;
use winapi::{
  shared::{
    minwindef::{BOOL, DWORD, HINSTANCE, LPARAM},
    windef::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, HWINEVENTHOOK, HWND},
    winerror::SUCCEEDED,
  },
  um::{
    combaseapi::{CoInitializeEx, CoUninitialize},
    libloaderapi::GetModuleHandleW,
    winuser::{
      DispatchMessageW, EnumWindows, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW,
      GetWindowTextW, IsWindowVisible, SetProcessDpiAwarenessContext, SetWinEventHook,
      SetWindowLongPtrW, TranslateMessage, UnhookWinEvent, UnregisterClassW, EVENT_OBJECT_CREATE,
      EVENT_OBJECT_DESTROY, EVENT_OBJECT_FOCUS, EVENT_OBJECT_LOCATIONCHANGE,
      EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
      EVENT_SYSTEM_MOVESIZEEND, GWLP_USERDATA, GWL_EXSTYLE, WINEVENT_OUTOFCONTEXT,
      WINEVENT_SKIPOWNPROCESS, WS_EX_TOOLWINDOW,
    },
  },
};
use window_border::WindowBorder;

use crate::logger::Logger;

mod cleanup;
mod config;
mod file_util;
mod frame_drawer;
mod logger;
mod scaling_util;
mod startup_util;
mod tray_util;
mod window_border;
mod window_corner_util;

lazy_static! {
  static ref CUTE_BORDERS: Mutex<CuteBorders> = Mutex::new(CuteBorders::new());
  static ref CLASS_NAME: Vec<u16> = get_wide_str("CuteBordersBorder");
}

fn get_wide_str(string: &str) -> Vec<u16> {
  OsStr::new(string)
    .encode_wide()
    .chain(std::iter::once(0))
    .collect()
}

fn main() {
  let mut cleanup = Cleanup::default();
  cleanup.add(|| {
    CUTE_BORDERS.lock().unwrap().drop_and_exit();
  });
  ctrlc::set_handler(|| {
    CUTE_BORDERS.lock().unwrap().drop_and_exit();
  })
  .expect("Failed to set ctrl+c handler");

  #[allow(unused_variables)]
  let tray_icon = TrayUtil::init();
  lazy_static::initialize(&CUTE_BORDERS);

  unsafe {
    let mut msg = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
      TranslateMessage(&msg);
      DispatchMessageW(&msg);
    }
  }
}

#[derive(PartialEq, Hash, Clone, Debug)]
pub struct Hwnd(pub HWND);
unsafe impl Send for Hwnd {}
impl Eq for Hwnd {}

#[derive(PartialEq, Hash, Clone, Debug)]
pub struct Hinstance(pub HINSTANCE);
unsafe impl Send for Hinstance {}

struct Hwineventhook(pub HWINEVENTHOOK);
unsafe impl Send for Hwineventhook {}

struct Wb(pub *mut WindowBorder);
unsafe impl Send for Wb {}

struct CuteBorders {
  tracked_windows: HashMap<Hwnd, Option<Wb>>,
  static_win_event_hooks: Vec<Hwineventhook>,
  hinstance: Hinstance,
}

impl CuteBorders {
  fn drop_and_exit(&mut self) -> ! {
    unsafe {
      CoUninitialize();
      UnregisterClassW(CLASS_NAME.as_ptr(), self.hinstance.0);
    }

    for (_, border) in self.tracked_windows.iter() {
      if let Some(border) = border {
        let border = unsafe { Box::from_raw(border.0 as *mut WindowBorder) };
        drop(border);
      }
    }
    self.tracked_windows.clear();

    for hook in self.static_win_event_hooks.iter() {
      unsafe { UnhookWinEvent(hook.0) };
    }

    std::process::exit(0);
  }

  fn new() -> Self {
    let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if h_instance.is_null() {
      Logger::log("[ERROR] Failed to get HINSTANCE");
      std::process::exit(1);
    } else {
      let mut cuteborders = Self {
        tracked_windows: HashMap::new(),
        static_win_event_hooks: vec![],
        hinstance: Hinstance(h_instance),
      };

      cuteborders.init();
      cuteborders
    }
  }

  fn init(&mut self) {
    let hr = unsafe {
      CoInitializeEx(
        std::ptr::null_mut(),
        0x2, /* COINIT_APARTMENTTHREADED */
      )
    };

    if !SUCCEEDED(hr) {
      Logger::log("[ERROR] Failed to initialize COM");
      std::process::exit(1);
    }

    unsafe {
      SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    self.subscribe_to_events();
    self.start_tracking_windows();
  }

  fn subscribe_to_events(&mut self) {
    let events_to_subscribe = vec![
      EVENT_OBJECT_LOCATIONCHANGE,
      EVENT_SYSTEM_MINIMIZESTART,
      EVENT_SYSTEM_MINIMIZEEND,
      EVENT_SYSTEM_MOVESIZEEND,
      EVENT_SYSTEM_FOREGROUND,
      EVENT_OBJECT_DESTROY,
      EVENT_OBJECT_CREATE,
      EVENT_OBJECT_FOCUS,
    ];

    for event in events_to_subscribe {
      let hook = unsafe {
        SetWinEventHook(
          event,
          event,
          std::ptr::null_mut(),
          Some(win_hook_proc),
          0,
          0,
          WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
      };

      if hook.is_null() {
        Logger::log("[ERROR] Failed to register hooks");
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      } else {
        self.static_win_event_hooks.push(Hwineventhook(hook));
      }
    }
  }

  fn start_tracking_windows(&mut self) {
    let mut visible_windows: Vec<Hwnd> = Vec::new();
    unsafe {
      EnumWindows(
        Some(enum_windows_callback),
        &mut visible_windows as *mut _ as LPARAM,
      );
    }

    for window in visible_windows {
      self.assign_border(window);
    }
  }

  fn assign_border(&mut self, window: Hwnd) {
    let is_window_on_current_desktop = true; // TODO: missing actual implementation

    if is_window_on_current_desktop {
      if let Some(border) = WindowBorder::new(window.clone(), self.hinstance.clone()) {
        let w = border.window.clone().unwrap();
        let ye = Box::into_raw(Box::new(border));
        self.tracked_windows.insert(window, Some(Wb(ye)));

        unsafe { SetWindowLongPtrW(w.0, GWLP_USERDATA, ye as *mut _ as isize) };
      }
    } else {
      self.tracked_windows.remove(&window);
    }
  }

  fn handle_win_hook_event(&mut self, event: DWORD, hwnd: Hwnd) {
    self
      .tracked_windows
      .retain(|window, _| unsafe { IsWindowVisible(window.0) } == 1);

    match event {
      EVENT_OBJECT_CREATE => {
        if should_add_border(&hwnd) {
          self.assign_border(hwnd);
        }
      }
      EVENT_OBJECT_LOCATIONCHANGE => {
        if let Some(&Some(border)) = self.tracked_windows.get(&hwnd).as_ref() {
          let border = unsafe { &mut *(border.0 as *mut WindowBorder) };
          border.update_border_position();
        }
      }
      EVENT_SYSTEM_MINIMIZEEND => {}
      EVENT_SYSTEM_MOVESIZEEND => {
        if let Some(&Some(border)) = self.tracked_windows.get(&hwnd).as_ref() {
          let border = unsafe { &mut *(border.0 as *mut WindowBorder) };
          border.update_border_position();
        }
      }
      EVENT_SYSTEM_FOREGROUND => {
        self.refresh_borders();
        for (_, border) in self.tracked_windows.iter() {
          if let Some(border) = border {
            let border = unsafe { &mut *(border.0 as *mut WindowBorder) };
            border.update_border_properties();
          }
        }
      }
      EVENT_OBJECT_FOCUS => {}
      _ => {}
    }
  }

  fn refresh_borders(&mut self) {
    let mut changes: Vec<(Hwnd, Option<bool>)> = Vec::new();

    for (window, border) in self.tracked_windows.iter() {
      let is_window_on_current_desktop = true; // TODO: missing actual implementation
      if is_window_on_current_desktop {
        if border.is_none() {
          changes.push((window.clone(), Some(true)));
        }
      } else if border.is_some() {
        changes.push((window.clone(), None));
      }
    }

    for (window, new_border) in changes {
      if new_border.is_some() {
        self.assign_border(window.clone());
      } else {
        self.tracked_windows.insert(window.clone(), None);
      }
    }
  }
}

fn should_add_border(hwnd: &Hwnd) -> bool {
  unsafe {
    if IsWindowVisible(hwnd.0) == 0 {
      return false;
    }

    if GetWindowTextLengthW(hwnd.0) < 1 {
      return false;
    }

    let ex_style = GetWindowLongPtrW(hwnd.0, GWL_EXSTYLE) as u32;
    if (ex_style & WS_EX_TOOLWINDOW) != 0 {
      return false;
    }

    let mut title_buffer: [u16; 512] = [0; 512];
    let title_length = GetWindowTextLengthW(hwnd.0) + 1;
    if title_length > 0 {
      GetWindowTextW(hwnd.0, title_buffer.as_mut_ptr(), title_length);
    }
    let title = OsString::from_wide(&title_buffer)
      .to_string_lossy()
      .into_owned();

    if title.is_empty() {
      return false;
    }

    true
  }
}

unsafe extern "system" fn win_hook_proc(
  _h_win_event_hook: HWINEVENTHOOK,
  event: DWORD,
  hwnd: HWND,
  _id_object: i32,
  _id_child: i32,
  _id_event_thread: DWORD,
  _dwms_event_time: DWORD,
) {
  CUTE_BORDERS
    .lock()
    .unwrap()
    .handle_win_hook_event(event, Hwnd(hwnd));
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
  if !should_add_border(&Hwnd(hwnd)) {
    return 1;
  }

  let visible_windows: &mut Vec<Hwnd> = &mut *(lparam as *mut Vec<Hwnd>);
  visible_windows.push(Hwnd(hwnd));

  1
}
