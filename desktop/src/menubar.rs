//! macOS-only resident status item. Hiding a popup never locks or quits its session.
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

pub fn configure(window: &tauri::WebviewWindow, registration: bool) -> tauri::Result<()> {
    window.set_decorations(registration)?;
    window.set_size(if registration {
        tauri::LogicalSize::new(700., 600.)
    } else {
        tauri::LogicalSize::new(320., 400.)
    })?;
    window.set_skip_taskbar(true)?;
    if registration {
        window.center()?;
    }
    Ok(())
}
pub fn show(app: &tauri::AppHandle, window: &tauri::WebviewWindow) {
    let registration = window.is_decorated().unwrap_or(true);
    if !registration {
        if let Some(tray) = app.tray_by_id("simplevault") {
            if let Ok(Some(rect)) = tray.rect() {
                let scale = window.scale_factor().unwrap_or(1.);
                let position = rect.position.to_physical::<i32>(scale);
                let size = rect.size.to_physical::<u32>(scale);
                let width = (320. * scale) as i32;
                let mut x = position.x + size.width as i32 / 2 - width / 2;
                let y = position.y + size.height as i32 + (4. * scale) as i32;
                if let Ok(Some(monitor)) =
                    app.monitor_from_point(position.x as f64, position.y as f64)
                {
                    x = x.clamp(
                        monitor.position().x,
                        (monitor.position().x + monitor.size().width as i32 - width)
                            .max(monitor.position().x),
                    );
                }
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
    }
    let _ = window.show();
    let _ = window.set_focus();
}
pub fn install(app: &mut tauri::App, registration: bool) -> Result<(), Box<dyn std::error::Error>> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    let window = app
        .get_webview_window("main")
        .ok_or("missing main window")?;
    configure(&window, registration)?;
    let open = MenuItem::with_id(app, "open", "Open SimpleVault", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit SimpleVault…", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    // Monochrome lock silhouette; macOS applies the menu-bar appearance as a template.
    let mut pixels = vec![0u8; 18 * 18 * 4];
    for y in 2..16 {
        for x in 3..15 {
            if (y >= 8) || ((5..13).contains(&x) && (y == 2 || x == 5 || x == 12)) {
                pixels[(y * 18 + x) * 4 + 3] = 255;
            }
        }
    }
    TrayIconBuilder::with_id("simplevault")
        .icon(tauri::image::Image::new_owned(pixels, 18, 18))
        .icon_as_template(true)
        .tooltip("SimpleVault")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        show(tray.app_handle(), &window);
                    }
                }
            }
        })
        .on_menu_event(|app, event| {
            if let Some(window) = app.get_webview_window("main") {
                show(app, &window);
                if event.id().as_ref() == "quit" {
                    let _ = window.emit("vault-close-requested", ());
                }
            }
        })
        .build(app)?;
    if registration {
        show(app.handle(), &window);
    } else {
        window.hide()?;
    }
    Ok(())
}
