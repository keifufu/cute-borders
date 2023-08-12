#![windows_subsystem = "windows"]

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::ffi::c_ulong;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::windows::prelude::OsStringExt;
use std::path::Path;
use std::sync::Mutex;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuId;
use tray_icon::menu::MenuItemBuilder;
use tray_icon::TrayIconBuilder;
use winapi::ctypes::c_int;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM};
use winapi::shared::windef::{HWINEVENTHOOK__, HWND};
use winapi::um::dwmapi::DwmSetWindowAttribute;
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
const DEFAULT_CONFIG: &str = include_str!("config.yaml");
lazy_static! {
  static ref CONFIG: Mutex<Config> = Mutex::new(Config::read());
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
// Maybe support Process later.
// Getting the process name seems annoying.
enum RuleMatch {
  Global,
  Title,
  Class,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowRule {
  #[serde(rename = "match")]
  rule_match: RuleMatch,
  contains: Option<String>,
  active_border_color: String,
  inactive_border_color: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
  run_at_startup: bool,
  window_rules: Vec<WindowRule>,
}

impl Config {
  fn read() -> Self {
    let user_profile =
      std::env::var("USERPROFILE").expect("USERPROFILE environment variable not found");
    let config_dir = format!("{}\\.cuteborders", user_profile);
    let config_file = format!("{}\\config.yaml", config_dir);

    if !Path::new(&config_dir).exists() {
      fs::create_dir(&config_dir).expect("Failed to create directory");
    }

    if !Path::new(&config_file).exists() {
      let mut file = fs::File::create(&config_file).expect("Failed to create config file");
      file
        .write_all(DEFAULT_CONFIG.as_bytes())
        .expect("Failed to write to config file");
    }

    let contents = fs::read_to_string(&config_file).expect("Failed to read config");

    let config: Config = serde_yaml::from_str(contents.as_str()).expect("Failed to parse config");
    config
  }
  fn reload() {
    let mut config = CONFIG.lock().unwrap();
    *config = Self::read();
  }
}

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
      panic!("Error setting up hooks");
    }

    let tray_menu = Menu::with_items(&[
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
    ])
    .unwrap();
    #[allow(unused_variables)]
    let tray_icon = TrayIconBuilder::new()
      .with_menu(Box::new(tray_menu))
      .with_menu_on_left_click(true)
      .build()
      .unwrap();

    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
      if event.id == MenuId::new("1") {
        Config::reload();
        apply_colors(false);
      } else if event.id == MenuId::new("2") {
        apply_colors(true);
        std::process::exit(0);
      }
    }));

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

fn get_colors_for_window(_hwnd: HWND, title: String, class: String, reset: bool) -> (u32, u32) {
  if reset {
    return (DWMWA_COLOR_DEFAULT, DWMWA_COLOR_DEFAULT);
  }

  let config = CONFIG.lock().unwrap();
  let mut color_active = DWMWA_COLOR_DEFAULT;
  let mut color_inactive = DWMWA_COLOR_DEFAULT;

  for rule in config.window_rules.iter() {
    match rule.rule_match {
      RuleMatch::Global => {
        color_active = hex_to_colorref(&rule.active_border_color).expect("Failed to convert hex");
        color_inactive =
          hex_to_colorref(&rule.inactive_border_color).expect("Failed to convert hex");
      }
      RuleMatch::Title => {
        if let Some(contains_str) = &rule.contains {
          if title.to_lowercase().contains(&contains_str.to_lowercase()) {
            color_active =
              hex_to_colorref(&rule.active_border_color).expect("Failed to convert hex");
            color_inactive =
              hex_to_colorref(&rule.inactive_border_color).expect("Failed to convert hex");
            break;
          }
        } else {
          panic!("Expected 'rule.contains' to be Some(value)");
        }
      }
      RuleMatch::Class => {
        if let Some(contains_str) = &rule.contains {
          if class.to_lowercase().contains(&contains_str.to_lowercase()) {
            color_active =
              hex_to_colorref(&rule.active_border_color).expect("Failed to convert hex");
            color_inactive =
              hex_to_colorref(&rule.inactive_border_color).expect("Failed to convert hex");
            break;
          }
        } else {
          panic!("Expected 'rule.contains' to be Some(value)");
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

fn hex_to_colorref(hex: &str) -> Option<u32> {
  if hex.len() != 7 || !hex.starts_with('#') {
    return None; // Invalid format
  }

  let r = u8::from_str_radix(&hex[1..3], 16);
  let g = u8::from_str_radix(&hex[3..5], 16);
  let b = u8::from_str_radix(&hex[5..7], 16);

  match (r, g, b) {
    (Ok(r), Ok(g), Ok(b)) => Some((b as u32) << 16 | (g as u32) << 8 | r as u32),
    _ => None, // Invalid component values
  }
}
