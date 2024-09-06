#![windows_subsystem = "windows"]
#![allow(unused_assignments)]

use check_elevation::is_elevated;
use config::Config;
use config::RuleMatch;
use logger::Logger;
use rainbow::Rainbow;
use std::ffi::c_ulong;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::prelude::OsStringExt;
use std::time::Duration;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuId;
use tray_icon::menu::MenuItemBuilder;
use tray_icon::Icon;
use tray_icon::TrayIconBuilder;
use util::get_exe_path;
use util::get_file_path;
use util::hex_to_colorref;
use util::set_startup;
use winapi::ctypes::c_int;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, LPARAM};
use winapi::shared::windef::HWND;
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::shellapi::ShellExecuteExW;
use winapi::um::shellapi::SEE_MASK_NOASYNC;
use winapi::um::shellapi::SEE_MASK_NOCLOSEPROCESS;
use winapi::um::shellapi::SHELLEXECUTEINFOW;
use winapi::um::winuser::EnumWindows;
use winapi::um::winuser::GetClassNameW;
use winapi::um::winuser::GetWindowTextLengthW;
use winapi::um::winuser::GetWindowTextW;
use winapi::um::winuser::WS_EX_TOOLWINDOW;
use winapi::um::winuser::{
  DispatchMessageW, GetForegroundWindow, GetMessageW, IsWindowVisible, TranslateMessage,
  GWL_EXSTYLE,
};

const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_COLOR_DEFAULT: u32 = 0xFFFFFFFF;
const DWMWA_COLOR_NONE: u32 = 0xFFFFFFFE;
const COLOR_INVALID: u32 = 0x000000FF;

mod config;
mod logger;
mod rainbow;
mod util;

fn main() {
  if let Err(err) = set_startup(true) {
    Logger::log("[ERROR] Failed to create or update startup task");
    Logger::log(&format!("[DEBUG] {:?}", err));
  }

  // I will just fucking update everything every 100ms
  // I might want to do this properly buuuuut I dont even use this myself.
  std::thread::spawn(|| loop {
    Rainbow::tick(Config::get().rainbow_speed.unwrap_or(1.0));
    apply_colors(false);
    std::thread::sleep(Duration::from_millis(100));
  });

  let is_elevated = is_elevated().unwrap_or(false);
  unsafe {
    #[allow(unused_variables)]
    let tray_icon; // needs to be in the main scope
    if !Config::get().hide_tray_icon.unwrap_or(false) {
      let tray_menu_builder = Menu::with_items(&[
        &MenuItemBuilder::new()
          .text("Open config")
          .enabled(true)
          .id(MenuId::new("0"))
          .build(),
        &MenuItemBuilder::new()
          .text("Reload config")
          .enabled(true)
          .id(MenuId::new("1"))
          .build(),
        &MenuItemBuilder::new()
          .text(if is_elevated { "Uninstall" } else { "Install" })
          .enabled(true)
          .id(MenuId::new("2"))
          .build(),
        &MenuItemBuilder::new()
          .text("Exit")
          .enabled(true)
          .id(MenuId::new("3"))
          .build(),
      ]);

      let tray_menu = match tray_menu_builder {
        Ok(tray_menu) => tray_menu,
        Err(err) => {
          Logger::log("[ERROR] Failed to build tray icon");
          Logger::log(&format!("[DEBUG] {:?}", err));
          std::process::exit(1);
        }
      };

      let icon = match Icon::from_resource(1, Some((64, 64))) {
        Ok(icon) => icon,
        Err(err) => {
          Logger::log("[ERROR] Failed to create icon");
          Logger::log(&format!("[DEBUG] {:?}", err));
          std::process::exit(1);
        }
      };

      let tray_icon_builder = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_menu_on_left_click(true)
        .with_icon(icon)
        .with_tooltip(format!("cute-borders v{}", env!("CARGO_PKG_VERSION")));

      tray_icon = match tray_icon_builder.build() {
        Ok(tray_icon) => tray_icon,
        Err(err) => {
          Logger::log("[ERROR] Failed to build tray icon");
          Logger::log(&format!("[DEBUG] {:?}", err));
          std::process::exit(1);
        }
      };

      MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == MenuId::new("0") {
          let _ = open::that(get_file_path("config.yaml"));
        } else if event.id == MenuId::new("1") {
          Config::reload();
          apply_colors(false);
        } else if event.id == MenuId::new("2") {
          if is_elevated {
            if let Err(err) = set_startup(false) {
              Logger::log("[ERROR] Failed to create or update startup task");
              Logger::log(&format!("[DEBUG] {:?}", err));
            }
            apply_colors(true);
            std::process::exit(0);
          } else {
            let lp_verb: Vec<u16> = OsStr::new("runas")
              .encode_wide()
              .chain(std::iter::once(0))
              .collect();
            let d = get_exe_path();
            let v = d.to_str().unwrap_or_default();
            let lp_file: Vec<u16> = OsStr::new(&v)
              .encode_wide()
              .chain(std::iter::once(0))
              .collect();
            let lp_par: Vec<u16> = OsStr::new("")
              .encode_wide()
              .chain(std::iter::once(0))
              .collect();

            let mut sei = SHELLEXECUTEINFOW {
              cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
              fMask: SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS,
              lpVerb: lp_verb.as_ptr(),
              lpFile: lp_file.as_ptr(),
              lpParameters: lp_par.as_ptr(),
              nShow: 1,
              dwHotKey: 0,
              hInstApp: std::ptr::null_mut(),
              hMonitor: std::ptr::null_mut(),
              hProcess: std::ptr::null_mut(),
              hkeyClass: std::ptr::null_mut(),
              hwnd: std::ptr::null_mut(),
              lpClass: std::ptr::null_mut(),
              lpDirectory: std::ptr::null_mut(),
              lpIDList: std::ptr::null_mut(),
            };

            ShellExecuteExW(&mut sei);
            apply_colors(true);
            std::process::exit(0);
          }
        } else if event.id == MenuId::new("3") {
          apply_colors(true);
          std::process::exit(0);
        }
      }));
    }

    let mut msg = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
      TranslateMessage(&msg);
      DispatchMessageW(&msg);
    }

    apply_colors(true);
  }
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
  if IsWindowVisible(hwnd) != 0 {
    let mut title_buffer: [u16; 512] = [0; 512];
    let text_length = GetWindowTextLengthW(hwnd) + 1;
    if text_length > 0 {
      GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), text_length as c_int);
    }
    let title = OsString::from_wide(&title_buffer)
      .to_string_lossy()
      .into_owned();

    let ex_style = winapi::um::winuser::GetWindowLongW(hwnd, GWL_EXSTYLE) as c_int;

    let mut class_buffer: [u16; 256] = [0; 256];
    let class_result = GetClassNameW(hwnd, class_buffer.as_mut_ptr(), class_buffer.len() as c_int);
    let mut class_name = String::new();
    if class_result > 0 {
      class_name = OsString::from_wide(&class_buffer)
        .to_string_lossy()
        .into_owned();
    }

    // Exclude certain window styles like WS_EX_TOOLWINDOW
    if ex_style & (WS_EX_TOOLWINDOW as i32) == 0 {
      let visible_windows: &mut Vec<(HWND, String, String)> =
        &mut *(lparam as *mut Vec<(HWND, String, String)>);
      visible_windows.push((hwnd, title, class_name));
    }
  }

  1
}

fn get_colors_for_window(_hwnd: HWND, title: String, class: String, reset: bool) -> (u32, u32) {
  if reset {
    return (DWMWA_COLOR_DEFAULT, DWMWA_COLOR_DEFAULT);
  }

  let config = Config::get();
  let mut color_active = COLOR_INVALID;
  let mut color_inactive = COLOR_INVALID;

  for rule in config.window_rules.iter() {
    match rule.rule_match {
      RuleMatch::Global => {
        color_active = hex_to_colorref(&rule.active_border_color);
        color_inactive = hex_to_colorref(&rule.inactive_border_color);
      }
      RuleMatch::Title => {
        if let Some(contains_str) = &rule.contains {
          if title.to_lowercase().contains(&contains_str.to_lowercase()) {
            color_active = hex_to_colorref(&rule.active_border_color);
            color_inactive = hex_to_colorref(&rule.inactive_border_color);
            break;
          }
        } else {
          Logger::log("Expected `contains` on `Match=\"Title\"`");
        }
      }
      RuleMatch::Class => {
        if let Some(contains_str) = &rule.contains {
          if class.to_lowercase().contains(&contains_str.to_lowercase()) {
            color_active = hex_to_colorref(&rule.active_border_color);
            color_inactive = hex_to_colorref(&rule.inactive_border_color);
            break;
          }
        } else {
          Logger::log("Expected `contains` on `Match=\"Class\"`");
        }
      }
    }
  }

  (color_active, color_inactive)
}

fn apply_colors(reset: bool) {
  let mut visible_windows: Vec<(HWND, String, String)> = Vec::new();
  unsafe {
    EnumWindows(
      Some(enum_windows_callback),
      &mut visible_windows as *mut _ as LPARAM,
    );
  }

  for (hwnd, title, class) in visible_windows {
    let (color_active, color_inactive) = get_colors_for_window(hwnd, title, class, reset);
    unsafe {
      let active = GetForegroundWindow();

      if active == hwnd {
        DwmSetWindowAttribute(
          hwnd,
          DWMWA_BORDER_COLOR,
          &color_active as *const _ as *const c_void,
          std::mem::size_of::<c_ulong>() as u32,
        );
      } else {
        DwmSetWindowAttribute(
          hwnd,
          DWMWA_BORDER_COLOR,
          &color_inactive as *const _ as *const c_void,
          std::mem::size_of::<c_ulong>() as u32,
        );
      }
    }
  }
}
