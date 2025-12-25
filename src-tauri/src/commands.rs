use crate::config::{Config, PendingFile};
use crate::file_watcher::FileWatcher;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

static WATCHER: Mutex<Option<Arc<Mutex<FileWatcher>>>> = Mutex::new(None);

pub fn init_watcher() -> Result<broadcast::Sender<String>, String> {
    let (tx, _) = broadcast::channel(100);
    let watcher = FileWatcher::new(tx.clone())?;
    let watcher_arc = Arc::new(Mutex::new(watcher));
    *WATCHER.lock().unwrap() = Some(watcher_arc);
    Ok(tx)
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    Ok(Config::load())
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    config.save()?;

    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let watcher = watcher_arc.lock().unwrap();
        watcher.update_config(config)?;
    }

    Ok(())
}

#[tauri::command]
pub fn start_watching(watched_folder: String) -> Result<(), String> {
    let path = Path::new(&watched_folder);
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create watched folder: {}", e))?;
    }

    let mut watcher_guard = WATCHER.lock().unwrap();
    if watcher_guard.is_none() {
        // Reinitialize if it was destroyed
        let (tx, _) = broadcast::channel(100);
        let watcher = FileWatcher::new(tx.clone())?;
        *watcher_guard = Some(Arc::new(Mutex::new(watcher)));
    }

    if let Some(watcher_arc) = watcher_guard.as_ref() {
        let mut watcher = watcher_arc.lock().unwrap();
        watcher.watch(path)?;
        Ok(())
    } else {
        Err("Watcher not initialized".to_string())
    }
}

#[tauri::command]
pub fn stop_watching() -> Result<(), String> {
    // Don't destroy the watcher, just unwatch the current path
    // The watcher will be reused when starting again
    Ok(())
}

#[tauri::command]
pub fn get_organization_mode() -> Result<String, String> {
    let config = Config::load();
    Ok(config.organization_mode)
}

#[tauri::command]
pub fn set_organization_mode(mode: String) -> Result<(), String> {
    if mode != "auto" && mode != "ask" && mode != "both" {
        return Err("Invalid organization mode. Must be 'auto', 'ask', or 'both'".to_string());
    }

    let mut config = Config::load();
    config.organization_mode = mode.clone();
    config.save()?;

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
pub fn process_pending_file(file_path: String, destination: Option<String>) -> Result<(), String> {
    if let Some(watcher_arc) = WATCHER.lock().unwrap().as_ref() {
        let watcher = watcher_arc.lock().unwrap();
        watcher.process_pending_file(&file_path, destination)
    } else {
        Err("Watcher not initialized".to_string())
    }
}

#[tauri::command]
pub fn move_file_manual(file_path: String, destination: String) -> Result<String, String> {
    use crate::file_organizer::organize_file_to_destination;
    use std::path::PathBuf;
    
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    organize_file_to_destination(&path, &destination)
}

