use crate::config::{Config, PendingFile};
use crate::file_watcher::FileWatcher;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tauri::Emitter;
use tauri::Manager;

static WATCHER: Mutex<Option<Arc<Mutex<FileWatcher>>>> = Mutex::new(None);

static MODAL_SHOWING: Mutex<bool> = Mutex::new(false);

pub fn init_watcher() -> Result<broadcast::Sender<String>, String> {
    eprintln!("init_watcher() called");
    let (tx, _) = broadcast::channel(100);
    let watcher = FileWatcher::new(tx.clone())?;
    let watcher_arc = Arc::new(Mutex::new(watcher));
    *WATCHER.lock().unwrap() = Some(watcher_arc);
    eprintln!("init_watcher() completed successfully");
    Ok(tx)
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    eprintln!("[COMMAND] ========== get_config() CALLED ==========");
    let config = Config::load();
    eprintln!("[COMMAND] get_config() returning:");
    eprintln!("[COMMAND]   watched_folder={:?}", config.watched_folder);
    eprintln!("[COMMAND]   organization_mode={}", config.organization_mode);
    eprintln!("[COMMAND]   rules count={}", config.rules.len());
    
    // Serialize to see what JSON looks like
    if let Ok(json) = serde_json::to_string(&config) {
        eprintln!("[COMMAND]   JSON (first 200 chars): {}", &json[..json.len().min(200)]);
    }
    
    Ok(config)
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: Config) -> Result<(), String> {
    eprintln!("[COMMAND] ========== save_config() CALLED ==========");
    eprintln!("[COMMAND] watched_folder={:?}", config.watched_folder);
    eprintln!("[COMMAND] organization_mode={}", config.organization_mode);
    eprintln!("[COMMAND] rules count={}", config.rules.len());
    
    config.save().map_err(|e| {
        eprintln!("[COMMAND] ❌ ERROR saving config: {}", e);
        e
    })?;
    eprintln!("[COMMAND] ✅ Config saved successfully");

    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let watcher = watcher_arc.lock().unwrap();
        watcher.update_config(config.clone())?;
    }

    // Apply settings immediately
    apply_settings(&app, &config)?;

    Ok(())
}

fn apply_settings(_app: &tauri::AppHandle, config: &Config) -> Result<(), String> {
    // Handle launch at login
    #[cfg(target_os = "macos")]
    {
        set_launch_at_login(config.launch_at_login)?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    use std::process::Command;
    
    // Get the app bundle path dynamically
    let app_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable: {}", e))?;
    
    // Navigate up to the .app bundle
    let app_bundle = app_path
        .ancestors()
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("app"));
    
    // If we can't find .app bundle, we're in dev mode - skip silently
    let app_bundle = match app_bundle {
        Some(bundle) => bundle,
        None => {
            // In dev mode, just save the preference without error
            return Ok(());
        }
    };
    
    let app_path_str = app_bundle.to_string_lossy();
    let app_name = app_bundle
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("FileFlow");
    
    if enabled {
        // First, try to remove any existing entry to avoid duplicates
        let remove_script = format!(
            "tell application \"System Events\"\n\
             try\n\
                 delete (every login item whose name is \"{}\")\n\
             end try\n\
             end tell",
            app_name
        );
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(&remove_script)
            .output();
        
        // Add to login items using osascript
        let add_script = format!(
            "tell application \"System Events\" to make login item at end with properties {{path:\"{}\", hidden:false}}",
            app_path_str
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&add_script)
            .output()
            .map_err(|e| format!("Failed to enable launch at login: {}", e))?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to enable launch at login: {}", error));
        }
    } else {
        // Remove from login items by name
        let script = format!(
            "tell application \"System Events\"\n\
             try\n\
                 delete (every login item whose name is \"{}\")\n\
             end try\n\
             end tell",
            app_name
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to disable launch at login: {}", e))?;
        
        // Don't error if it wasn't in login items - this is expected
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            // Only error if it's not a "not found" type error
            if !error.contains("Can't get") && !error.contains("doesn't understand") && !error.is_empty() {
                return Err(format!("Failed to disable launch at login: {}", error));
            }
        }
    }
    
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_launch_at_login(_enabled: bool) -> Result<(), String> {
    // Not implemented for other platforms yet
    Ok(())
}

#[tauri::command]
pub fn start_watching(watched_folder: String) -> Result<(), String> {
    eprintln!("[COMMAND] ========== start_watching CALLED ==========");
    eprintln!("[COMMAND] watched_folder parameter: '{}'", watched_folder);
    eprintln!("[COMMAND] watched_folder length: {}", watched_folder.len());
    eprintln!("[COMMAND] watched_folder is_empty: {}", watched_folder.is_empty());
    
    let path = Path::new(&watched_folder);
    eprintln!("[COMMAND] Path exists: {}", path.exists());
    
    if !path.exists() {
        eprintln!("[COMMAND] Path does not exist, creating: {}", path.display());
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create watched folder: {}", e))?;
    }

    let mut watcher_guard = WATCHER.lock().unwrap();
    if watcher_guard.is_none() {
        eprintln!("[COMMAND] Watcher is None, reinitializing...");
        // Reinitialize if it was destroyed
        let (tx, _) = broadcast::channel(100);
        let watcher = FileWatcher::new(tx.clone())?;
        *watcher_guard = Some(Arc::new(Mutex::new(watcher)));
        eprintln!("[COMMAND] Watcher reinitialized");
    }

    if let Some(watcher_arc) = watcher_guard.as_ref() {
        let mut watcher = watcher_arc.lock().unwrap();
        eprintln!("[COMMAND] Calling watcher.watch() for path: {}", path.display());
        match watcher.watch(path) {
            Ok(_) => {
                eprintln!("[COMMAND] ✅ Successfully started watching: {}", path.display());
                Ok(())
            }
            Err(e) => {
                eprintln!("[COMMAND] ❌ ERROR watching path: {}", e);
                Err(e)
            }
        }
    } else {
        eprintln!("[COMMAND] ❌ ERROR: Watcher not initialized");
        Err("Watcher not initialized".to_string())
    }
}

#[tauri::command]
pub fn stop_watching() -> Result<(), String> {
    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let mut watcher = watcher_arc.lock().unwrap();
        watcher.unwatch_current()?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_organization_mode() -> Result<String, String> {
    let config = Config::load();
    Ok(config.organization_mode)
}

#[tauri::command]
pub fn set_organization_mode(mode: String) -> Result<(), String> {
    eprintln!("[COMMAND] ========== set_organization_mode() CALLED ==========");
    eprintln!("[COMMAND] mode parameter: '{}'", mode);
    
    if mode != "auto" && mode != "ask" && mode != "both" {
        eprintln!("[COMMAND] ❌ ERROR: Invalid organization mode");
        return Err("Invalid organization mode. Must be 'auto', 'ask', or 'both'".to_string());
    }

    let mut config = Config::load();
    eprintln!("[COMMAND] Current config - watched_folder: {:?}, organization_mode: {}", 
        config.watched_folder, config.organization_mode);
    config.organization_mode = mode.clone();
    eprintln!("[COMMAND] Setting organization_mode to: {}", mode);
    
    config.save().map_err(|e| {
        eprintln!("[COMMAND] ❌ ERROR saving config: {}", e);
        e
    })?;
    eprintln!("[COMMAND] ✅ Config saved successfully with organization_mode: {}", mode);

    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let watcher = watcher_arc.lock().unwrap();
        watcher.update_config(config)?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_pending_files() -> Result<Vec<PendingFile>, String> {
    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let watcher = watcher_arc.lock().unwrap();
        Ok(watcher.get_pending_files())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn process_pending_file(app: tauri::AppHandle, filePath: String, destination: Option<String>, newName: Option<String>) -> Result<(), String> {
    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let watcher = watcher_arc.lock().unwrap();
        watcher.process_pending_file(&filePath, destination, newName)?;
        
        // Refresh the file list in the modal instead of closing it
        if let Some(window) = app.get_webview_window("file-organization") {
            let js_code = "if (window.refreshFileList) { window.refreshFileList(); }";
            let _ = window.eval(js_code);
        }
        
        // If no more pending files, close the modal
        let pending_count = watcher.get_pending_files().len();
        if pending_count == 0 {
            *MODAL_SHOWING.lock().unwrap() = false;
            let _ = close_file_organization_modal(app);
        }
    } else {
        return Err("Watcher not initialized".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn move_file_manual(file_path: String, destination: String) -> Result<String, String> {
    use crate::file_organizer::organize_file_to_destination;
    use std::path::PathBuf;
    
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

        organize_file_to_destination(&path, &destination, None)
}

#[tauri::command]
pub async fn show_file_notification(
    app: tauri::AppHandle,
    file_path: String,
    file_name: String,
    file_size: u64,
) -> Result<(), String> {
    let _size_str = if file_size < 1024 {
        format!("{} B", file_size)
    } else if file_size < 1024 * 1024 {
        format!("{:.1} KB", file_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0))
    };

    // Emit event for frontend to handle notification
    app.emit("file-queued", serde_json::json!({
        "path": file_path,
        "name": file_name,
        "size": file_size
    }))
    .map_err(|e| format!("Failed to emit notification event: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn process_file_from_notification(
    file_path: String,
    action: String,
) -> Result<(), String> {
    if action == "skip" {
        if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
            let watcher = watcher_arc.lock().unwrap();
            watcher.process_pending_file(&file_path, None, None)?;
        }
        Ok(())
    } else {
        Err("Unknown action".to_string())
    }
}

#[tauri::command]
pub fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        window.show().map_err(|e| format!("Failed to show window: {}", e))?;
        window.set_focus().map_err(|e| format!("Failed to focus window: {}", e))?;
    }
    Ok(())
}

fn show_file_organization_modal_internal(app: tauri::AppHandle) -> Result<(), String> {
    // Get the window (should be created from config)
    let window = app.get_webview_window("file-organization")
        .ok_or_else(|| "File organization window not found. Please restart the app to ensure the window is created.".to_string())?;
    
    // Show window - it will load index.html which will detect it's the modal window
    window.show().map_err(|e| format!("Failed to show window: {}", e))?;
    
    // Trigger a refresh of the pending files list in the modal
    let js_code = "if (window.refreshFileList) { window.refreshFileList(); }";
    std::thread::sleep(std::time::Duration::from_millis(300));
    let _ = window.eval(js_code);
    
    window.set_focus().map_err(|e| format!("Failed to focus window: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub fn show_file_organization_modal(
    app: tauri::AppHandle,
    _file_path: String,
    _file_name: String,
    _file_size: u64,
) -> Result<(), String> {
    // Check if modal is already showing
    {
        let mut showing = MODAL_SHOWING.lock().unwrap();
        if *showing {
            // Modal already open, just refresh the file list
            if let Some(window) = app.get_webview_window("file-organization") {
                let js_code = "if (window.refreshFileList) { window.refreshFileList(); }";
                let _ = window.eval(js_code);
            }
            return Ok(());
        }
        *showing = true;
    }
    
    // Show the modal window (it will display all pending files)
    show_file_organization_modal_internal(app)?;
    
    Ok(())
}

#[tauri::command]
pub fn close_file_organization_modal(app: tauri::AppHandle) -> Result<(), String> {
    *MODAL_SHOWING.lock().unwrap() = false;
    if let Some(window) = app.get_webview_window("file-organization") {
        window.hide().map_err(|e| format!("Failed to hide window: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> Result<(String, String), String> {
    let version = app.package_info().version.to_string();
    let name = app.package_info().name.clone();
    Ok((version, name))
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .output()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .output()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .output()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn suppress_update_alert_for_days(days: i64) -> Result<(), String> {
    let mut config = crate::config::Config::load();
    config.suppress_update_alert_for_days(days);
    config.save()?;
    Ok(())
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        // In dev mode, emit update-latest after a small delay to show the checking state briefly
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let _ = app_clone.emit("update-latest", serde_json::json!({}));
        });
        return Ok(());
    }
    
    #[cfg(not(debug_assertions))]
    {
        use tauri_plugin_updater::UpdaterExt;
        
        let updater_builder = app.updater_builder();
        
        match updater_builder.build() {
            Ok(updater) => {
                match updater.check().await {
                    Ok(Some(update)) => {
                        let _ = app.emit("update-available", serde_json::json!({
                            "version": update.version.to_string()
                        }));
                    }
                    Ok(None) => {
                        let _ = app.emit("update-latest", serde_json::json!({}));
                    }
                    Err(e) => {
                        let _ = app.emit("update-error", serde_json::json!({
                            "message": format!("Unable to check for updates: {}", e)
                        }));
                    }
                }
            }
            Err(e) => {
                let _ = app.emit("update-error", serde_json::json!({
                    "message": format!("Unable to initialize updater: {}", e)
                }));
            }
        }
        
        Ok(())
    }
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        return Err("Update installation is disabled in development mode.".to_string());
    }
    
    #[cfg(not(debug_assertions))]
    {
        use tauri_plugin_updater::UpdaterExt;
        
        let updater_builder = app.updater_builder();
        
        match updater_builder.build() {
            Ok(updater) => {
                match updater.check().await {
                    Ok(Some(update)) => {
                        let _ = app.emit("update-installing", serde_json::json!({}));
                        
                        if let Err(e) = update.download_and_install(|_chunk_length, _content_length| {}, || {}).await {
                            let error_msg = format!("Failed to install update: {}", e);
                            let _ = app.emit("update-error", serde_json::json!({
                                "message": error_msg.clone()
                            }));
                            return Err(error_msg);
                        } else {
                            // Update installed successfully - restart the app
                            // The download_and_install should handle restart, but we'll ensure it happens
                            let _ = app.restart();
                        }
                    }
                    Ok(None) => {
                        let error_msg = "No update available".to_string();
                        let _ = app.emit("update-error", serde_json::json!({
                            "message": error_msg.clone()
                        }));
                        return Err(error_msg);
                    }
                    Err(e) => {
                        let error_msg = format!("Unable to check for updates: {}", e);
                        let _ = app.emit("update-error", serde_json::json!({
                            "message": error_msg.clone()
                        }));
                        return Err(error_msg);
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Unable to initialize updater: {}", e);
                let _ = app.emit("update-error", serde_json::json!({
                    "message": error_msg.clone()
                }));
                return Err(error_msg);
            }
        }
    }
}

