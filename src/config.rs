use std::{io::Read, sync::Mutex};

use crate::{
  logger::Logger,
  util::{disable_startup, enable_startup, get_file},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG: &str = include_str!("data/config.yaml");

lazy_static! {
  static ref CONFIG: Mutex<Config> = Mutex::new(Config::new());
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
// Maybe support Process later.
// Getting the process name seems annoying.
pub enum RuleMatch {
  Global,
  Title,
  Class,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowRule {
  #[serde(rename = "match")]
  pub rule_match: RuleMatch,
  pub contains: Option<String>,
  pub active_border_color: String,
  pub inactive_border_color: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
  pub run_at_startup: bool,
  pub window_rules: Vec<WindowRule>,
}

impl Config {
  fn new() -> Self {
    let mut file = get_file("config.yaml", DEFAULT_CONFIG);
    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
      Ok(..) => {}
      Err(err) => {
        Logger::log("[ERROR] Failed to read config file");
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
    }
    let config: Config = match serde_yaml::from_str(contents.as_str()) {
      Ok(config) => config,
      Err(err) => {
        Logger::log("[ERROR] Failed to parse config file");
        Logger::log(&format!("[DEBUG] {:?}", err));
        std::process::exit(1);
      }
    };

    if config.run_at_startup {
      enable_startup();
    } else {
      disable_startup();
    }

    config
  }
  pub fn reload() {
    let mut config = CONFIG.lock().unwrap();
    *config = Self::new();
  }
  pub fn get() -> Self {
    CONFIG.lock().unwrap().clone()
  }
}
