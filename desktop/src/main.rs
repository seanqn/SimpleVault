#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::Mutex;
#[cfg(not(target_os = "macos"))]
use tauri::Emitter;
use tauri::Manager;
#[cfg(target_os = "macos")]
mod menubar;
use vault_bridge::{ClientError, Commands, Request, Response};
struct Host {
    service: Box<dyn Commands>,
    session: &'static str,
    #[cfg(target_os = "macos")]
    registration: bool,
}
#[tauri::command]
fn vault_command(
    request: Request,
    state: tauri::State<'_, Mutex<Host>>,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> Result<Response, ClientError> {
    let mut host = state.lock().map_err(|_| ClientError {
        code: "unavailable",
        message: "The vault is unavailable. Restart SimpleVault.",
    })?;
    let response = host.service.dispatch(request)?;
    if let Some(snapshot) = &response.snapshot {
        #[cfg(target_os = "macos")]
        {
            let registration = snapshot.uid.is_none() && !snapshot.temporary;
            if registration != host.registration {
                host.registration = registration;
                menubar::configure(&window, registration).map_err(|_| ClientError {
                    code: "window_failed",
                    message: "The window could not be updated.",
                })?;
                menubar::show(&app, &window);
            }
        }
        if snapshot.session != host.session {
            host.session = snapshot.session;
            match snapshot.session {
                #[cfg(not(target_os = "macos"))]
                "unlocked" => {
                    let _ = window.set_size(tauri::LogicalSize::new(320.0, 400.0));
                    let _ = window.set_decorations(false);
                }
                #[cfg(not(target_os = "macos"))]
                "locked" => {
                    let _ = window.set_size(tauri::LogicalSize::new(700.0, 600.0));
                    let _ = window.set_decorations(true);
                }
                "closed" => {
                    drop(host);
                    app.exit(0);
                    return Ok(response);
                }
                _ => {}
            }
        }
    }
    Ok(response)
}
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(feature = "dev-vault")]
            #[allow(unused_mut)]
            let mut service: Box<dyn Commands> = Box::new(
                vault_bridge::development::TemporaryVault::new()
                    .map_err(|_| std::io::Error::other("could not initialize temporary vault"))?,
            );
            #[cfg(not(feature = "dev-vault"))]
            #[allow(unused_mut)]
            let mut service: Box<dyn Commands> =
                Box::new(vault_bridge::authentication::PasswordVault::new(
                    app.path().app_data_dir()?.join("vault.sqlite"),
                ));
            #[cfg(target_os = "macos")]
            let registration = {
                service
                    .dispatch(Request::Snapshot {})
                    .ok()
                    .and_then(|mut r| r.snapshot.take())
                    .as_ref()
                    .is_some_and(|s| s.uid.is_none() && !s.temporary)
            };
            app.manage(Mutex::new(Host {
                service,
                session: "locked",
                #[cfg(target_os = "macos")]
                registration,
            }));
            #[cfg(target_os = "macos")]
            menubar::install(app, registration)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::Focused(false) = event {
                if !window.is_decorated().unwrap_or(true) {
                    let _ = window.hide();
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                #[cfg(target_os = "macos")]
                let _ = window.hide();
                #[cfg(not(target_os = "macos"))]
                let _ = window.emit("vault-close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![vault_command])
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
}
