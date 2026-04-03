use serde::Serialize;
use tauri::State;
use crate::utils::registry_helper;
use crate::utils::license_guard::{LicenseState, require_license};
use winreg::enums::*;
use winreg::HKEY;

#[derive(Debug, Serialize)]
pub struct RegistryTweakResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub fn get_registry_status() -> Result<Vec<RegistryTweakStatus>, String> {
    let tweaks = vec![
        check_tweak("Game DVR Disabled", HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 0),
        check_tweak("Game Bar Disabled", HKEY_CURRENT_USER,
            r"SOFTWARE\Microsoft\GameBar", "AllowAutoGameMode", 1),
        check_tweak("Fullscreen Optimizations", HKEY_CURRENT_USER,
            r"System\GameConfigStore", "GameDVR_FSEBehavior", 2),
        check_tweak("Mouse Acceleration Off", HKEY_CURRENT_USER,
            r"Control Panel\Mouse", "MouseSpeed", 0),
        check_tweak("Network Throttling Disabled", HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
            "NetworkThrottlingIndex", 0xFFFFFFFF),
        check_tweak("GPU Scheduling", HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            "HwSchMode", 2),
    ];
    Ok(tweaks)
}

#[derive(Debug, Serialize)]
pub struct RegistryTweakStatus {
    pub name: String,
    pub applied: bool,
    pub current_value: String,
}

fn check_tweak(name: &str, hkey: HKEY, subkey: &str, value_name: &str, expected: u32) -> RegistryTweakStatus {
    match registry_helper::get_dword(hkey, subkey, value_name) {
        Ok(val) => RegistryTweakStatus {
            name: name.to_string(),
            applied: val == expected,
            current_value: format!("{}", val),
        },
        Err(_) => RegistryTweakStatus {
            name: name.to_string(),
            applied: false,
            current_value: "Not set".to_string(),
        },
    }
}

#[tauri::command]
pub fn apply_registry_tweaks(state: State<'_, LicenseState>) -> Result<Vec<RegistryTweakResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("registry_tweaks");
    let mut results = Vec::new();

    // 1. Disable Game DVR
    results.push(apply_tweak("Disable Game DVR", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
            "AppCaptureEnabled", 0
        )
    }));

    // 2. Disable Game Bar auto game mode issues
    results.push(apply_tweak("Enable Game Mode", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"SOFTWARE\Microsoft\GameBar",
            "AllowAutoGameMode", 1
        )
    }));

    // 3. Disable Fullscreen Optimizations globally
    results.push(apply_tweak("Disable Fullscreen Optimizations", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"System\GameConfigStore",
            "GameDVR_FSEBehavior", 2
        )
    }));

    // 4. Disable Mouse Acceleration
    results.push(apply_tweak("Disable Mouse Acceleration", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"Control Panel\Mouse",
            "MouseSpeed", 0
        )
    }));

    // 5. Disable Network Throttling
    results.push(apply_tweak("Disable Network Throttling", || {
        registry_helper::set_dword(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
            "NetworkThrottlingIndex", 0xFFFFFFFF
        )
    }));

    // 6. Multimedia scheduling priority for games
    results.push(apply_tweak("GPU Priority for Games", || {
        registry_helper::set_dword(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
            "GPU Priority", 8
        )?;
        registry_helper::set_dword(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
            "Priority", 6
        )?;
        registry_helper::set_string(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
            "Scheduling Category", "High"
        )
    }));

    // 7. Enable Hardware GPU Scheduling
    // Only apply on supported GPUs (requires WDDM 2.7+ driver, Win10 2004+)
    // Skip on GTX 900/1000 series and older to prevent grey screen issues
    results.push(apply_tweak("Enable HW GPU Scheduling", || {
        // Check Windows build — requires 19041+ (Win10 2004)
        let win_build = registry_helper::get_dword(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "CurrentBuildNumber",
        ).unwrap_or(0);

        // Read as string since CurrentBuildNumber is REG_SZ
        let build_str: String = {
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            hklm.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
                .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber"))
                .unwrap_or_default()
        };
        let build: u32 = build_str.trim().parse().unwrap_or(win_build);

        if build < 19041 {
            return Err("HW GPU Scheduling requires Windows 10 version 2004 or newer".to_string());
        }

        // Check GPU driver version via registry — skip Maxwell/Pascal (GTX 900/1000)
        // These use WDDM < 2.7 and can grey-screen with HwSchMode=2
        let gpu_ok = {
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let enum_path = r"SYSTEM\CurrentControlSet\Enum\PCI";
            let mut supported = true;

            if let Ok(pci) = hklm.open_subkey(enum_path) {
                'outer: for dev in pci.enum_keys().filter_map(|k| k.ok()) {
                    let dev_path = format!(r"{}\{}", enum_path, dev);
                    if let Ok(dk) = hklm.open_subkey(&dev_path) {
                        for inst in dk.enum_keys().filter_map(|k| k.ok()) {
                            let inst_path = format!(r"{}\{}", dev_path, inst);
                            if let Ok(ik) = hklm.open_subkey(&inst_path) {
                                let class: String = ik.get_value("Class").unwrap_or_default();
                                if !class.eq_ignore_ascii_case("Display") { continue; }
                                let desc: String = ik.get_value("DeviceDesc").unwrap_or_default();
                                let desc_lower = desc.to_lowercase();
                                // GTX 900 (Maxwell) and GTX 1000 (Pascal) don't support HwSch
                                if (desc_lower.contains("gtx 9") || desc_lower.contains("gtx 10"))
                                    && desc_lower.contains("nvidia") {
                                    supported = false;
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
            supported
        };

        if !gpu_ok {
            return Err("HW GPU Scheduling skipped: not supported on GTX 900/1000 series".to_string());
        }

        registry_helper::set_dword(
            HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            "HwSchMode", 2
        )
    }));

    Ok(results)
}

fn apply_tweak<F>(name: &str, f: F) -> RegistryTweakResult
where
    F: FnOnce() -> Result<(), String>,
{
    match f() {
        Ok(_) => RegistryTweakResult {
            name: name.to_string(),
            success: true,
            message: format!("✓ {} applied", name),
        },
        Err(e) => RegistryTweakResult {
            name: name.to_string(),
            success: false,
            message: format!("✗ {} failed: {}", name, e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_registry_status() {
        let status = get_registry_status();
        assert!(status.is_ok());
        let items = status.unwrap();
        assert!(items.len() >= 5);
        for item in &items {
            assert!(!item.name.is_empty());
        }
    }

    #[test]
    fn test_check_tweak_nonexistent() {
        let result = check_tweak(
            "Test",
            HKEY_CURRENT_USER,
            r"Software\RustOpti\NonExistent",
            "TestVal",
            0,
        );
        assert!(!result.applied);
        assert_eq!(result.current_value, "Not set");
    }
}
