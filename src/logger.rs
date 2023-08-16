use lazy_static::lazy_static;
use std::{io::Write, sync::Mutex};

use crate::file_util::FileUtil;

lazy_static! {
  static ref LOGGER: Mutex<Logger> = Mutex::new(Logger::new().unwrap());
}

pub struct Logger {
  file: std::fs::File,
  last_message: Option<String>,
}

impl Logger {
  fn new() -> Result<Self, std::io::Error> {
    let file = FileUtil::get_file("log.txt", "");
    Ok(Logger {
      file,
      last_message: None,
    })
  }

  // Logs to console in debug mode, to file in release mode.
  pub fn log(message: &str) {
    #[cfg(debug_assertions)]
    {
      println!("{}", message);
    }

    let mut logger = LOGGER.lock().unwrap();

    if let Some(ref last_message) = logger.last_message {
      if last_message == message {
        return; // Don't log the same message again (can't be bothered to do it properly :3)
      }
    }

    let formatted_message = format!("{}\n", message);
    logger
      .file
      .write_all(formatted_message.as_bytes())
      .expect("Failed to write to log");
    logger.file.flush().expect("Failed to flush log");

    logger.last_message = Some(message.to_string());
  }
}
