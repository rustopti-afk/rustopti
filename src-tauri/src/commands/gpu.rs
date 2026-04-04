use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
use crate::utils::license_guard::{LicenseState, require_license};
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize)]
pub struct GpuTweakResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GpuStatus {
    pub vendor: String,
    pub tweaks: Vec<GpuTweakInfo>,
}

#[derive(Debug, Serialize)]
pub struct GpuTweakInfo {
    pub name: String,
    pub description: String,
    pub status: String,
}

#[tauri::command]
pub fn detect_gpu_vendor() -> Result<String, String> {
    // Try registry first
    if let Ok(vendor) = detect_vendor_from_registry() {
        return Ok(vendor);
    }
    // Fallback: wmic
    detect_vendor_from_wmic()
}

fn detect_vendor_from_registry() -> Result<String, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let video_key = hklm
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Video")
        .map_err(|e| e.to_string())?;

    for guid in video_key.enum_keys() {
        if let Ok(guid) = guid {
            let subkey = format!(r"SYSTEM\CurrentControlSet\Control\Video\{}\0000", guid);
            if let Ok(device_key) = hklm.open_subkey(&subkey) {
                if let Ok(desc) = device_key.get_value::<String, _>("DriverDesc") {
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("nvidia") || desc_lower.contains("geforce") {
                        return Ok("NVIDIA".to_string());
                    }
                    if desc_lower.contains("amd") || desc_lower.contains("radeon") {
                        return Ok("AMD".to_string());
                    }
                    if desc_lower.contains("intel") {
                        return Ok("Intel".to_string());
                    }
                }
            }
        }
    }
    Err("Not found in registry".to_string())
}

fn detect_vendor_from_wmic() -> Result<String, String> {
    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name", "/format:value"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    for line in stdout.lines() {
        if let Some(name) = line.strip_prefix("name=") {
            if name.contains("nvidia") || name.contains("geforce") {
                return Ok("NVIDIA".to_string());
            }
            if name.contains("amd") || name.contains("radeon") {
                return Ok("AMD".to_string());
            }
            if name.contains("intel") {
                return Ok("Intel".to_string());
            }
        }
    }
    Ok("Unknown".to_string())
}

#[tauri::command]
pub fn apply_gpu_tweaks(state: State<'_, LicenseState>) -> Result<Vec<GpuTweakResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("gpu_tweaks");
    let vendor = detect_gpu_vendor().unwrap_or("Unknown".to_string());
    let mut results = Vec::new();

    match vendor.as_str() {
        "NVIDIA" => {
            results.push(apply_nvidia_tweak("Power Management → Max Performance",
                "nvidia-smi", &["-pm", "1"]));

            results.push(apply_nvidia_registry_tweak(
                "Shader Cache Size → 10GB",
                r"SOFTWARE\NVIDIA Corporation\Global\NVTweak",
                "ShaderCacheSize", 10240,
            ));

            results.push(apply_nvidia_registry_tweak(
                "Low Latency Mode → On",
                r"SOFTWARE\NVIDIA Corporation\Global\NVTweak",
                "LowLatencyMode", 1,
            ));

            results.push(apply_nvidia_registry_tweak(
                "Threaded Optimization → On",
                r"SOFTWARE\NVIDIA Corporation\Global\NVTweak",
                "ThreadedOptimization", 1,
            ));

            results.push(apply_nvidia_registry_tweak(
                "Pre-rendered Frames → 1",
                r"SOFTWARE\NVIDIA Corporation\Global\NVTweak",
                "PreRenderedFrames", 1,
            ));

            // NVIDIA Image Scaling (NIS) — driver-level upscaling for all games
            results.push(apply_nvidia_registry_tweak(
                "NVIDIA Image Scaling (NIS) → Enabled",
                r"SOFTWARE\NVIDIA Corporation\Global\NVTweak",
                "NvidiaImageScalingEnable", 1,
            ));
            // NIS sharpness: 50% (range 0–100, stored as 0–100)
            results.push(apply_nvidia_registry_tweak(
                "NIS Sharpness → 50%",
                r"SOFTWARE\NVIDIA Corporation\Global\NVTweak",
                "NvidiaImageScalingSharpness", 50,
            ));
        }
        "AMD" => {
            results.push(GpuTweakResult {
                name: "AMD Anti-Lag".to_string(),
                success: true,
                message: "→ Enable AMD Anti-Lag in Radeon Software".to_string(),
            });
            results.push(GpuTweakResult {
                name: "AMD Chill".to_string(),
                success: true,
                message: "→ Disable AMD Chill for competitive gaming".to_string(),
            });
            results.push(GpuTweakResult {
                name: "AMD Power Mode".to_string(),
                success: true,
                message: "→ Set GPU Workload → Graphics mode".to_string(),
            });

            // AMD Radeon Super Resolution (RSR) — driver-level upscaling for all games
            results.push(apply_amd_rsr_tweak());
        }
        _ => {
            results.push(GpuTweakResult {
                name: "GPU Detection".to_string(),
                success: false,
                message: "✗ Could not detect GPU vendor".to_string(),
            });
        }
    }

    Ok(results)
}

fn apply_amd_rsr_tweak() -> GpuTweakResult {
    use crate::utils::registry_helper;
    use winreg::enums::*;

    // AMD RSR stored under HKCU in AMD's driver key
    let subkey = r"SOFTWARE\AMD\CN";
    let results = [
        ("RSREnabled", 1u32),
        ("RSRSharpness", 50u32), // 0–100
    ];

    let mut all_ok = true;
    for (name, val) in &results {
        if registry_helper::set_dword(HKEY_CURRENT_USER, subkey, name, *val).is_err() {
            all_ok = false;
        }
    }

    if all_ok {
        GpuTweakResult {
            name: "AMD RSR (Radeon Super Resolution) → Enabled".to_string(),
            success: true,
            message: "✓ RSR ввімкнено, різкість 50%".to_string(),
        }
    } else {
        GpuTweakResult {
            name: "AMD RSR → Failed".to_string(),
            success: false,
            message: "✗ Не вдалось записати RSR registry. Можливо AMD Software не встановлено".to_string(),
        }
    }
}

#[tauri::command]
pub fn get_upscaling_status() -> Result<serde_json::Value, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let vendor = detect_gpu_vendor().unwrap_or("Unknown".to_string());

    match vendor.as_str() {
        "NVIDIA" => {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let key = hklm.open_subkey(r"SOFTWARE\NVIDIA Corporation\Global\NVTweak");
            let enabled: u32 = key.as_ref().ok()
                .and_then(|k| k.get_value("NvidiaImageScalingEnable").ok())
                .unwrap_or(0);
            let sharpness: u32 = key.as_ref().ok()
                .and_then(|k| k.get_value("NvidiaImageScalingSharpness").ok())
                .unwrap_or(50);
            Ok(serde_json::json!({
                "vendor": "NVIDIA",
                "technology": "NIS (NVIDIA Image Scaling)",
                "enabled": enabled == 1,
                "sharpness": sharpness,
            }))
        }
        "AMD" => {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let key = hkcu.open_subkey(r"SOFTWARE\AMD\CN");
            let enabled: u32 = key.as_ref().ok()
                .and_then(|k| k.get_value("RSREnabled").ok())
                .unwrap_or(0);
            let sharpness: u32 = key.as_ref().ok()
                .and_then(|k| k.get_value("RSRSharpness").ok())
                .unwrap_or(50);
            Ok(serde_json::json!({
                "vendor": "AMD",
                "technology": "RSR (Radeon Super Resolution)",
                "enabled": enabled == 1,
                "sharpness": sharpness,
            }))
        }
        _ => Ok(serde_json::json!({
            "vendor": vendor,
            "technology": "Not available",
            "enabled": false,
            "sharpness": 0,
        }))
    }
}

#[tauri::command]
pub fn set_upscaling(enabled: bool, sharpness: u32, state: State<'_, LicenseState>) -> Result<String, String> {
    require_license(&state)?;
    use crate::utils::registry_helper;
    use winreg::enums::*;

    let vendor = detect_gpu_vendor().unwrap_or("Unknown".to_string());
    let sharpness = sharpness.clamp(0, 100);

    match vendor.as_str() {
        "NVIDIA" => {
            let subkey = r"SOFTWARE\NVIDIA Corporation\Global\NVTweak";
            registry_helper::set_dword(HKEY_LOCAL_MACHINE, subkey, "NvidiaImageScalingEnable", enabled as u32)
                .map_err(|e| e.to_string())?;
            registry_helper::set_dword(HKEY_LOCAL_MACHINE, subkey, "NvidiaImageScalingSharpness", sharpness)
                .map_err(|e| e.to_string())?;
            Ok(format!("NIS {} (різкість {}%)", if enabled { "ввімкнено" } else { "вимкнено" }, sharpness))
        }
        "AMD" => {
            let subkey = r"SOFTWARE\AMD\CN";
            registry_helper::set_dword(HKEY_CURRENT_USER, subkey, "RSREnabled", enabled as u32)
                .map_err(|e| e.to_string())?;
            registry_helper::set_dword(HKEY_CURRENT_USER, subkey, "RSRSharpness", sharpness)
                .map_err(|e| e.to_string())?;
            Ok(format!("RSR {} (різкість {}%)", if enabled { "ввімкнено" } else { "вимкнено" }, sharpness))
        }
        _ => Err("GPU не підтримується (тільки NVIDIA та AMD)".to_string())
    }
}

fn apply_nvidia_tweak(name: &str, cmd: &str, args: &[&str]) -> GpuTweakResult {
    match Command::new(cmd).args(args).creation_flags(CREATE_NO_WINDOW).output() {
        Ok(output) => {
            if output.status.success() {
                GpuTweakResult {
                    name: name.to_string(),
                    success: true,
                    message: format!("✓ {}", name),
                }
            } else {
                GpuTweakResult {
                    name: name.to_string(),
                    success: false,
                    message: format!("✗ {} (command failed)", name),
                }
            }
        }
        Err(e) => GpuTweakResult {
            name: name.to_string(),
            success: false,
            message: format!("✗ {} — {}", name, e),
        },
    }
}

fn apply_nvidia_registry_tweak(name: &str, subkey: &str, value: &str, data: u32) -> GpuTweakResult {
    use crate::utils::registry_helper;
    use winreg::enums::*;

    match registry_helper::set_dword(HKEY_LOCAL_MACHINE, subkey, value, data) {
        Ok(_) => GpuTweakResult {
            name: name.to_string(),
            success: true,
            message: format!("✓ {}", name),
        },
        Err(e) => GpuTweakResult {
            name: name.to_string(),
            success: false,
            message: format!("✗ {} — {}", name, e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gpu_vendor() {
        let result = detect_gpu_vendor();
        assert!(result.is_ok());
        let vendor = result.unwrap();
        println!("Detected GPU: {}", vendor);
        assert!(["NVIDIA", "AMD", "Intel", "Unknown"].contains(&vendor.as_str()));
    }

    #[test]
    fn test_apply_gpu_tweaks_returns_results() {
        let results = apply_gpu_tweaks();
        assert!(results.is_ok());
        let tweaks = results.unwrap();
        assert!(!tweaks.is_empty());
        for tweak in &tweaks {
            assert!(!tweak.name.is_empty());
            assert!(!tweak.message.is_empty());
        }
    }
}
