use crate::logger::Logger;
use crate::CUTE_BORDERS;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub struct FileUtil {}
impl FileUtil {
  pub fn get_file(filename: &str, default_content: &str) -> std::fs::File {
    let user_profile_path = match std::env::var("USERPROFILE") {
      Ok(user_profile_path) => user_profile_path,
      Err(err) => {
        Logger::log("[ERROR] Failed to find USERPROFILE environment variable");
        Logger::log(&format!("[DEBUG] {:?}", err));
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    };
    let dirpath = format!("{}\\.cuteborders", user_profile_path);
    let filepath = format!("{}\\{}", dirpath, filename);

    if !Path::new(&dirpath).exists() {
      if let Err(err) = std::fs::create_dir(&dirpath) {
        Logger::log(&format!("[ERROR] Failed to create directory: {}", &dirpath));
        Logger::log(&format!("[DEBUG] {:?}", err));
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    }

    if !Path::new(&filepath).exists() {
      let mut file = match File::create(&filepath) {
        Ok(file) => file,
        Err(err) => {
          Logger::log(&format!("[ERROR] Failed to create file: {}", &filepath));
          Logger::log(&format!("[DEBUG] {:?}", err));
          CUTE_BORDERS.lock().unwrap().drop_and_exit();
        }
      };

      if let Err(err) = file.write_all(default_content.as_bytes()) {
        Logger::log(&format!("[ERROR] Failed to write to file: {}", &filepath));
        Logger::log(&format!("[DEBUG] {:?}", err));
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
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
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    };

    file
  }
}
