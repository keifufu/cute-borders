use winapi::{
  shared::{
    windef::HMONITOR,
    winerror::{E_FAIL, HRESULT, S_OK},
  },
  um::{
    shellscalingapi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    winuser::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST},
  },
};

use crate::Hwnd;
pub struct ScalingUtil {}
impl ScalingUtil {
  pub fn scaling_factor(window: &Hwnd) -> f32 {
    if let Ok(dpi) = Self::get_screen_dpi_for_window(window) {
      dpi as f32 / 96.0
    } else {
      1.0
    }
  }

  fn get_screen_dpi_for_monitor(target_monitor: HMONITOR) -> Result<u32, HRESULT> {
    if target_monitor.is_null() {
      return Err(E_FAIL);
    }

    let mut dpi: u32 = 0;

    let result = unsafe {
      GetDpiForMonitor(
        target_monitor,
        MDT_EFFECTIVE_DPI,
        &mut dpi,
        std::ptr::null_mut(),
      )
    };

    if result == S_OK {
      Ok(dpi)
    } else {
      Err(result)
    }
  }

  fn get_screen_dpi_for_window(hwnd: &Hwnd) -> Result<u32, HRESULT> {
    let target_monitor = unsafe { MonitorFromWindow(hwnd.0, MONITOR_DEFAULTTONEAREST) };
    Self::get_screen_dpi_for_monitor(target_monitor)
  }
}
