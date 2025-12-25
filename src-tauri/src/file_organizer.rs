use crate::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

pub fn organize_file(file_path: &Path, config: &Config) -> Result<String, String> {
    let extension_str = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| "other".to_string());

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let created_date = file_path
        .metadata()
        .ok()
        .and_then(|m| m.created().ok());

    let path_buf = file_path.to_path_buf();

    let destination = config
        .get_destination_folder(&path_buf, &extension_str, file_name, created_date)
        .ok_or_else(|| format!("No matching rule found for file: {}", file_name))?;

    let dest_path = PathBuf::from(&destination);
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path)
            .map_err(|e| format!("Failed to create destination folder: {}", e))?;
    }

    let file_name_obj = file_path
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?;

    let mut dest_file = dest_path.join(file_name_obj);
    let mut counter = 1;

    while dest_file.exists() {
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let new_name = if ext.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, ext)
        };
        dest_file = dest_path.join(&new_name);
        counter += 1;
    }

    fs::rename(file_path, &dest_file)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    Ok(dest_file.to_string_lossy().to_string())
}

pub fn organize_file_to_destination(file_path: &Path, destination: &str) -> Result<String, String> {
    let dest_path = PathBuf::from(destination);
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path)
            .map_err(|e| format!("Failed to create destination folder: {}", e))?;
    }

    let file_name_obj = file_path
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?;

    let mut dest_file = dest_path.join(file_name_obj);
    let mut counter = 1;

    while dest_file.exists() {
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let new_name = if ext.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, ext)
        };
        dest_file = dest_path.join(&new_name);
        counter += 1;
    }

    fs::rename(file_path, &dest_file)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    Ok(dest_file.to_string_lossy().to_string())
}

