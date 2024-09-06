use check_elevation::is_elevated;
use planif::enums::TaskCreationFlags;
use planif::schedule::TaskScheduler;
use planif::schedule_builder::Action;
use planif::schedule_builder::ScheduleBuilder;
use planif::settings::Duration;
use planif::settings::LogonType;
use planif::settings::PrincipalSettings;
use planif::settings::RunLevel;
use planif::settings::Settings;
use std::{
  env,
  fs::{self, File, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
};
use winapi::shared::minwindef::BOOL;
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::dwmapi::DwmGetColorizationColor;
use winapi::um::winnt::{KEY_READ, KEY_WRITE};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::rainbow::Rainbow;
use crate::{logger::Logger, COLOR_INVALID, DWMWA_COLOR_DEFAULT, DWMWA_COLOR_NONE};

pub fn get_file_path(filename: &str) -> String {
  let user_profile_path = match std::env::var("USERPROFILE") {
    Ok(user_profile_path) => user_profile_path,
    Err(err) => {
      Logger::log("[ERROR] Failed to find USERPROFILE environment variable");
      Logger::log(&format!("[DEBUG] {:?}", err));
      std::process::exit(1);
    }
  };
  let dirpath = format!("{}\\.cuteborders", user_profile_path);
  let filepath = format!("{}\\{}", dirpath, filename);

  if !Path::new(&dirpath).exists() {
    if let Err(err) = fs::create_dir(&dirpath) {
      Logger::log(&format!("[ERROR] Failed to create directory: {}", &dirpath));
      Logger::log(&format!("[DEBUG] {:?}", err));
      std::process::exit(1);
    }
  }
  return filepath;
}

pub fn get_file(filename: &str, default_content: &str) -> std::fs::File {
  let filepath = get_file_path(filename);

  if !Path::new(&filepath).exists() {
    let mut file = match File::create(&filepath) {
      Ok(file) => file,
      Err(err) => {
        Logger::log(&format!("[ERROR] Failed to create file: {}", &filepath));
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
    };

    if let Err(err) = file.write_all(default_content.as_bytes()) {
      Logger::log(&format!("[ERROR] Failed to write to file: {}", &filepath));
      Logger::log(&format!("[DEBUG] {:?}", err));
      std::process::exit(1);
    }
  }

  let file = match OpenOptions::new()
    .read(true)
    .write(true)
    .append(true)
    .open(&filepath)
  {
    Ok(file) => file,
    Err(err) => {
      Logger::log(&format!("[ERROR] Failed to open file: {}", &filepath));
      Logger::log(&format!("[DEBUG] {:?}", err));
      std::process::exit(1);
    }
  };

  file
}

pub fn hex_to_colorref(hex: &str) -> u32 {
  if hex == "transparent" {
    return DWMWA_COLOR_NONE;
  }

  if hex == "accent" {
    let mut colorization: u32 = 0;
    let mut opaqueblend: BOOL = 0;
    // should not call this every single fucking time but whatever
    let result = unsafe { DwmGetColorizationColor(&mut colorization, &mut opaqueblend) };
    if SUCCEEDED(result) {
      let red = (colorization & 0x00FF0000) >> 16;
      let green = (colorization & 0x0000FF00) >> 8;
      let blue = (colorization & 0x000000FF) >> 0;
      let bbggrr = (blue << 16) | (green << 8) | red;
      return bbggrr;
    } else {
      Logger::log(&format!(
        "[ERROR] Failed to retrieve accent color: 0x{:08X})",
        result
      ));
      // Not returning COLOR_INVALID here since the config is not invalid,
      // instead returning DWMWA_COLOR_DEFAULT to let the system handle it.
      return DWMWA_COLOR_DEFAULT;
    }
  }

  if hex == "rainbow" {
    return Rainbow::get_color();
  }

  if hex.len() != 7 || !hex.starts_with('#') {
    Logger::log(&format!("[ERROR] Invalid hex: {}", hex));
    return COLOR_INVALID;
  }

  let r = u8::from_str_radix(&hex[1..3], 16);
  let g = u8::from_str_radix(&hex[3..5], 16);
  let b = u8::from_str_radix(&hex[5..7], 16);

  match (r, g, b) {
    (Ok(r), Ok(g), Ok(b)) => (b as u32) << 16 | (g as u32) << 8 | r as u32,
    _ => {
      Logger::log(&format!("[ERROR] Invalid hex: {}", hex));
      COLOR_INVALID
    }
  }
}

fn clean_old_registry_key() {
  let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
    "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
    KEY_READ | KEY_WRITE,
  ) {
    Ok(key) => Some(key),
    Err(_) => None,
  };

  if let Some(key) = key {
    let _ = key.delete_value("cute-borders");
  }
}

pub fn get_exe_path() -> PathBuf {
  let exe_path: PathBuf = match env::current_exe() {
    Ok(path) => path,
    Err(err) => {
      Logger::log("[ERROR] Failed to find own executable path");
      Logger::log(&format!("[DEBUG] {:?}", err));
      std::process::exit(1);
    }
  };

  let user_profile_path = match std::env::var("USERPROFILE") {
    Ok(user_profile_path) => user_profile_path,
    Err(err) => {
      Logger::log("[ERROR] Failed to find USERPROFILE environment variable");
      Logger::log(&format!("[DEBUG] {:?}", err));
      std::process::exit(1);
    }
  };
  let new_exe_path = PathBuf::from(format!(
    "{}\\.cuteborders\\cute-borders.exe",
    user_profile_path,
  ));

  if exe_path != new_exe_path {
    if Path::new(&new_exe_path).exists() {
      match fs::remove_file(&new_exe_path) {
        Ok(_) => {}
        Err(err) => {
          Logger::log(&format!(
            "[ERROR] Failed to delete file: {}",
            &new_exe_path.to_string_lossy()
          ));
          Logger::log(&format!("[DEBUG] {:?}", err));
          std::process::exit(1);
        }
      }
    }

    match fs::copy(&exe_path, &new_exe_path) {
      Ok(_) => {}
      Err(err) => {
        Logger::log(&format!(
          "[ERROR] Failed to copy file: {} to: {}",
          &exe_path.to_string_lossy(),
          &new_exe_path.to_string_lossy()
        ));
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
    }
  }

  return new_exe_path;
}

pub fn set_startup(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
  clean_old_registry_key();
  let exe_path = get_exe_path();
  let is_elevated = is_elevated().unwrap_or(false);

  if !is_elevated {
    return Ok(());
  }

  let ts = TaskScheduler::new()?;
  let com = ts.get_com();
  let sb = ScheduleBuilder::new(&com).unwrap();

  let mut settings = Settings::new();
  settings.stop_if_going_on_batteries = Some(false);
  settings.disallow_start_if_on_batteries = Some(false);
  settings.enabled = Some(true);

  let action = Action::new("cute-borders-action", &exe_path.to_string_lossy(), "", "");

  let delay = Duration {
    seconds: Some(5),
    // see https://github.com/mattrobineau/planif/commit/ac2e7f79ec8de8935c6292d64533a6c7ce37212e
    // github has 1.0.1 but crates.io doesnt
    hours: Some(0),
    ..Default::default()
  };

  sb.create_logon()
    .settings(settings)?
    .author("keifufu")?
    .description("cute-borders startup")?
    .principal(PrincipalSettings {
      display_name: "".to_string(),
      group_id: None,
      id: "".to_string(),
      logon_type: LogonType::Password,
      run_level: RunLevel::Highest,
      user_id: None,
    })?
    .trigger("cute-borders-trigger", enabled)?
    .delay(delay)?
    .action(action)?
    .build()?
    .register("cute-borders", TaskCreationFlags::CreateOrUpdate as i32)?;

  Ok(())
}
