use winapi::shared::minwindef::INT;
use winapi::shared::windef::HWND;
use winapi::shared::winerror::{E_INVALIDARG, S_OK};
use winapi::um::dwmapi::DwmGetWindowAttribute;

const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWCP_DEFAULT: INT = 0;
// const DWMWCP_DONOTROUND: INT = 1;
const DWMWCP_ROUND: INT = 2;
const DWMWCP_ROUNDSMALL: INT = 3;

pub struct WindowCornerUtil;

impl WindowCornerUtil {
  pub fn corner_preference(window: HWND) -> INT {
    let mut corner_preference: INT = -1;
    let res = unsafe {
      DwmGetWindowAttribute(
        window,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &mut corner_preference as *mut INT as *mut _,
        std::mem::size_of::<INT>() as u32,
      )
    };

    if res != S_OK && res != E_INVALIDARG {
      // Don't even log errors, on win10 this will always fail after all.
      // Logger::log("[ERROR] Failed to get corner preference");
      // Logger::log(&format!("[DEBUG] {:?}", res));
    }

    corner_preference
  }

  pub fn corners_radius(window: HWND) -> i32 {
    let corner_preference = Self::corner_preference(window);

    match corner_preference {
      DWMWCP_ROUND => 8,
      DWMWCP_ROUNDSMALL => 4,
      DWMWCP_DEFAULT => 8,
      _ => 0,
    }
  }
}
