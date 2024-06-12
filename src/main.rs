#![windows_subsystem = "windows"]
#![allow(unused_assignments)]

use config::Config;
use config::RuleMatch;
use logger::Logger;
use util::get_file_path;
use std::ffi::c_ulong;
use std::ffi::OsString;
use std::os::windows::prelude::OsStringExt;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuId;
use tray_icon::menu::MenuItemBuilder;
use tray_icon::Icon;
use tray_icon::TrayIconBuilder;
use util::hex_to_colorref;
use winapi::ctypes::c_int;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM};
use winapi::shared::windef::{HWINEVENTHOOK__, HWND};
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winuser::EnumWindows;
use winapi::um::winuser::GetClassNameW;
use winapi::um::winuser::GetWindowTextLengthW;
use winapi::um::winuser::GetWindowTextW;
use winapi::um::winuser::WS_EX_TOOLWINDOW;
use winapi::um::winuser::{
  DispatchMessageW, GetForegroundWindow, GetMessageW, IsWindowVisible, SetWinEventHook,
  TranslateMessage, UnhookWinEvent, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY,
  EVENT_SYSTEM_FOREGROUND, GWL_EXSTYLE, WINEVENT_OUTOFCONTEXT,
};

const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_COLOR_DEFAULT: u32 = 0xFFFFFFFF;

mod config;
mod logger;
mod util;

fn main() {
  unsafe {
    let create_hook = SetWinEventHook(
      EVENT_OBJECT_CREATE,
      EVENT_OBJECT_DESTROY,
      std::ptr::null_mut(),
      Some(create_window_event),
      0,
      0,
      WINEVENT_OUTOFCONTEXT,
    );

    let foreground_hook = SetWinEventHook(
      EVENT_SYSTEM_FOREGROUND,
      EVENT_SYSTEM_FOREGROUND,
      std::ptr::null_mut(),
      Some(foreground_event),
      0,
      0,
      WINEVENT_OUTOFCONTEXT,
    );

    if create_hook.is_null() || foreground_hook.is_null() {
      Logger::log("[ERROR] Failed to set up hooks");
      Logger::log(&format!("[DEBUG] {:?}", GetLastError()));
      std::process::exit(1);
    }

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
          .text("Exit")
          .enabled(true)
          .id(MenuId::new("2"))
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
  
      MenuEvent::set_event_handler(Some(|event: MenuEvent| {
        if event.id == MenuId::new("0") {
          let _ = open::that(get_file_path("config.yaml"));
        } else if event.id == MenuId::new("1") {
          Config::reload();
          apply_colors(false);
        } else if event.id == MenuId::new("2") {
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

    UnhookWinEvent(create_hook);
    UnhookWinEvent(foreground_hook);
    apply_colors(true);
  }
}

unsafe extern "system" fn create_window_event(
  _h_win_event_hook: *mut HWINEVENTHOOK__,
  event: DWORD,
  _hwnd: HWND,
  _id_object: i32,
  _id_child: i32,
  _id_event_thread: DWORD,
  _dwms_event_time: DWORD,
) {
  if event == EVENT_OBJECT_CREATE || event == EVENT_OBJECT_DESTROY {
    apply_colors(false);
  }
}

unsafe extern "system" fn foreground_event(
  _h_win_event_hook: *mut HWINEVENTHOOK__,
  _event: DWORD,
  _hwnd: HWND,
  _id_object: i32,
  _id_child: i32,
  _id_event_thread: DWORD,
  _dwms_event_time: DWORD,
) {
  apply_colors(false);
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
    let result = GetClassNameW(hwnd, class_buffer.as_mut_ptr(), class_buffer.len() as c_int);
    let mut class_name = String::new();
    if result > 0 {
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
  let mut color_active = DWMWA_COLOR_DEFAULT;
  let mut color_inactive = DWMWA_COLOR_DEFAULT;

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
