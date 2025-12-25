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
pub struct Config {
    pub watched_folder: Option<String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    // Keep mappings for backward compatibility during migration
    #[serde(default)]
    pub mappings: std::collections::HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            watched_folder: None,
            rules: Vec::new(),
            mappings: std::collections::HashMap::new(),
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
        // First, try rules-based matching
        for rule in &self.rules {
            if rule.matches(file_path, file_extension, file_name, created_date) {
                if !rule.destination.is_empty() {
                    return Some(rule.destination.clone());
                }
            }
        }

        // Fallback to old mappings for backward compatibility
        let ext = file_extension.to_lowercase();
        if let Some(folder) = self.mappings.get(&ext) {
            if !folder.is_empty() {
                return Some(folder.clone());
            }
        }
        self.mappings.get("other").and_then(|f| {
            if !f.is_empty() {
                Some(f.clone())
            } else {
                None
            }
        })
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

