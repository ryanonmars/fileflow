use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RuleCondition {
    #[serde(rename = "filetype")]
    FileType { value: String },
    #[serde(rename = "name")]
    Name { pattern: String },
    #[serde(rename = "created_date")]
    CreatedDate { 
        operator: String, // "before", "after", "on"
        value: String // ISO date string or relative date
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default)]
    pub name: Option<String>,
    pub condition: RuleCondition,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub detected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub watched_folder: Option<String>,
    #[serde(default = "default_organization_mode")]
    pub organization_mode: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
    // Keep mappings for backward compatibility during migration
    #[serde(default)]
    pub mappings: std::collections::HashMap<String, String>,
    #[serde(default = "default_show_menu_bar_icon")]
    pub show_menu_bar_icon: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default)]
    pub auto_check_for_updates: bool,
    #[serde(default)]
    pub update_alert_suppress_until: Option<i64>, // Unix timestamp - suppress alerts until this time
}

fn default_show_menu_bar_icon() -> bool {
    true
}

fn default_organization_mode() -> String {
    "auto".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            watched_folder: None,
            organization_mode: "auto".to_string(),
            rules: Vec::new(),
            mappings: std::collections::HashMap::new(),
            show_menu_bar_icon: true,
            launch_at_login: false,
            auto_check_for_updates: true,
            update_alert_suppress_until: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("folder-watcher");
        path.push("config.json");
        path
    }

    pub fn get_destination_folder(
        &self,
        file_path: &PathBuf,
        file_extension: &str,
        file_name: &str,
        created_date: Option<std::time::SystemTime>,
    ) -> Option<String> {
        // Only use rules-based matching - no fallbacks
        for rule in &self.rules {
            if rule.matches(file_path, file_extension, file_name, created_date) {
                if !rule.destination.is_empty() {
                    return Some(rule.destination.clone());
                }
            }
        }
        None
    }
    
    pub fn should_show_update_alert(&self) -> bool {
        if let Some(suppress_until) = self.update_alert_suppress_until {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            now >= suppress_until
        } else {
            true
        }
    }
    
    pub fn suppress_update_alert_for_days(&mut self, days: i64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let seconds_in_days = days * 24 * 60 * 60;
        self.update_alert_suppress_until = Some(now + seconds_in_days);
    }
}

impl Rule {
    pub fn matches(
        &self,
        file_path: &PathBuf,
        file_extension: &str,
        file_name: &str,
        created_date: Option<std::time::SystemTime>,
    ) -> bool {
        match &self.condition {
            RuleCondition::FileType { value } => {
                let ext = file_extension.to_lowercase();
                value.to_lowercase() == ext || value.to_lowercase() == "*"
            }
            RuleCondition::Name { pattern } => {
                // Simple pattern matching - supports * wildcard
                if pattern.contains('*') {
                    let regex_pattern = pattern
                        .replace("*", ".*")
                        .replace(".", "\\.");
                    if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
                        re.is_match(file_name)
                    } else {
                        file_name.contains(pattern)
                    }
                } else {
                    file_name.contains(pattern)
                }
            }
            RuleCondition::CreatedDate { operator, value } => {
                if let Some(created) = created_date {
                    // Parse the value as a date and compare
                    // For now, simple implementation - can be enhanced
                    match operator.as_str() {
                        "before" | "after" | "on" => {
                            // TODO: Implement date parsing and comparison
                            // For now, return false to avoid incorrect matches
                            false
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
        }
    }
}

