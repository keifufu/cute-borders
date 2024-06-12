use std::{
  env,
  fs::{self, File, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
};

use winapi::um::winnt::{KEY_READ, KEY_WRITE};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::{logger::Logger, DWMWA_COLOR_DEFAULT, DWMWA_COLOR_NONE};

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

  if hex.len() != 7 || !hex.starts_with('#') {
    Logger::log(&format!("[ERROR] Invalid hex: {}", hex));
    return DWMWA_COLOR_DEFAULT;
  }

  let r = u8::from_str_radix(&hex[1..3], 16);
  let g = u8::from_str_radix(&hex[3..5], 16);
  let b = u8::from_str_radix(&hex[5..7], 16);

  match (r, g, b) {
    (Ok(r), Ok(g), Ok(b)) => (b as u32) << 16 | (g as u32) << 8 | r as u32,
    _ => {
      Logger::log(&format!("[ERROR] Invalid hex: {}", hex));
      DWMWA_COLOR_DEFAULT
    }
  }
}

fn get_registry_key() -> Option<RegKey> {
  match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
    "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
    KEY_READ | KEY_WRITE,
  ) {
    Ok(key) => Some(key),
    Err(err) => {
      Logger::log("[ERROR] Failed to open registry key");
      Logger::log(&format!("[DEBUG] {:?}", err));
      None
    }
  }
}

fn key_exists(app_name: &str) -> bool {
  match get_registry_key() {
    Some(key) => key.get_raw_value(app_name).is_ok(),
    None => false,
  }
}

pub fn enable_startup() {
  if key_exists("cute-borders") {
    return;
  }

  let exe_path = match env::current_exe() {
    Ok(path) => path,
    Err(err) => {
      Logger::log("[ERROR] Failed to find own executable path");
      Logger::log(&format!("[DEBUG] {:?}", err));
      return;
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
          return;
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
        return;
      }
    }
  }

  if let Some(key) = get_registry_key() {
    if let Err(err) = key.set_value("cute-borders", &new_exe_path.to_string_lossy().to_string()) {
      Logger::log("[ERROR] Failed to set registry key");
      Logger::log(&format!("[DEBUG] {:?}", err));
    }
  }
}

pub fn disable_startup() {
  if !key_exists("cute-borders") {
    return;
  }

  if let Some(key) = get_registry_key() {
    match key.delete_value("cute-borders") {
      Ok(_) => {}
      Err(err) => {
        Logger::log("[ERROR] Failed to delete registry key");
        Logger::log(&format!("[DEBUG] {:?}", err));
      }
    }
  }
}
