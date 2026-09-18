mod commands;
mod providers;
mod runtime;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition, WindowEvent,
};

fn position_main_window(window: &tauri::WebviewWindow) {
    const MARGIN: i32 = 18;

    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let Ok(window_size) = window.outer_size() else {
        return;
    };
    let work_area = monitor.work_area();
    let x = work_area.position.x + work_area.size.width as i32 - window_size.width as i32 - MARGIN;
    let y = work_area.position.y + MARGIN;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn notify_running(app: &tauri::AppHandle) {
    use tauri_plugin_notification::{NotificationExt, PermissionState};

    let notification = app.notification();
    let state = notification
        .permission_state()
        .unwrap_or(PermissionState::Denied);
    let granted = match state {
        PermissionState::Granted => true,
        PermissionState::Prompt | PermissionState::PromptWithRationale => notification
            .request_permission()
            .map(|s| s == PermissionState::Granted)
            .unwrap_or(false),
        PermissionState::Denied => false,
    };

    if granted {
        let _ = notification
            .builder()
            .title("Quotify is running")
            .body("Look for the tray icon, or press Ctrl+Shift+U to open.")
            .show();
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::runtime_commands::list_runtimes,
            commands::provider_commands::list_providers,
            commands::usage_commands::fetch_provider_usage
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let open_item = MenuItem::with_id(app, "open", "Open Quotify", true, None::<&str>)?;
            let refresh_item =
                MenuItem::with_id(app, "refresh_all", "Refresh All", false, None::<&str>)?;
            let environments_item =
                MenuItem::with_id(app, "environments", "Environments", false, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings", false, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &open_item,
                    &refresh_item,
                    &environments_item,
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_item,
                ],
            )?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };

                let toggle_shortcut =
                    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyU);

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            if shortcut == &toggle_shortcut {
                                if let ShortcutState::Pressed = event.state() {
                                    toggle_main_window(app);
                                }
                            }
                        })
                        .build(),
                )?;

                app.global_shortcut().register(toggle_shortcut)?;
            }

            let runtime_manager = runtime::RuntimeManager::discover();
            for rt in runtime_manager.runtimes() {
                println!("INFO runtime {} detected", rt.name());
            }

            // Detected once at startup and cached, not re-scanned per
            // frontend request — each provider check on a WSL runtime
            // spawns a wsl.exe process, which is slow (WSL cold start) and
            // was previously done twice (once here, once when the UI called
            // list_providers).
            let installations = providers::ProviderRegistry::discover(&runtime_manager);
            for installation in &installations {
                println!(
                    "INFO {} found in {}",
                    installation.provider_name, installation.runtime_name
                );
            }

            app.manage(runtime_manager);
            app.manage(commands::provider_commands::ProviderState(installations));
            app.manage(providers::AdapterRegistry::with_defaults());

            if let Some(window) = app.get_webview_window("main") {
                position_main_window(&window);
            }

            notify_running(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
