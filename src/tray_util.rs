use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItemBuilder};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::config::Config;
use crate::logger::Logger;
use crate::CUTE_BORDERS;

pub struct TrayUtil {}

impl TrayUtil {
  pub fn init() -> TrayIcon {
    let tray_menu_builder = Menu::with_items(&[
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
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    };

    let icon = match Icon::from_resource(1, Some((64, 64))) {
      Ok(icon) => icon,
      Err(err) => {
        Logger::log("[ERROR] Failed to create icon");
        Logger::log(&format!("[DEBUG] {:?}", err));
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    };

    let tray_icon_builder = TrayIconBuilder::new()
      .with_menu(Box::new(tray_menu))
      .with_menu_on_left_click(true)
      .with_icon(icon)
      .with_tooltip(format!("cute-borders v{}", env!("CARGO_PKG_VERSION")));

    let tray_icon = match tray_icon_builder.build() {
      Ok(tray_icon) => tray_icon,
      Err(err) => {
        Logger::log("[ERROR] Failed to build tray icon");
        Logger::log(&format!("[DEBUG] {:?}", err));
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    };

    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
      if event.id == MenuId::new("1") {
        Config::reload();
      } else if event.id == MenuId::new("2") {
        CUTE_BORDERS.lock().unwrap().drop_and_exit();
      }
    }));

    tray_icon
  }
}
