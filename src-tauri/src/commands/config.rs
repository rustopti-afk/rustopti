use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub language: Option<String>,
    pub run_as_admin_acknowledged: Option<bool>,
}

fn get_config_path(_app: &AppHandle) -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    let mut path = PathBuf::from(appdata);
    path.push("RustOpti");
    
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    
    path.push("config.json");
    Ok(path)
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = get_config_path(&app)?;
    
    if !path.exists() {
        let default_config = AppConfig {
            language: Some("en".to_string()),
            run_as_admin_acknowledged: Some(true),
        };
        save_config_internal(&path, &default_config)?;
        return Ok(default_config);
    }
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&content).unwrap_or_default();
    
    Ok(config)
}

#[tauri::command]
pub fn update_config(app: AppHandle, new_config: AppConfig) -> Result<(), String> {
    let path = get_config_path(&app)?;
    save_config_internal(&path, &new_config)
}

fn save_config_internal(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}
