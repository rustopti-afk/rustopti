use serde::Serialize;
use tauri::State;
use std::fs;
use std::path::PathBuf;
use crate::utils::license_guard::{LicenseState, require_license};

#[derive(Debug, Serialize)]
pub struct CleanupResult {
    pub category: String,
    pub files_deleted: u64,
    pub bytes_freed: u64,
    pub message: String,
}

#[tauri::command]
pub fn get_cleanup_preview() -> Result<Vec<CleanupPreview>, String> {
    let mut previews = Vec::new();

    // Temp folder
    if let Ok(temp) = std::env::var("TEMP") {
        let (count, size) = scan_directory(&PathBuf::from(&temp));
        previews.push(CleanupPreview {
            category: "Windows Temp".to_string(),
            path: temp,
            file_count: count,
            size_mb: size as f64 / 1_048_576.0,
        });
    }

    // Windows Temp
    let win_temp = PathBuf::from(r"C:\Windows\Temp");
    if win_temp.exists() {
        let (count, size) = scan_directory(&win_temp);
        previews.push(CleanupPreview {
            category: "System Temp".to_string(),
            path: win_temp.to_string_lossy().to_string(),
            file_count: count,
            size_mb: size as f64 / 1_048_576.0,
        });
    }

    // Prefetch
    let prefetch = PathBuf::from(r"C:\Windows\Prefetch");
    if prefetch.exists() {
        let (count, size) = scan_directory(&prefetch);
        previews.push(CleanupPreview {
            category: "Prefetch".to_string(),
            path: prefetch.to_string_lossy().to_string(),
            file_count: count,
            size_mb: size as f64 / 1_048_576.0,
        });
    }

    // Recent files
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let recent = PathBuf::from(&user_profile).join("AppData").join("Roaming").join("Microsoft").join("Windows").join("Recent");
        if recent.exists() {
            let (count, size) = scan_directory(&recent);
            previews.push(CleanupPreview {
                category: "Recent Files".to_string(),
                path: recent.to_string_lossy().to_string(),
                file_count: count,
                size_mb: size as f64 / 1_048_576.0,
            });
        }
    }

    Ok(previews)
}

#[derive(Debug, Serialize)]
pub struct CleanupPreview {
    pub category: String,
    pub path: String,
    pub file_count: u64,
    pub size_mb: f64,
}

#[tauri::command]
pub fn run_disk_cleanup(state: State<'_, LicenseState>) -> Result<Vec<CleanupResult>, String> {
    require_license(&state)?;
    let mut results = Vec::new();

    // Clean user TEMP
    if let Ok(temp) = std::env::var("TEMP") {
        results.push(clean_directory("Windows Temp", &PathBuf::from(temp)));
    }

    // Clean Windows Temp
    let win_temp = PathBuf::from(r"C:\Windows\Temp");
    if win_temp.exists() {
        results.push(clean_directory("System Temp", &win_temp));
    }

    // Clean Prefetch
    let prefetch = PathBuf::from(r"C:\Windows\Prefetch");
    if prefetch.exists() {
        results.push(clean_directory("Prefetch", &prefetch));
    }

    Ok(results)
}

fn scan_directory(path: &PathBuf) -> (u64, u64) {
    let mut count = 0u64;
    let mut total_size = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    count += 1;
                    total_size += meta.len();
                }
            }
        }
    }
    (count, total_size)
}

fn clean_directory(category: &str, path: &PathBuf) -> CleanupResult {
    let mut deleted = 0u64;
    let mut freed = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let size = meta.len();
                    if fs::remove_file(entry.path()).is_ok() {
                        deleted += 1;
                        freed += size;
                    }
                }
            }
        }
    }

    CleanupResult {
        category: category.to_string(),
        files_deleted: deleted,
        bytes_freed: freed,
        message: format!("✓ {} — deleted {} files, freed {:.1} MB",
            category, deleted, freed as f64 / 1_048_576.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cleanup_preview() {
        let preview = get_cleanup_preview();
        assert!(preview.is_ok());
        let items = preview.unwrap();
        // Should find at least TEMP directory
        assert!(!items.is_empty(), "Should find at least one cleanup target");
    }

    #[test]
    fn test_scan_nonexistent_directory() {
        let (count, size) = scan_directory(&PathBuf::from(r"C:\NonExistentDir12345"));
        assert_eq!(count, 0);
        assert_eq!(size, 0);
    }
}
