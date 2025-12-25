use crate::config::{Config, PendingFile};
use crate::file_organizer::{organize_file, organize_file_to_destination};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use std::time::SystemTime;

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    config: Arc<Mutex<Config>>,
    event_tx: broadcast::Sender<String>,
    pending_files: Arc<Mutex<Vec<PendingFile>>>,
}

impl FileWatcher {
    pub fn new(event_tx: broadcast::Sender<String>) -> Result<Self, String> {
        let config = Arc::new(Mutex::new(Config::load()));
        let pending_files = Arc::new(Mutex::new(Vec::<PendingFile>::new()));
        let config_clone = config.clone();
        let pending_files_clone = pending_files.clone();
        let event_tx_clone = event_tx.clone();

        let watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
            match result {
                Ok(event) => {
                    if let EventKind::Create(_) = event.kind {
                        for path in event.paths {
                            if path.is_file() {
                                let path_clone = path.clone();
                                let config_clone2 = config_clone.clone();
                                let pending_files_clone2 = pending_files_clone.clone();
                                let event_tx_clone2 = event_tx_clone.clone();
                                
                                std::thread::spawn(move || {
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                    if path_clone.exists() {
                                        let config = config_clone2.lock().unwrap();
                                        let mode = config.organization_mode.as_str();
                                        
                                        match mode {
                                            "auto" => {
                                                let extension_str = path_clone
                                                    .extension()
                                                    .and_then(|ext| ext.to_str())
                                                    .map(|e| e.to_lowercase())
                                                    .unwrap_or_else(|| "other".to_string());
                                                
                                                let file_name = path_clone
                                                    .file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("");
                                                
                                                let created_date = path_clone
                                                    .metadata()
                                                    .ok()
                                                    .and_then(|m| m.created().ok());
                                                
                                                if let Some(destination) = config.get_destination_folder(
                                                    &path_clone,
                                                    &extension_str,
                                                    file_name,
                                                    created_date,
                                                ) {
                                                    match organize_file_to_destination(&path_clone, &destination) {
                                                        Ok(dest) => {
                                                            let _ = event_tx_clone2.send(format!(
                                                                "Moved: {} -> {}",
                                                                path_clone.display(),
                                                                dest
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            let _ = event_tx_clone2.send(format!(
                                                                "Error organizing {}: {}",
                                                                path_clone.display(),
                                                                e
                                                            ));
                                                        }
                                                    }
                                                } else {
                                                    let _ = event_tx_clone2.send(format!(
                                                        "No rule found for {} - file not moved",
                                                        path_clone.display()
                                                    ));
                                                }
                                            }
                                            "ask" => {
                                                if let Err(e) = Self::add_pending_file_internal(
                                                    &path_clone,
                                                    &pending_files_clone2,
                                                    &event_tx_clone2,
                                                ) {
                                                    let _ = event_tx_clone2.send(format!(
                                                        "Error queueing {}: {}",
                                                        path_clone.display(),
                                                        e
                                                    ));
                                                }
                                            }
                                            "both" => {
                                                let extension_str = path_clone
                                                    .extension()
                                                    .and_then(|ext| ext.to_str())
                                                    .map(|e| e.to_lowercase())
                                                    .unwrap_or_else(|| "other".to_string());
                                                
                                                let file_name = path_clone
                                                    .file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("");
                                                
                                                let created_date = path_clone
                                                    .metadata()
                                                    .ok()
                                                    .and_then(|m| m.created().ok());
                                                
                                                if let Some(destination) = config.get_destination_folder(
                                                    &path_clone,
                                                    &extension_str,
                                                    file_name,
                                                    created_date,
                                                ) {
                                                    match organize_file_to_destination(&path_clone, &destination) {
                                                        Ok(dest) => {
                                                            let _ = event_tx_clone2.send(format!(
                                                                "Moved: {} -> {}",
                                                                path_clone.display(),
                                                                dest
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            let _ = event_tx_clone2.send(format!(
                                                                "Error organizing {}: {}",
                                                                path_clone.display(),
                                                                e
                                                            ));
                                                        }
                                                    }
                                                } else {
                                                    if let Err(e) = Self::add_pending_file_internal(
                                                        &path_clone,
                                                        &pending_files_clone2,
                                                        &event_tx_clone2,
                                                    ) {
                                                        let _ = event_tx_clone2.send(format!(
                                                            "Error queueing {}: {}",
                                                            path_clone.display(),
                                                            e
                                                        ));
                                                    }
                                                }
                                            }
                                            _ => {
                                                let _ = event_tx_clone2.send(format!(
                                                    "Unknown organization mode: {}",
                                                    mode
                                                ));
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = event_tx_clone.send(format!("Watch error: {}", e));
                }
            }
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        Ok(FileWatcher {
            watcher,
            config,
            event_tx,
            pending_files,
        })
    }

    fn add_pending_file_internal(
        path: &std::path::Path,
        pending_files: &Arc<Mutex<Vec<PendingFile>>>,
        event_tx: &broadcast::Sender<String>,
    ) -> Result<(), String> {
        let metadata = path.metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_string();
        
        let size = metadata.len();
        
        let detected_at = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let pending_file = PendingFile {
            path: path.to_string_lossy().to_string(),
            name: file_name.clone(),
            extension,
            size,
            detected_at: detected_at.to_string(),
        };
        
        let mut pending = pending_files.lock().unwrap();
        pending.push(pending_file.clone());
        
        let _ = event_tx.send(format!("file_queued:{}", path.display()));
        
        Ok(())
    }

    pub fn watch(&mut self, path: &Path) -> Result<(), String> {
        self.watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;
        Ok(())
    }

    pub fn update_config(&self, config: Config) -> Result<(), String> {
        *self.config.lock().unwrap() = config;
        Ok(())
    }

    pub fn get_event_receiver(&self) -> broadcast::Receiver<String> {
        self.event_tx.subscribe()
    }

    pub fn add_pending_file(&self, file_path: std::path::PathBuf) -> Result<(), String> {
        Self::add_pending_file_internal(&file_path, &self.pending_files, &self.event_tx)
    }

    pub fn get_pending_files(&self) -> Vec<PendingFile> {
        self.pending_files.lock().unwrap().clone()
    }

    pub fn remove_pending_file(&self, path: &str) -> Result<(), String> {
        let mut pending = self.pending_files.lock().unwrap();
        pending.retain(|f| f.path != path);
        Ok(())
    }

    pub fn process_pending_file(&self, file_path: &str, destination: Option<String>) -> Result<(), String> {
        let path = std::path::PathBuf::from(file_path);
        
        if !path.exists() {
            self.remove_pending_file(file_path)?;
            return Err("File no longer exists".to_string());
        }

        if let Some(dest) = destination {
            match organize_file_to_destination(&path, &dest) {
                Ok(_) => {
                    self.remove_pending_file(file_path)?;
                    let _ = self.event_tx.send(format!(
                        "Moved: {} -> {}",
                        path.display(),
                        dest
                    ));
                    Ok(())
                }
                Err(e) => Err(format!("Failed to move file: {}", e))
            }
        } else {
            self.remove_pending_file(file_path)?;
            let _ = self.event_tx.send(format!("Skipped: {}", path.display()));
            Ok(())
        }
    }
}

