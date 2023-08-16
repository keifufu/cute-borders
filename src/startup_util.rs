use std::fs;
use std::path::{Path, PathBuf};

use winapi::um::winnt::{KEY_READ, KEY_WRITE};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use crate::logger::Logger;
use crate::CUTE_BORDERS;

pub struct StartupUtil {}

impl StartupUtil {
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
    match Self::get_registry_key() {
      Some(key) => key.get_raw_value(app_name).is_ok(),
      None => false,
    }
  }

  pub fn enable_startup() {
    if Self::key_exists("cute-borders") {
      return;
    }

    let exe_path = match std::env::current_exe() {
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
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
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

    if let Some(key) = Self::get_registry_key() {
      if let Err(err) = key.set_value("cute-borders", &new_exe_path.to_string_lossy().to_string()) {
        Logger::log("[ERROR] Failed to set registry key");
        Logger::log(&format!("[DEBUG] {:?}", err));
      }
    }
  }

  pub fn disable_startup() {
    if !Self::key_exists("cute-borders") {
      return;
    }

    if let Some(key) = Self::get_registry_key() {
      match key.delete_value("cute-borders") {
        Ok(_) => {}
        Err(err) => {
          Logger::log("[ERROR] Failed to delete registry key");
          Logger::log(&format!("[DEBUG] {:?}", err));
        }
      }
    }
  }
}
