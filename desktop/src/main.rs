#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use vault_bridge::{ClientError, Commands, Request, Response};
struct Host {
    service: Box<dyn Commands>,
    session: &'static str,
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
        if snapshot.session != host.session {
            host.session = snapshot.session;
            match snapshot.session {
                "unlocked" => {
                    let _ = window.set_size(tauri::LogicalSize::new(320.0, 400.0));
                    let _ = window.set_decorations(false);
                }
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
            let service: Box<dyn Commands> = Box::new(
                vault_bridge::development::TemporaryVault::new()
                    .map_err(|_| std::io::Error::other("could not initialize temporary vault"))?,
            );
            #[cfg(not(feature = "dev-vault"))]
            let service: Box<dyn Commands> = Box::new(vault_bridge::Bridge::new(
                app.path().app_data_dir()?.join("vault.sqlite"),
                vault_bridge::UnavailableKeys,
                false,
                None,
            ));
            app.manage(Mutex::new(Host {
                service,
                session: "locked",
            }));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("vault-close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![vault_command])
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
}
