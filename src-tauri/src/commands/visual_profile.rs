use serde::Serialize;
use tauri::State;
use std::fs;
use std::path::PathBuf;
use crate::utils::license_guard::{LicenseState, require_license};

#[derive(Debug, Serialize)]
pub struct VisualProfileResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

/// Apply one-click visual profile:
/// - NVIDIA: brightness 65%, contrast 70%, gamma 1.40
/// - Rust client.cfg: optimal player settings
#[tauri::command]
pub fn apply_visual_profile(state: State<'_, LicenseState>) -> Result<Vec<VisualProfileResult>, String> {
    require_license(&state)?;
    let mut results = Vec::new();

    results.push(apply_nvidia_color_profile());
    results.extend(apply_rust_cfg());

    Ok(results)
}

#[tauri::command]
pub fn get_visual_profile_status() -> Result<serde_json::Value, String> {
    let cfg_path = find_rust_cfg_path();
    Ok(serde_json::json!({
        "nvidia": {
            "brightness": 65,
            "contrast": 70,
            "gamma": 1.40
        },
        "rust_cfg_found": cfg_path.is_some(),
        "rust_cfg_path": cfg_path.map(|p| p.to_string_lossy().to_string())
    }))
}

fn apply_nvidia_color_profile() -> VisualProfileResult {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    use winreg::RegKey;

    // NVIDIA stores per-display color settings in the driver registry
    // We set them via the NVTweak path and also via SetDeviceGammaRamp
    let subkey = r"SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}\0000";

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Try to find the display adapter key
    let adapter_base = r"SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}";
    let mut applied = false;

    if let Ok(base_key) = hklm.open_subkey(adapter_base) {
        for idx in base_key.enum_keys().flatten() {
            let sub_path = format!(r"{}\{}", adapter_base, idx);
            if let Ok(sub) = hklm.open_subkey_with_flags(&sub_path, winreg::enums::KEY_READ) {
                if let Ok(desc) = sub.get_value::<String, _>("DriverDesc") {
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("nvidia") || desc_lower.contains("geforce") {
                        // Set color profile values
                        // Brightness: 0-100 → store as percentage
                        // Contrast: 0-100 → store as percentage
                        // Gamma: stored as value * 1000 (1400 = 1.40)
                        let _ = registry_helper::set_dword(
                            HKEY_LOCAL_MACHINE, &sub_path, "DefaultSettings.Brightness", 65
                        );
                        let _ = registry_helper::set_dword(
                            HKEY_LOCAL_MACHINE, &sub_path, "DefaultSettings.Contrast", 70
                        );
                        let _ = registry_helper::set_dword(
                            HKEY_LOCAL_MACHINE, &sub_path, "DefaultSettings.Gamma", 1400
                        );
                        applied = true;
                        break;
                    }
                }
            }
        }
    }

    // Also apply gamma via Windows GDI SetDeviceGammaRamp
    let gamma_ok = apply_gamma_ramp(65, 70, 1.40);

    if applied || gamma_ok {
        VisualProfileResult {
            name: "NVIDIA Color Profile".to_string(),
            success: true,
            message: "✓ Brightness 65% · Contrast 70% · Gamma 1.40".to_string(),
        }
    } else {
        VisualProfileResult {
            name: "NVIDIA Color Profile".to_string(),
            success: false,
            message: "✗ Could not apply color profile — reopen NVIDIA Control Panel".to_string(),
        }
    }
}

fn apply_gamma_ramp(brightness: u8, contrast: u8, gamma: f64) -> bool {
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
    use windows::Win32::Foundation::HWND;

    // SetDeviceGammaRamp via raw extern (not exposed in windows crate v0.58 directly)
    #[link(name = "gdi32")]
    extern "system" {
        fn SetDeviceGammaRamp(hdc: *mut std::ffi::c_void, lpramp: *const u16) -> i32;
    }

    unsafe {
        let hdc = GetDC(HWND(std::ptr::null_mut()));
        if hdc.is_invalid() { return false; }

        let mut ramp: [u16; 768] = [0u16; 768];
        let b = brightness as f64 / 100.0;
        let c = contrast as f64 / 100.0;

        for i in 0usize..256 {
            let normalized = i as f64 / 255.0;
            let g = normalized.powf(1.0 / gamma);
            let val = ((g * c + (b - 0.5) * (1.0 - c) + 0.5) * 65535.0)
                .max(0.0)
                .min(65535.0) as u16;
            ramp[i] = val;
            ramp[i + 256] = val;
            ramp[i + 512] = val;
        }

        let ok = SetDeviceGammaRamp(hdc.0 as *mut _, ramp.as_ptr()) != 0;
        ReleaseDC(HWND(std::ptr::null_mut()), hdc);
        ok
    }
}

fn find_rust_cfg_path() -> Option<PathBuf> {
    // Rust stores cfg in %APPDATA%\Roaming\Rust\cfg\ or via Steam userdata
    let appdata = std::env::var("APPDATA").ok()?;
    let cfg_path = PathBuf::from(&appdata).join("Rust").join("cfg");
    if cfg_path.exists() {
        return Some(cfg_path.join("client.cfg"));
    }

    // Try Steam userdata paths
    let steam_paths = vec![
        r"C:\Program Files (x86)\Steam\userdata",
        r"D:\Steam\userdata",
        r"C:\Steam\userdata",
    ];
    for steam_base in &steam_paths {
        let base = PathBuf::from(steam_base);
        if base.exists() {
            // Walk userdata/*/252490/local/cfg/
            if let Ok(entries) = fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let cfg = entry.path().join("252490").join("local").join("cfg").join("client.cfg");
                    if cfg.exists() {
                        return Some(cfg);
                    }
                }
            }
        }
    }
    None
}

fn apply_rust_cfg() -> Vec<VisualProfileResult> {
    let mut results = Vec::new();

    let cfg_settings = vec![
        ("graphics.brightness", "0.65"),
        ("graphics.contrast", "1.10"),
        ("graphics.gamma", "1.40"),
        ("graphics.shafts", "0"),
        ("graphics.bloom", "0"),
        ("graphics.motionblur", "false"),
        ("graphics.dof", "0"),
        ("graphics.vignetteenabled", "false"),
        ("graphics.tonemapping", "0"),
        ("headlerp", "0"),
    ];

    match find_rust_cfg_path() {
        Some(cfg_path) => {
            // Read existing config
            let existing = fs::read_to_string(&cfg_path).unwrap_or_default();
            let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();

            for (key, value) in &cfg_settings {
                let new_line = format!("{} \"{}\"", key, value);
                // Replace or append
                let mut found = false;
                for line in lines.iter_mut() {
                    if line.to_lowercase().starts_with(&key.to_lowercase()) {
                        *line = new_line.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    lines.push(new_line);
                }
            }

            match fs::write(&cfg_path, lines.join("\n")) {
                Ok(_) => results.push(VisualProfileResult {
                    name: "Rust client.cfg".to_string(),
                    success: true,
                    message: format!("✓ Applied {} visual settings to client.cfg", cfg_settings.len()),
                }),
                Err(e) => results.push(VisualProfileResult {
                    name: "Rust client.cfg".to_string(),
                    success: false,
                    message: format!("✗ Could not write cfg: {}", e),
                }),
            }
        }
        None => {
            results.push(VisualProfileResult {
                name: "Rust client.cfg".to_string(),
                success: false,
                message: "✗ Rust not found — launch Rust once to create cfg folder".to_string(),
            });
        }
    }

    results
}
