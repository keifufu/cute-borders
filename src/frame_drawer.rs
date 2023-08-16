use crate::logger::Logger;
use crate::Hwnd;
use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, num::Wrapping};
use winapi::shared::d3d9types::D3DCOLORVALUE;
use winapi::um::d2d1::{
  D2D1_BRUSH_PROPERTIES, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_MATRIX_3X2_F,
  D2D1_PRESENT_OPTIONS_NONE,
};
use winapi::{
  ctypes::c_void,
  shared::{
    basetsd::UINT_PTR,
    dxgiformat::DXGI_FORMAT_UNKNOWN,
    windef::{COLORREF, RECT},
    winerror::SUCCEEDED,
  },
  um::{
    d2d1::{
      D2D1CreateFactory, ID2D1Brush, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
      D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_COLOR_F, D2D1_FACTORY_TYPE_MULTI_THREADED,
      D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RECT_F, D2D1_RENDER_TARGET_PROPERTIES,
      D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE, D2D1_ROUNDED_RECT,
      D2D1_SIZE_U,
    },
    d2d1_1::ID2D1Factory1,
    dcommon::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT},
    dwmapi::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
    unknwnbase::IUnknown,
    wingdi::{GetBValue, GetGValue, GetRValue},
    winuser::{ShowWindow, SW_HIDE, SW_SHOW},
  },
  Interface,
};
use wio::com::ComPtr;

struct DrawableRect {
  rect: Option<D2D1_RECT_F>,
  rounded_rect: Option<D2D1_ROUNDED_RECT>,
  border_color: D2D1_COLOR_F,
  thickness: u32,
}

impl Default for DrawableRect {
  fn default() -> Self {
    Self {
      rect: None,
      rounded_rect: None,
      border_color: FrameDrawer::convert_color(0xFFFFFF),
      thickness: 0,
    }
  }
}

struct RenderTarget(ComPtr<ID2D1HwndRenderTarget>);
unsafe impl Send for RenderTarget {}

struct BorderBrush(ComPtr<ID2D1SolidColorBrush>);
unsafe impl Send for BorderBrush {}

pub struct FrameDrawer {
  window: Hwnd,
  render_target_size_hash: usize,
  render_target: Option<RenderTarget>,
  border_brush: Option<BorderBrush>,
  scene_rect: DrawableRect,
}

impl FrameDrawer {
  pub fn new(window: Hwnd) -> Option<Self> {
    let mut frame_drawer = Self {
      window,
      render_target_size_hash: 0,
      render_target: None,
      border_brush: None,
      scene_rect: DrawableRect::default(),
    };

    if frame_drawer.init() {
      return Some(frame_drawer);
    }

    None
  }

  fn init(&mut self) -> bool {
    let mut client_rect: RECT = RECT {
      bottom: 0,
      left: 0,
      right: 0,
      top: 0,
    };
    if !SUCCEEDED(unsafe {
      DwmGetWindowAttribute(
        self.window.0,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut client_rect as *mut _ as *mut c_void,
        std::mem::size_of::<RECT>() as u32,
      )
    }) {
      return false;
    }

    self.create_render_targets(client_rect)
  }

  fn create_render_targets(&mut self, client_rect: RECT) -> bool {
    let dpi = 96.0;
    let render_target_properties = D2D1_RENDER_TARGET_PROPERTIES {
      _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
      pixelFormat: D2D1_PIXEL_FORMAT {
        format: DXGI_FORMAT_UNKNOWN,
        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
      },
      dpiX: dpi,
      dpiY: dpi,
      usage: D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE,
      minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    };

    let render_target_size = D2D1_SIZE_U {
      width: (client_rect.right - client_rect.left) as u32,
      height: (client_rect.bottom - client_rect.top) as u32,
    };
    let rect_hash = d2d_rect_uhash(render_target_size);
    if self.render_target.is_some() && rect_hash == self.render_target_size_hash {
      return true;
    }

    self.render_target = None;

    let hwnd_render_target_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
      hwnd: self.window.0,
      pixelSize: render_target_size,
      presentOptions: D2D1_PRESENT_OPTIONS_NONE,
    };

    let mut res = std::ptr::null_mut();
    let hr = unsafe {
      (*self.get_d2d_factory()).CreateHwndRenderTarget(
        &render_target_properties,
        &hwnd_render_target_properties,
        &mut res,
      )
    };
    self.render_target = Some(RenderTarget(unsafe { ComPtr::from_raw(res) }));
    if !SUCCEEDED(hr) || self.render_target.is_none() {
      return false;
    }

    if let Some(render_target) = self.render_target.as_ref() {
      unsafe {
        render_target
          .0
          .SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
      }

      self.render_target_size_hash = rect_hash;

      true
    } else {
      false
    }
  }

  pub fn hide(&self) {
    unsafe { ShowWindow(self.window.0, SW_HIDE) };
  }

  pub fn show(&mut self) {
    unsafe { ShowWindow(self.window.0, SW_SHOW) };
    self.render();
  }

  pub fn set_border_rect(
    &mut self,
    window_rect: RECT,
    color: COLORREF,
    thickness: u32,
    radius: f32,
  ) {
    let mut new_scene_rect = DrawableRect {
      border_color: Self::convert_color(color),
      thickness,
      ..Default::default()
    };

    if radius != 0.0 {
      new_scene_rect.rounded_rect = Some(self.convert_rounded_rect(window_rect, thickness, radius));
    } else {
      new_scene_rect.rect = Some(self.convert_rect(window_rect, thickness));
    }

    let color_updated =
      are_d2dcolorvalue_equal(&self.scene_rect.border_color, &new_scene_rect.border_color);
    let thickness_updated = self.scene_rect.thickness != new_scene_rect.thickness;
    let corners_updated = self.scene_rect.rect.is_some() != new_scene_rect.rect.is_some()
      || self.scene_rect.rounded_rect.is_some() != new_scene_rect.rounded_rect.is_some();
    let needs_redraw = color_updated || thickness_updated || corners_updated;

    let mut client_rect = RECT {
      bottom: 0,
      left: 0,
      right: 0,
      top: 0,
    };
    if !SUCCEEDED(unsafe {
      DwmGetWindowAttribute(
        self.window.0,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut client_rect as *mut _ as *mut c_void,
        std::mem::size_of::<RECT>() as u32,
      )
    }) {
      return;
    }

    self.scene_rect = new_scene_rect;

    let render_target_size = D2D1_SIZE_U {
      width: (client_rect.right - client_rect.left) as u32,
      height: (client_rect.bottom - client_rect.top) as u32,
    };

    let rect_hash = d2d_rect_uhash(render_target_size);

    let at_the_desired_size =
      (rect_hash == self.render_target_size_hash) && self.render_target.is_some();
    if !at_the_desired_size {
      let resize_ok = self.render_target.is_some()
        && SUCCEEDED(unsafe {
          self
            .render_target
            .as_ref()
            .unwrap()
            .0
            .Resize(&render_target_size)
        });
      if !resize_ok {
        if !self.create_render_targets(client_rect) {
          Logger::log("[ERROR] Failed to create render targets");
        }
      } else {
        self.render_target_size_hash = rect_hash;
      }
    }

    if color_updated {
      self.border_brush = None;
      if let Some(render_target) = self.render_target.as_ref() {
        let mut res = std::ptr::null_mut();
        unsafe {
          render_target.0.CreateSolidColorBrush(
            &self.scene_rect.border_color,
            &D2D1_BRUSH_PROPERTIES {
              opacity: 1.0,
              transform: D2D1_MATRIX_3X2_F {
                matrix: [[1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
              },
            },
            &mut res,
          );
        }
        self.border_brush = Some(BorderBrush(unsafe { ComPtr::from_raw(res) }));
      }
    }

    if !at_the_desired_size || needs_redraw {
      self.render();
    }
  }

  fn get_d2d_factory(&self) -> *mut ID2D1Factory {
    static mut ID2D1_FACTORY: UINT_PTR = 0;

    unsafe {
      if ID2D1_FACTORY == 0 {
        let mut res: *mut IUnknown = std::ptr::null_mut();
        D2D1CreateFactory(
          D2D1_FACTORY_TYPE_MULTI_THREADED,
          &ID2D1Factory1::uuidof(),
          std::ptr::null_mut(),
          &mut res as *mut _ as *mut *mut c_void,
        );
        ID2D1_FACTORY = res as UINT_PTR;
      }

      ID2D1_FACTORY as *mut ID2D1Factory
    }
  }

  fn convert_color(color: COLORREF) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
      r: GetRValue(color) as f32 / 255.0,
      g: GetGValue(color) as f32 / 255.0,
      b: GetBValue(color) as f32 / 255.0,
      a: 1.0,
    }
  }

  fn convert_rect(&self, rect: RECT, thickness: u32) -> D2D1_RECT_F {
    let half_thickness = thickness as f32 / 2.0;

    // 1 is needed to eliminate the gap between border and window
    D2D1_RECT_F {
      left: rect.left as f32 + half_thickness + 1.0,
      top: rect.top as f32 + half_thickness + 1.0,
      right: rect.right as f32 - half_thickness - 1.0,
      bottom: rect.bottom as f32 - half_thickness - 1.0,
    }
  }

  fn convert_rounded_rect(&self, rect: RECT, thickness: u32, radius: f32) -> D2D1_ROUNDED_RECT {
    let rect = self.convert_rect(rect, thickness);
    D2D1_ROUNDED_RECT {
      rect,
      radiusX: radius,
      radiusY: radius,
    }
  }

  fn render(&mut self) {
    if let (Some(render_target), Some(border_brush)) =
      (&mut self.render_target, &mut self.border_brush)
    {
      unsafe {
        render_target.0.BeginDraw();
        render_target.0.Clear(&D2D1_COLOR_F {
          r: 0.0,
          g: 0.0,
          b: 0.0,
          a: 0.0,
        });

        if let Some(rounded_rect) = &self.scene_rect.rounded_rect {
          render_target.0.DrawRoundedRectangle(
            rounded_rect,
            border_brush.0.as_raw() as *mut ID2D1Brush,
            self.scene_rect.thickness as f32,
            std::ptr::null_mut(),
          );
        } else if let Some(rect) = &self.scene_rect.rect {
          render_target.0.DrawRectangle(
            rect,
            border_brush.0.as_raw() as *mut ID2D1Brush,
            self.scene_rect.thickness as f32,
            std::ptr::null_mut(),
          );
        }

        render_target
          .0
          .EndDraw(std::ptr::null_mut(), std::ptr::null_mut());
      }
    }
  }
}

fn d2d_rect_uhash(rect: D2D1_SIZE_U) -> usize {
  let pod_repr: Wrapping<usize> = unsafe { Wrapping(std::mem::transmute_copy(&rect)) };

  let mut hasher = DefaultHasher::new();
  pod_repr.hash(&mut hasher);
  hasher.finish() as usize
}

fn are_d2dcolorvalue_equal(a: &D3DCOLORVALUE, b: &D3DCOLORVALUE) -> bool {
  a.r == b.r && a.g == b.g && a.b == b.b && a.a == b.a
}
