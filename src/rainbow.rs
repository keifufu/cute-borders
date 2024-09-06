use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

use crate::DWMWA_COLOR_DEFAULT;

lazy_static! {
  static ref RAINBOW: Mutex<Rainbow> = Mutex::new(Rainbow::new());
}

pub struct Rainbow {
  color: Arc<Mutex<u32>>,
  hue: Arc<Mutex<f32>>,
}

impl Rainbow {
  fn new() -> Self {
    let color = Arc::new(Mutex::new(DWMWA_COLOR_DEFAULT));
    let hue = Arc::new(Mutex::new(0.0));
    Rainbow { color, hue }
  }
  pub fn tick(speed: f32) {
    let rainbow = RAINBOW.lock().unwrap();
    let mut hue = rainbow.hue.lock().unwrap();
    let (r, g, b) = hsl_to_rgb(*hue, 1.0, 0.5);
    let color_value = 0x00u32 | ((b as u32) << 16) | ((g as u32) << 8) | ((r as u32) << 0);

    let mut color = rainbow.color.lock().unwrap();
    *color = color_value;
    *hue = (*hue + speed) % 360.0;
  }
  pub fn get_color() -> u32 {
    let rainbow = RAINBOW.lock().unwrap();
    let color = rainbow.color.lock().unwrap();
    *color
  }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
  let m = l - c / 2.0;

  let (r_prime, g_prime, b_prime) = if 0.0 <= h && h < 60.0 {
    (c, x, 0.0)
  } else if 60.0 <= h && h < 120.0 {
    (x, c, 0.0)
  } else if 120.0 <= h && h < 180.0 {
    (0.0, c, x)
  } else if 180.0 <= h && h < 240.0 {
    (0.0, x, c)
  } else if 240.0 <= h && h < 300.0 {
    (x, 0.0, c)
  } else {
    (c, 0.0, x)
  };

  let r = ((r_prime + m) * 255.0).round() as u8;
  let g = ((g_prime + m) * 255.0).round() as u8;
  let b = ((b_prime + m) * 255.0).round() as u8;

  (r, g, b)
}
