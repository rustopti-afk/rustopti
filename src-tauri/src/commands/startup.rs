use serde::Serialize;
use tauri::State;
use winreg::enums::*;
use winreg::RegKey;
use winreg::HKEY;
use crate::utils::license_guard::{LicenseState, require_license};

#[derive(Debug, Serialize)]
pub struct StartupItem {
    pub name: String,
    pub command: String,
    pub location: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct StartupResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

/// Known bloatware startup items that are safe to disable
const SAFE_TO_DISABLE: &[&str] = &[
    "OneDrive", "Cortana", "Teams", "Skype", "Discord",
    "Spotify", "Steam", "EpicGamesLauncher", "Origin",
    "iTunesHelper", "AdobeGCInvoker", "CCleaner",
];

#[tauri::command]
pub fn get_startup_items() -> Result<Vec<StartupItem>, String> {
    let mut items = Vec::new();

    // HKCU Run
    scan_run_key(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Run", "HKCU\\Run", &mut items);

    // HKLM Run
    scan_run_key(HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Run", "HKLM\\Run", &mut items);

    // HKCU RunOnce
    scan_run_key(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\RunOnce", "HKCU\\RunOnce", &mut items);

    Ok(items)
}

fn scan_run_key(hkey: HKEY, subkey: &str, location: &str, items: &mut Vec<StartupItem>) {
    let root = RegKey::predef(hkey);
    if let Ok(key) = root.open_subkey(subkey) {
        for value in key.enum_values() {
            if let Ok((name, data)) = value {
                items.push(StartupItem {
                    name: name.clone(),
                    command: format!("{:?}", data),
                    location: location.to_string(),
                    enabled: true,
                });
            }
        }
    }
}

#[tauri::command]
pub fn disable_startup_item(name: String, location: String, state: State<'_, LicenseState>) -> Result<StartupResult, String> {
    require_license(&state)?;
    let (hkey, subkey) = parse_location(&location)?;
    let root = RegKey::predef(hkey);

    match root.open_subkey_with_flags(subkey, winreg::enums::KEY_SET_VALUE) {
        Ok(key) => {
            match key.delete_value(&name) {
                Ok(_) => Ok(StartupResult {
                    name: name.clone(),
                    success: true,
                    message: format!("✓ Disabled startup: {}", name),
                }),
                Err(e) => Ok(StartupResult {
                    name: name.clone(),
                    success: false,
                    message: format!("✗ Failed to disable {}: {}", name, e),
                }),
            }
        }
        Err(e) => Err(format!("Failed to open registry: {}", e)),
    }
}

#[tauri::command]
pub fn get_disable_recommendations() -> Result<Vec<String>, String> {
    let items = get_startup_items()?;
    let mut recommendations = Vec::new();

    for item in &items {
        for &bloat in SAFE_TO_DISABLE {
            if item.name.to_lowercase().contains(&bloat.to_lowercase()) {
                recommendations.push(format!("→ Consider disabling: {} ({})", item.name, item.location));
            }
        }
    }

    if recommendations.is_empty() {
        recommendations.push("✓ No known bloatware in startup".to_string());
    }

    Ok(recommendations)
}

fn parse_location(location: &str) -> Result<(HKEY, &str), String> {
    match location {
        "HKCU\\Run" => Ok((HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Run")),
        "HKLM\\Run" => Ok((HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Run")),
        "HKCU\\RunOnce" => Ok((HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\RunOnce")),
        _ => Err(format!("Unknown location: {}", location)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_startup_items() {
        let items = get_startup_items();
        assert!(items.is_ok());
        // May or may not have items, but should not error
    }

    #[test]
    fn test_parse_location() {
        assert!(parse_location("HKCU\\Run").is_ok());
        assert!(parse_location("HKLM\\Run").is_ok());
        assert!(parse_location("Invalid").is_err());
    }

    #[test]
    fn test_get_recommendations() {
        let recs = get_disable_recommendations();
        assert!(recs.is_ok());
    }
}
