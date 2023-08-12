use std::{
  fs::{self, File, OpenOptions},
  io::Write,
  path::Path,
};

use crate::{logger::Logger, DWMWA_COLOR_DEFAULT};

pub fn get_file(filename: &str, default_content: &str) -> std::fs::File {
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
    match fs::create_dir(&dirpath) {
      Ok(_) => {}
      Err(err) => {
        Logger::log(&format!("[ERROR] Failed to create directory: {}", &dirpath));
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
    }
  }

  if !Path::new(&filepath).exists() {
    let mut file = match File::create(&filepath) {
      Ok(file) => file,
      Err(err) => {
        Logger::log(&format!("[ERROR] Failed to create file: {}", &filepath));
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
    };

    match file.write_all(default_content.as_bytes()) {
      Ok(_) => {}
      Err(err) => {
        Logger::log(&format!("[ERROR] Failed to write to file: {}", &filepath));
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
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
