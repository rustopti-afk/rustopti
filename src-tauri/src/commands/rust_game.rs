use serde::Serialize;
use tauri::State;
use std::path::PathBuf;
use crate::utils::license_guard::{LicenseState, require_license};

#[derive(Debug, Serialize)]
pub struct RustTweakResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RustGameInfo {
    pub installed: bool,
    pub install_path: Option<String>,
    pub current_launch_options: Option<String>,
}

/// Recommended launch options for Rust
const LAUNCH_OPTIONS: &str = "-high -nolog -nopopupwindow -window-mode exclusive";

/// Recommended console commands
const CONSOLE_COMMANDS: &[(&str, &str)] = &[
    ("gc.buffer", "4096"),
    ("graphics.shaderlod", "600"),
    ("render.distance", "1500"),
    ("fps.limit", "-1"),
];

#[tauri::command]
pub fn detect_rust_installation() -> Result<RustGameInfo, String> {
    // Common Rust install paths via Steam
    let common_paths = vec![
        r"C:\Program Files (x86)\Steam\steamapps\common\Rust",
        r"D:\SteamLibrary\steamapps\common\Rust",
        r"E:\SteamLibrary\steamapps\common\Rust",
        r"C:\SteamLibrary\steamapps\common\Rust",
    ];

    for path in &common_paths {
        let rust_path = PathBuf::from(path);
        let exe_path = rust_path.join("RustClient.exe");
        if exe_path.exists() {
            return Ok(RustGameInfo {
                installed: true,
                install_path: Some(path.to_string()),
                current_launch_options: get_steam_launch_options(),
            });
        }
    }

    Ok(RustGameInfo {
        installed: false,
        install_path: None,
        current_launch_options: None,
    })
}

#[tauri::command]
pub fn get_recommended_launch_options() -> Result<String, String> {
    Ok(LAUNCH_OPTIONS.to_string())
}

#[tauri::command]
pub fn get_recommended_console_commands() -> Result<Vec<(String, String)>, String> {
    Ok(CONSOLE_COMMANDS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect())
}

#[tauri::command]
pub fn apply_rust_tweaks(state: State<'_, LicenseState>) -> Result<Vec<RustTweakResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("rust_tweaks");
    let mut results = Vec::new();

    // 1. Set launch options recommendation
    results.push(RustTweakResult {
        name: "Launch Options".to_string(),
        success: true,
        message: format!("→ Set Steam launch options to: {}", LAUNCH_OPTIONS),
    });

    // 2. Disable fullscreen optimizations on RustClient.exe
    let rust_info = detect_rust_installation().unwrap_or(RustGameInfo {
        installed: false,
        install_path: None,
        current_launch_options: None,
    });

    if let Some(ref path) = rust_info.install_path {
        let exe_path = format!(r"{}\RustClient.exe", path);

        // Set compatibility flags
        use crate::utils::registry_helper;
        use winreg::enums::*;

        let compat_subkey = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
        match registry_helper::set_string(
            HKEY_CURRENT_USER,
            compat_subkey,
            &exe_path,
            "~ DISABLEDXMAXIMIZEDWINDOWEDMODE",
        ) {
            Ok(_) => results.push(RustTweakResult {
                name: "Disable Fullscreen Optimizations".to_string(),
                success: true,
                message: "✓ Fullscreen optimizations disabled for RustClient.exe".to_string(),
            }),
            Err(e) => results.push(RustTweakResult {
                name: "Disable Fullscreen Optimizations".to_string(),
                success: false,
                message: format!("✗ {}", e),
            }),
        }

        // 3. Set high priority via registry
        match registry_helper::set_string(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\RustClient.exe\PerfOptions",
            "CpuPriorityClass",
            "3", // High
        ) {
            Ok(_) => results.push(RustTweakResult {
                name: "CPU Priority → High".to_string(),
                success: true,
                message: "✓ RustClient.exe set to High priority".to_string(),
            }),
            Err(e) => results.push(RustTweakResult {
                name: "CPU Priority → High".to_string(),
                success: false,
                message: format!("✗ {} (needs admin)", e),
            }),
        }
    } else {
        results.push(RustTweakResult {
            name: "Rust Detection".to_string(),
            success: false,
            message: "→ Rust game not found. Install via Steam first.".to_string(),
        });
    }

    // 4. Console commands recommendations
    for (cmd, val) in CONSOLE_COMMANDS {
        results.push(RustTweakResult {
            name: format!("Console: {}", cmd),
            success: true,
            message: format!("→ Press F1 in-game and type: {} {}", cmd, val),
        });
    }

    Ok(results)
}

fn get_steam_launch_options() -> Option<String> {
    // Steam stores launch options in localconfig.vdf — complex parsing
    // For now, we just return the recommended options
    Some(LAUNCH_OPTIONS.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust_installation() {
        let info = detect_rust_installation();
        assert!(info.is_ok());
        // May or may not find Rust, but should not error
        println!("Rust installed: {}", info.unwrap().installed);
    }

    #[test]
    fn test_get_recommended_launch_options() {
        let opts = get_recommended_launch_options();
        assert!(opts.is_ok());
        assert!(opts.unwrap().contains("-high"));
    }

    #[test]
    fn test_get_console_commands() {
        let cmds = get_recommended_console_commands();
        assert!(cmds.is_ok());
        assert!(!cmds.unwrap().is_empty());
    }

    #[test]
    fn test_apply_rust_tweaks() {
        let results = apply_rust_tweaks();
        assert!(results.is_ok());
        let tweaks = results.unwrap();
        assert!(!tweaks.is_empty());
    }
}
