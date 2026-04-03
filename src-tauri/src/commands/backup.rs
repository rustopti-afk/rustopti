use serde::{Serialize, Deserialize};
use tauri::State;
use chrono::Local;
use std::process::Command;
use std::os::windows::process::CommandExt;
use crate::utils::license_guard::{LicenseState, require_license};

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupEntry {
    pub timestamp: String,
    pub action: String,
    pub details: String,
}

#[derive(Debug, Serialize)]
pub struct BackupResult {
    pub success: bool,
    pub message: String,
}

/// Sanitize user input for safe use in PowerShell strings.
/// Removes characters that could break out of quoted strings.
fn sanitize_for_powershell(input: &str) -> String {
    input.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '.')
        .take(100) // Limit length
        .collect()
}

/// Validate filename: no path separators, no traversal, must end with .reg
fn is_safe_filename(name: &str) -> bool {
    !name.contains('/') &&
    !name.contains('\\') &&
    !name.contains("..") &&
    name.ends_with(".reg") &&
    name.len() < 256 &&
    name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

#[tauri::command]
pub fn create_restore_point(description: String, state: State<'_, LicenseState>) -> Result<BackupResult, String> {
    require_license(&state)?;

    // Sanitize description to prevent PowerShell injection
    let safe_desc = sanitize_for_powershell(&description);

    let ps_cmd = format!(
        r#"Checkpoint-Computer -Description "{}" -RestorePointType "MODIFY_SETTINGS""#,
        safe_desc
    );

    match Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) if o.status.success() => Ok(BackupResult {
            success: true,
            message: format!("✓ Restore point created: {}", safe_desc),
        }),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            Ok(BackupResult {
                success: false,
                message: format!("✗ Failed (needs admin): {}", stderr.chars().take(200).collect::<String>()),
            })
        }
        Err(e) => Ok(BackupResult {
            success: false,
            message: format!("✗ {}", e),
        }),
    }
}

#[tauri::command]
pub fn export_registry_backup(keys: Vec<String>, state: State<'_, LicenseState>) -> Result<Vec<BackupResult>, String> {
    require_license(&state)?;

    let mut results = Vec::new();
    let backup_dir = get_backup_dir()?;

    // Only allow known safe registry paths
    let allowed_prefixes = [
        r"HKCU\Software\Microsoft",
        r"HKCU\Control Panel",
        r"HKCU\System\GameConfigStore",
        r"HKLM\SOFTWARE\Microsoft",
        r"HKLM\SOFTWARE\NVIDIA",
        r"HKLM\SYSTEM\CurrentControlSet",
    ];

    for key_path in &keys {
        // Validate registry path against allowlist
        let allowed = allowed_prefixes.iter().any(|prefix| key_path.starts_with(prefix));
        if !allowed {
            results.push(BackupResult {
                success: false,
                message: format!("✗ Registry path not allowed: {}", key_path),
            });
            continue;
        }

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let safe_name = key_path
            .replace('\\', "_")
            .replace(' ', "_");
        let filename = format!("{}_{}.reg", safe_name, timestamp);
        let filepath = backup_dir.join(&filename);

        let output = Command::new("reg")
            .args(["export", key_path, &filepath.to_string_lossy(), "/y"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match output {
            Ok(o) if o.status.success() => results.push(BackupResult {
                success: true,
                message: format!("✓ Exported: {} → {}", key_path, filename),
            }),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                results.push(BackupResult {
                    success: false,
                    message: format!("✗ Failed to export {}: {}", key_path, stderr.trim()),
                });
            }
            Err(e) => results.push(BackupResult {
                success: false,
                message: format!("✗ {}: {}", key_path, e),
            }),
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn restore_registry_backup(filename: String, state: State<'_, LicenseState>) -> Result<BackupResult, String> {
    require_license(&state)?;

    // Prevent path traversal
    if !is_safe_filename(&filename) {
        return Ok(BackupResult {
            success: false,
            message: "✗ Invalid filename".to_string(),
        });
    }

    let backup_dir = get_backup_dir()?;
    let filepath = backup_dir.join(&filename);

    // Double-check the resolved path is still inside backup_dir
    let canonical = filepath.canonicalize().unwrap_or(filepath.clone());
    let canonical_dir = backup_dir.canonicalize().unwrap_or(backup_dir.clone());
    if !canonical.starts_with(&canonical_dir) {
        return Ok(BackupResult {
            success: false,
            message: "✗ Access denied: path outside backup directory".to_string(),
        });
    }

    if !filepath.exists() {
        return Ok(BackupResult {
            success: false,
            message: format!("✗ Backup file not found: {}", filename),
        });
    }

    match Command::new("reg")
        .args(["import", &filepath.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) if o.status.success() => Ok(BackupResult {
            success: true,
            message: format!("✓ Registry restored from: {}", filename),
        }),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            Ok(BackupResult {
                success: false,
                message: format!("✗ Restore failed: {}", stderr.trim()),
            })
        }
        Err(e) => Ok(BackupResult {
            success: false,
            message: format!("✗ {}", e),
        }),
    }
}

#[tauri::command]
pub fn list_backups() -> Result<Vec<String>, String> {
    let backup_dir = get_backup_dir()?;
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&backup_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".reg") {
                    files.push(name.to_string());
                }
            }
        }
    }

    files.sort_by(|a, b| b.cmp(a));
    Ok(files)
}

#[tauri::command]
pub fn backup_all_before_optimization(state: State<'_, LicenseState>) -> Result<Vec<BackupResult>, String> {
    require_license(&state)?;
    let keys = vec![
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR".to_string(),
        r"HKCU\Control Panel\Mouse".to_string(),
        r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile".to_string(),
        r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters".to_string(),
        r"HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers".to_string(),
    ];

    let mut results = Vec::new();

    let rp = create_restore_point("RustOpti Optimization".to_string(), state.clone());
    if let Ok(r) = rp {
        results.push(r);
    }

    let exports = export_registry_backup(keys, state)?;
    results.extend(exports);

    let entry = BackupEntry {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        action: "Full Backup".to_string(),
        details: "Pre-optimization backup created".to_string(),
    };
    let _ = save_backup_log(entry);

    Ok(results)
}

fn get_backup_dir() -> Result<std::path::PathBuf, String> {
    let dir = if let Ok(appdata) = std::env::var("APPDATA") {
        std::path::PathBuf::from(appdata).join("RustOpti").join("backups")
    } else {
        std::path::PathBuf::from(r"C:\RustOpti\backups")
    };

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create backup dir: {}", e))?;
    Ok(dir)
}

fn save_backup_log(entry: BackupEntry) -> Result<(), String> {
    let dir = get_backup_dir()?;
    let log_path = dir.join("backup_log.json");

    let mut entries: Vec<BackupEntry> = if log_path.exists() {
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    entries.push(entry);

    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&log_path, json).map_err(|e| e.to_string())
}
