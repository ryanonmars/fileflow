mod commands;
mod config;
mod file_organizer;
mod file_watcher;

use commands::*;
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let show_item = MenuItem::with_id(app, "show", "Settings", true, None::<&str>)?;
            let update_item = MenuItem::with_id(app, "update", "Update", true, None::<&str>)?;
            let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &update_item, &about_item, &quit_item])?;

            let tray_icon = {
                let icon_bytes = include_bytes!("../icons/app-icon.png");
                let img = image::load_from_memory(icon_bytes)
                    .ok()
                    .and_then(|img| {
                        let rgba = img.to_rgba8();
                        let (width, height) = rgba.dimensions();
                        Some(tauri::image::Image::new_owned(rgba.into_raw(), width, height))
                    });
                img.or_else(|| app.default_window_icon().cloned())
            };

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon.unwrap().clone())
                .menu(&menu)
                .tooltip("FileFlow")
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            std::process::exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "update" => {
                            // Show the update window (separate window, like about)
                            if let Some(window) = app.get_webview_window("update") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "about" => {
                            // Show the about window (separate window, like file-organization)
                            if let Some(window) = app.get_webview_window("about") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window_clone.hide();
                    api.prevent_close();
                }
            });

            // Ensure modal windows exist - access them to trigger creation from config
            if let Some(modal_window) = app.get_webview_window("file-organization") {
                let modal_window_clone = modal_window.clone();
                modal_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        // Allow modal to close normally
                    }
                });
            }
            
            if let Some(about_window) = app.get_webview_window("about") {
                let about_window_clone = about_window.clone();
                about_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let _ = about_window_clone.hide();
                        api.prevent_close();
                    }
                });
            }
            
            if let Some(update_window) = app.get_webview_window("update") {
                let update_window_clone = update_window.clone();
                update_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let _ = update_window_clone.hide();
                        api.prevent_close();
                    }
                });
            }

            let event_tx = init_watcher().map_err(|e| {
                eprintln!("Failed to initialize watcher: {}", e);
                e
            })?;

            // Check for updates on startup if enabled
            #[cfg(not(debug_assertions))]
            {
                let app_handle = app.handle().clone();
                let config = crate::config::Config::load();
                if config.auto_check_for_updates && config.should_show_update_alert() {
                    tauri::async_runtime::spawn(async move {
                        // Small delay to let app fully start
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        
                        use tauri_plugin_updater::UpdaterExt;
                        if let Ok(updater_builder) = app_handle.updater_builder().build() {
                            if let Ok(Some(update)) = updater_builder.check().await {
                                // Check again if we should show alert (might have changed)
                                let config = crate::config::Config::load();
                                if !config.should_show_update_alert() {
                                    return;
                                }
                                
                                use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                                let dialog = app_handle.dialog();
                                let message = format!(
                                    "Update available: Version {}\n\nGo to the Update menu to install it.\n\n(You can ignore this alert for 7 days from the Update menu.)",
                                    update.version
                                );
                                dialog.message(&message)
                                    .kind(MessageDialogKind::Info)
                                    .title("Update Available")
                                    .show(|_| {});
                            }
                        }
                    });
                }
            }

            // Start periodic update checking if enabled
            #[cfg(not(debug_assertions))]
            {
                let app_handle = app.handle().clone();
                let config = crate::config::Config::load();
                if config.auto_check_for_updates {
                    tauri::async_runtime::spawn(async move {
                        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Check every hour
                        loop {
                            interval.tick().await;
                            let config = crate::config::Config::load();
                            if !config.auto_check_for_updates {
                                break;
                            }
                            
                            use tauri_plugin_updater::UpdaterExt;
                            if let Ok(updater_builder) = app_handle.updater_builder().build() {
                                if let Ok(Some(update)) = updater_builder.check().await {
                                    // Emit event to show update available
                                    let _ = app_handle.emit("update-available", serde_json::json!({
                                        "version": update.version.to_string()
                                    }));
                                    
                                    // Check if we should show alert
                                    let config = crate::config::Config::load();
                                    if config.should_show_update_alert() {
                                        // Show dialog alert
                                        use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                                        let dialog = app_handle.dialog();
                                        let message = format!(
                                            "Update available: Version {}\n\nGo to the Update menu to install it.\n\n(You can ignore this alert for 7 days from the Update menu.)",
                                            update.version
                                        );
                                        dialog.message(&message)
                                            .kind(MessageDialogKind::Info)
                                            .title("Update Available")
                                            .show(|_| {});
                                    }
                                }
                            }
                        }
                    });
                }
            }

            let app_handle = app.handle().clone();
            let mut rx = event_tx.subscribe();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let handle = rt.handle().clone();
                handle.spawn(async move {
                    loop {
                        if let Ok(msg) = rx.recv().await {
                            if msg.starts_with("file_queued:") {
                                let parts: Vec<&str> = msg.splitn(3, '|').collect();
                                if parts.len() >= 3 {
                                    let file_path = parts[0].strip_prefix("file_queued:").unwrap_or("");
                                    let file_name = parts[1];
                                    let file_size: u64 = parts[2].parse().unwrap_or(0);
                                    
                                    let app_for_modal = app_handle.clone();
                                    let file_path_clone = file_path.to_string();
                                    let file_name_clone = file_name.to_string();
                                    
                                    // Show the modal window
                                    if let Err(e) = commands::show_file_organization_modal(
                                        app_for_modal,
                                        file_path_clone,
                                        file_name_clone,
                                        file_size,
                                    ) {
                                        eprintln!("Failed to show file organization modal: {}", e);
                                    }
                                }
                            }
                        }
                    }
                });
                // Keep runtime alive
                rt.block_on(std::future::pending::<()>());
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            start_watching,
            stop_watching,
            get_organization_mode,
            set_organization_mode,
            get_pending_files,
            process_pending_file,
            move_file_manual,
            show_file_notification,
            process_file_from_notification,
            open_settings_window,
            show_file_organization_modal,
            close_file_organization_modal,
            get_app_info,
            check_for_updates,
            install_update,
            suppress_update_alert_for_days
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

