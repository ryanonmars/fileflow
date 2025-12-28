mod commands;
mod config;
mod file_organizer;
mod file_watcher;

use commands::*;
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
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

            let show_item = MenuItem::with_id(app, "show", "Show Settings", true, None::<&str>)?;
            let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &about_item, &quit_item])?;

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
                        "about" => {
                            use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                            use tauri_plugin_updater::UpdaterExt;
                            
                            let version = app.package_info().version.to_string();
                            let product_name = app.package_info().name.clone();
                            let app_handle = app.clone();
                            
                            // Check for updates asynchronously
                            tauri::async_runtime::spawn(async move {
                                let dialog = app_handle.dialog();
                                let updater_builder = app_handle.updater_builder();
                                
                                match updater_builder.build() {
                                    Ok(updater) => {
                                        match updater.check().await {
                                            Ok(Some(update)) => {
                                                let message = format!(
                                                    "{}\nVersion {}\n\nUpdate available: {}",
                                                    product_name, version, update.version
                                                );
                                                dialog.message(&message)
                                                    .kind(MessageDialogKind::Info)
                                                    .title("About - Update Available")
                                                    .show(|_| {});
                                            }
                                            Ok(None) => {
                                                let message = format!("{}\nVersion {}\n\nYou are running the latest version.", product_name, version);
                                                dialog.message(&message)
                                                    .kind(MessageDialogKind::Info)
                                                    .title("About")
                                                    .show(|_| {});
                                            }
                                            Err(e) => {
                                                let message = format!("{}\nVersion {}\n\nUnable to check for updates.", product_name, version);
                                                dialog.message(&message)
                                                    .kind(MessageDialogKind::Info)
                                                    .title("About")
                                                    .show(|_| {});
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        let message = format!("{}\nVersion {}", product_name, version);
                                        dialog.message(&message)
                                            .kind(MessageDialogKind::Info)
                                            .title("About")
                                            .show(|_| {});
                                    }
                                }
                            });
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

            // Ensure modal window exists - access it to trigger creation from config
            if let Some(modal_window) = app.get_webview_window("file-organization") {
                let modal_window_clone = modal_window.clone();
                modal_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        // Allow modal to close normally
                    }
                });
            }

            let event_tx = init_watcher().map_err(|e| {
                eprintln!("Failed to initialize watcher: {}", e);
                e
            })?;

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
            close_file_organization_modal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

