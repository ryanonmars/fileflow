mod commands;
mod config;
mod file_organizer;
mod file_watcher;

use commands::*;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|_app| {
            init_watcher().map_err(|e| {
                eprintln!("Failed to initialize watcher: {}", e);
                e
            })?;
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
            move_file_manual
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

