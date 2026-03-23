// ==================== System Tray ====================

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};

/// IDs of the tray menu items, used to match events in the polling loop.
pub struct TrayMenuIds {
    pub show: tray_icon::menu::MenuId,
    pub toggle_autostart: tray_icon::menu::MenuId,
    pub quit: tray_icon::menu::MenuId,
}

/// Owns the `TrayIcon` — dropping this removes the icon from the taskbar.
/// Keep it alive for the entire lifetime of the application.
pub struct Tray {
    pub ids: TrayMenuIds,
    _icon: TrayIcon, // must not be dropped
}

impl Tray {
    pub fn new(autostart_enabled: bool) -> Self {
        let show_item = MenuItem::new("Open Nexora Printer Manager", true, None);
        let autostart_item = MenuItem::new(
            if autostart_enabled {
                "✓ Launch at Startup"
            } else {
                "  Launch at Startup"
            },
            true,
            None,
        );
        let quit_item = MenuItem::new("Quit", true, None);

        let menu = Menu::new();
        menu.append(&show_item).unwrap();
        menu.append(&autostart_item).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&quit_item).unwrap();

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Nexora Printer Manager")
            .with_icon(load_icon())
            .build()
            .expect("Failed to build tray icon");

        Self {
            ids: TrayMenuIds {
                show: show_item.id().clone(),
                toggle_autostart: autostart_item.id().clone(),
                quit: quit_item.id().clone(),
            },
            _icon: icon,
        }
    }

    /// Returns the channel to poll for menu click events.
    pub fn event_receiver() -> &'static std::sync::mpsc::Receiver<MenuEvent> {
        MenuEvent::receiver()
    }
}

fn load_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).expect("Failed to load tray icon");
    let (w, h) = image::GenericImageView::dimensions(&img);
    let rgba = img.into_rgba8().into_raw();
    tray_icon::Icon::from_rgba(rgba, w, h).expect("Failed to create tray icon")
}
