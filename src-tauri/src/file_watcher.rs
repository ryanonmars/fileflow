use crate::config::Config;
use crate::file_organizer::organize_file;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    config: Arc<Mutex<Config>>,
    event_tx: broadcast::Sender<String>,
}

impl FileWatcher {
    pub fn new(event_tx: broadcast::Sender<String>) -> Result<Self, String> {
        let config = Arc::new(Mutex::new(Config::load()));
        let config_clone = config.clone();
        let event_tx_clone = event_tx.clone();

        let watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
            match result {
                Ok(event) => {
                    if let EventKind::Create(_) = event.kind {
                        for path in event.paths {
                            if path.is_file() {
                                let path_clone = path.clone();
                                let config_clone2 = config_clone.clone();
                                let event_tx_clone2 = event_tx_clone.clone();
                                
                                std::thread::spawn(move || {
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                    if path_clone.exists() {
                                        let config = config_clone2.lock().unwrap();
                                        match organize_file(&path_clone, &config) {
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
        })
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
}

