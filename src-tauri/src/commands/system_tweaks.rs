use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
use crate::utils::license_guard::{LicenseState, require_license};
use crate::utils::registry_helper;
use winreg::enums::*;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize)]
pub struct SystemTweakResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SystemTweakStatus {
    pub name: String,
    pub optimized: bool,
    pub current_value: String,
}

// ═══════════════════════════════════════════════════════════════
// MSI Mode for GPU
//
// Message Signaled Interrupts — faster than line-based interrupts.
// Reduces input lag by 2-5ms. Works on all modern GPUs.
// ═══════════════════════════════════════════════════════════════

fn is_display_device(inst_key: &winreg::RegKey) -> bool {
    // Primary check: Class == "Display"
    let class: String = inst_key.get_value("Class").unwrap_or_default();
    if class.eq_ignore_ascii_case("Display") {
        return true;
    }
    // Fallback: ClassGUID matches Display adapters GUID
    let class_guid: String = inst_key.get_value("ClassGUID").unwrap_or_default();
    if class_guid.eq_ignore_ascii_case("{4d36e968-e325-11ce-bfc1-08002be10318}") {
        return true;
    }
    // Fallback: DeviceDesc or HardwareID contains GPU keywords
    let desc: String = inst_key.get_value("DeviceDesc").unwrap_or_default();
    let desc_lower = desc.to_lowercase();
    desc_lower.contains("nvidia") || desc_lower.contains("geforce")
        || desc_lower.contains("radeon") || desc_lower.contains("amd")
        || desc_lower.contains("intel") && desc_lower.contains("graphics")
}

/// Get MSI mode status for GPU
#[tauri::command]
pub fn get_msi_mode_status() -> Result<Vec<SystemTweakStatus>, String> {
    let mut results = Vec::new();

    // Find GPU device in registry
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let enum_path = r"SYSTEM\CurrentControlSet\Enum\PCI";

    if let Ok(pci_key) = hklm.open_subkey(enum_path) {
        for device_id in pci_key.enum_keys().filter_map(|k| k.ok()) {
            let device_path = format!(r"{}\{}", enum_path, device_id);
            if let Ok(device_key) = hklm.open_subkey(&device_path) {
                for instance in device_key.enum_keys().filter_map(|k| k.ok()) {
                    let instance_path = format!(r"{}\{}", device_path, instance);
                    if let Ok(inst_key) = hklm.open_subkey(&instance_path) {
                        if is_display_device(&inst_key) {
                            // Check MSI mode
                            let msi_path = format!(r"{}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties", instance_path);
                            let msi_enabled = registry_helper::get_dword(
                                HKEY_LOCAL_MACHINE, &msi_path, "MSISupported"
                            ).unwrap_or(0);

                            let desc: String = inst_key.get_value("DeviceDesc").unwrap_or_default();
                            let name = desc.split(';').last().unwrap_or(&desc).trim().to_string();

                            results.push(SystemTweakStatus {
                                name: format!("MSI Mode: {}", if name.is_empty() { "GPU" } else { &name }),
                                optimized: msi_enabled == 1,
                                current_value: if msi_enabled == 1 { "Enabled".to_string() } else { "Disabled".to_string() },
                            });
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        results.push(SystemTweakStatus {
            name: "MSI Mode".to_string(),
            optimized: false,
            current_value: "GPU not found — run as Administrator".to_string(),
        });
    }

    Ok(results)
}

/// Enable MSI mode for all GPUs
#[tauri::command]
pub fn enable_msi_mode(state: State<'_, LicenseState>) -> Result<Vec<SystemTweakResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("msi_mode");
    let mut results = Vec::new();

    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let enum_path = r"SYSTEM\CurrentControlSet\Enum\PCI";

    if let Ok(pci_key) = hklm.open_subkey(enum_path) {
        for device_id in pci_key.enum_keys().filter_map(|k| k.ok()) {
            let device_path = format!(r"{}\{}", enum_path, device_id);
            if let Ok(device_key) = hklm.open_subkey(&device_path) {
                for instance in device_key.enum_keys().filter_map(|k| k.ok()) {
                    let instance_path = format!(r"{}\{}", device_path, instance);
                    if let Ok(inst_key) = hklm.open_subkey(&instance_path) {
                        if is_display_device(&inst_key) {
                            let desc: String = inst_key.get_value("DeviceDesc").unwrap_or_default();
                            let name = desc.split(';').last().unwrap_or(&desc).trim().to_string();
                            let display_name = if name.is_empty() { "GPU".to_string() } else { name };
                            let msi_path = format!(r"{}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties", instance_path);
                            match registry_helper::set_dword(HKEY_LOCAL_MACHINE, &msi_path, "MSISupported", 1) {
                                Ok(_) => results.push(SystemTweakResult {
                                    name: format!("MSI Mode: {}", display_name),
                                    success: true,
                                    message: format!("✓ MSI Mode enabled for {}. Reboot required.", display_name),
                                }),
                                Err(e) => results.push(SystemTweakResult {
                                    name: format!("MSI Mode: {}", display_name),
                                    success: false,
                                    message: format!("✗ Failed (needs admin): {}", e),
                                }),
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        results.push(SystemTweakResult {
            name: "MSI Mode".to_string(),
            success: false,
            message: "✗ No GPU found in registry".to_string(),
        });
    }

    Ok(results)
}

// ═══════════════════════════════════════════════════════════════
// SysMain / Superfetch
//
// Windows caches apps to RAM in background. During gaming this
// causes random disk I/O and stutters. Disabling it frees RAM
// and removes a source of micro-stutters.
// ═══════════════════════════════════════════════════════════════

/// Get SysMain service status
#[tauri::command]
pub fn get_sysmain_status() -> Result<SystemTweakStatus, String> {
    let output = Command::new("sc")
        .args(["query", "SysMain"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let is_running = stdout.contains("RUNNING");
    let is_stopped = stdout.contains("STOPPED");

    Ok(SystemTweakStatus {
        name: "SysMain (Superfetch)".to_string(),
        optimized: !is_running,
        current_value: if is_running { "Running".to_string() }
                       else if is_stopped { "Stopped".to_string() }
                       else { "Unknown".to_string() },
    })
}

/// Disable SysMain service
#[tauri::command]
pub fn disable_sysmain(state: State<'_, LicenseState>) -> Result<SystemTweakResult, String> {
    require_license(&state)?;

    // Stop and disable the service
    let stop = Command::new("sc").args(["stop", "SysMain"])
        .creation_flags(CREATE_NO_WINDOW).output();
    let disable = Command::new("sc").args(["config", "SysMain", "start=", "disabled"])
        .creation_flags(CREATE_NO_WINDOW).output();

    match disable {
        Ok(o) if o.status.success() => {
            let _ = stop;
            crate::utils::cleanup::register_tweak("sysmain");
            Ok(SystemTweakResult {
                name: "SysMain".to_string(),
                success: true,
                message: "✓ SysMain disabled. Less background disk I/O.".to_string(),
            })
        }
        _ => {
            let _ = stop;
            Ok(SystemTweakResult {
                name: "SysMain".to_string(),
                success: false,
                message: "✗ Failed (needs admin rights)".to_string(),
            })
        }
    }
}

/// Re-enable SysMain service
#[tauri::command]
pub fn enable_sysmain(state: State<'_, LicenseState>) -> Result<SystemTweakResult, String> {
    require_license(&state)?;

    let _ = Command::new("sc").args(["config", "SysMain", "start=", "auto"])
        .creation_flags(CREATE_NO_WINDOW).output();
    let _ = Command::new("sc").args(["start", "SysMain"])
        .creation_flags(CREATE_NO_WINDOW).output();

    Ok(SystemTweakResult {
        name: "SysMain".to_string(),
        success: true,
        message: "✓ SysMain re-enabled.".to_string(),
    })
}

// ═══════════════════════════════════════════════════════════════
// Windows Visual Effects
//
// Disabling animations, transparency, shadows, and smooth scrolling
// frees GPU and CPU resources. Noticeable on weaker hardware.
// ═══════════════════════════════════════════════════════════════

/// Get current visual effects status
#[tauri::command]
pub fn get_visual_effects_status() -> Result<SystemTweakStatus, String> {
    // Check if visual effects are set to "Best Performance"
    let val = registry_helper::get_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
        "VisualFXSetting"
    ).unwrap_or(0);

    Ok(SystemTweakStatus {
        name: "Visual Effects".to_string(),
        optimized: val == 2, // 2 = Best Performance
        current_value: match val {
            0 => "Let Windows decide".to_string(),
            1 => "Best Appearance".to_string(),
            2 => "Best Performance".to_string(),
            3 => "Custom".to_string(),
            _ => format!("Unknown ({})", val),
        },
    })
}

/// Set visual effects to "Best Performance"
#[tauri::command]
pub fn disable_visual_effects(state: State<'_, LicenseState>) -> Result<Vec<SystemTweakResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("visual_effects");
    let mut results = Vec::new();

    // Set to "Best Performance" mode
    results.push(apply_tweak("Visual Effects → Best Performance", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
            "VisualFXSetting", 2
        )
    }));

    // Disable transparency
    results.push(apply_tweak("Disable Transparency", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "EnableTransparency", 0
        )
    }));

    // Disable animations
    results.push(apply_tweak("Disable Window Animations", || {
        registry_helper::set_string(
            HKEY_CURRENT_USER,
            r"Control Panel\Desktop\WindowMetrics",
            "MinAnimate", "0"
        )
    }));

    // Disable smooth scrolling
    results.push(apply_tweak("Disable Smooth Scrolling", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"Control Panel\Desktop",
            "SmoothScroll", 0
        )
    }));

    // Disable cursor shadow
    results.push(apply_tweak("Disable Cursor Shadow", || {
        registry_helper::set_dword(
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
            "ListviewShadow", 0
        )
    }));

    Ok(results)
}

/// Restore visual effects to default
#[tauri::command]
pub fn restore_visual_effects(state: State<'_, LicenseState>) -> Result<SystemTweakResult, String> {
    require_license(&state)?;

    let _ = registry_helper::set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
        "VisualFXSetting", 0 // Let Windows decide
    );
    let _ = registry_helper::set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        "EnableTransparency", 1
    );

    Ok(SystemTweakResult {
        name: "Visual Effects".to_string(),
        success: true,
        message: "✓ Visual effects restored to default.".to_string(),
    })
}

fn apply_tweak<F>(name: &str, f: F) -> SystemTweakResult
where F: FnOnce() -> Result<(), String> {
    match f() {
        Ok(_) => SystemTweakResult {
            name: name.to_string(), success: true,
            message: format!("✓ {}", name),
        },
        Err(e) => SystemTweakResult {
            name: name.to_string(), success: false,
            message: format!("✗ {} — {}", name, e),
        },
    }
}
