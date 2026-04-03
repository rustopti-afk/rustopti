use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
use crate::utils::license_guard::{LicenseState, require_license};
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize)]
pub struct PowerResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PowerPlanInfo {
    pub name: String,
    pub guid: String,
    pub active: bool,
}

#[tauri::command]
pub fn get_power_plans() -> Result<Vec<PowerPlanInfo>, String> {
    // Use PowerShell wrapper to force UTF-8 output and avoid OEM codepage encoding issues
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; powercfg /list",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run powercfg: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut plans = Vec::new();

    for line in stdout.lines() {
        if line.contains("GUID") {
            // Format: Power Scheme GUID: GUID  (Name) *
            let active = line.contains('*');
            if let Some(guid_start) = line.find("GUID: ") {
                let guid_part = &line[guid_start + 6..];
                if let Some(guid_end) = guid_part.find(' ') {
                    let guid = guid_part[..guid_end].trim().to_string();
                    let name = if let (Some(start), Some(end)) = (guid_part.find('('), guid_part.find(')')) {
                        guid_part[start + 1..end].to_string()
                    } else {
                        "Unknown".to_string()
                    };
                    plans.push(PowerPlanInfo { name, guid, active });
                }
            }
        }
    }

    Ok(plans)
}

#[tauri::command]
pub fn apply_power_tweaks(state: State<'_, LicenseState>) -> Result<Vec<PowerResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("power_tweaks");
    let mut results = Vec::new();

    // 1. Create or activate Ultimate Performance plan
    results.push(create_ultimate_performance());

    // 2. Disable CPU core parking via powercfg
    results.push(apply_power_cmd(
        "Disable CPU Core Parking",
        &["powercfg", "/setacvalueindex", "scheme_current", "sub_processor", "CPMINCORES", "100"],
    ));

    // 3. Set minimum processor state to 100%
    results.push(apply_power_cmd(
        "CPU Min State → 100%",
        &["powercfg", "/setacvalueindex", "scheme_current", "sub_processor", "PROCTHROTTLEMIN", "100"],
    ));

    // 4. Disable USB selective suspend
    results.push(apply_power_cmd(
        "Disable USB Suspend",
        &["powercfg", "/setacvalueindex", "scheme_current", "2a737441-1930-4402-8d77-b2bebba308a3", "48e6b7a6-50f5-4782-a5d4-53bb8f07e226", "0"],
    ));

    // 5. Apply current scheme
    let apply_output = Command::new("powercfg")
        .args(["/setactive", "scheme_current"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match apply_output {
        Ok(_) => results.push(PowerResult {
            name: "Apply Power Scheme".to_string(),
            success: true,
            message: "✓ Power scheme activated".to_string(),
        }),
        Err(e) => results.push(PowerResult {
            name: "Apply Power Scheme".to_string(),
            success: false,
            message: format!("✗ Failed: {}", e),
        }),
    }

    Ok(results)
}

fn create_ultimate_performance() -> PowerResult {
    // First check if Ultimate Performance plan already exists to avoid duplicates
    let list_output = Command::new("powercfg")
        .args(["/list"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    if let Ok(list) = list_output {
        let list_str = String::from_utf8_lossy(&list.stdout).to_lowercase();
        // Find any existing GUID that came from duplicating the Ultimate Performance scheme
        // The name varies by locale but the source GUID is always e9a42b02...
        // We detect it by looking for plans named after Ultimate Performance in any language
        for line in list_str.lines() {
            if line.contains("e9a42b02") {
                // Already exists — just activate it
                if let Some(guid_start) = line.find("guid: ") {
                    let guid_part = &line[guid_start + 6..];
                    if let Some(guid_end) = guid_part.find(' ') {
                        let guid = guid_part[..guid_end].trim().to_string();
                        let _ = Command::new("powercfg")
                            .args(["/setactive", &guid])
                            .creation_flags(CREATE_NO_WINDOW)
                            .output();
                        return PowerResult {
                            name: "Ultimate Performance Plan".to_string(),
                            success: true,
                            message: "✓ Ultimate Performance plan activated".to_string(),
                        };
                    }
                }
            }
        }
        // Also check by localized name containing "ultimate" keyword
        for line in list_str.lines() {
            if (line.contains("ultimate") || line.contains("максимальна")) && line.contains("guid:") {
                if let Some(guid_start) = line.find("guid: ") {
                    let guid_part = &line[guid_start + 6..];
                    if let Some(guid_end) = guid_part.find(' ') {
                        let guid = guid_part[..guid_end].trim().to_string();
                        let _ = Command::new("powercfg")
                            .args(["/setactive", &guid])
                            .creation_flags(CREATE_NO_WINDOW)
                            .output();
                        return PowerResult {
                            name: "Ultimate Performance Plan".to_string(),
                            success: true,
                            message: "✓ Ultimate Performance plan activated".to_string(),
                        };
                    }
                }
            }
        }
    }

    // Plan not found — create it once
    let output = Command::new("powercfg")
        .args(["/duplicatescheme", "e9a42b02-d5df-448d-aa00-03f14749eb61"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Extract the new GUID and activate it
            if let Some(guid_start) = stdout.to_lowercase().find("guid: ") {
                let guid_part = &stdout[guid_start + 6..];
                if let Some(guid_end) = guid_part.find(|c: char| c.is_whitespace()) {
                    let guid = guid_part[..guid_end].trim().to_string();
                    let _ = Command::new("powercfg")
                        .args(["/setactive", &guid])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                }
            }
            if o.status.success() || stdout.contains("GUID") || stdout.contains("Guid") {
                PowerResult {
                    name: "Ultimate Performance Plan".to_string(),
                    success: true,
                    message: "✓ Ultimate Performance plan created and activated".to_string(),
                }
            } else {
                // Fallback: set High Performance
                let _ = Command::new("powercfg")
                    .args(["/setactive", "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
                PowerResult {
                    name: "High Performance Plan".to_string(),
                    success: true,
                    message: "✓ High Performance plan activated (Ultimate not available)".to_string(),
                }
            }
        }
        Err(e) => PowerResult {
            name: "Ultimate Performance Plan".to_string(),
            success: false,
            message: format!("✗ Failed: {}", e),
        },
    }
}

fn apply_power_cmd(name: &str, args: &[&str]) -> PowerResult {
    match Command::new(args[0]).args(&args[1..]).creation_flags(CREATE_NO_WINDOW).output() {
        Ok(o) if o.status.success() => PowerResult {
            name: name.to_string(),
            success: true,
            message: format!("✓ {}", name),
        },
        Ok(o) => PowerResult {
            name: name.to_string(),
            success: false,
            message: format!("✗ {} — exit code: {:?}", name, o.status.code()),
        },
        Err(e) => PowerResult {
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
    fn test_get_power_plans() {
        let plans = get_power_plans();
        assert!(plans.is_ok(), "Failed: {:?}", plans.err());
        let list = plans.unwrap();
        assert!(!list.is_empty(), "No power plans found");
        // At least one should be active
        assert!(list.iter().any(|p| p.active), "No active plan found");
    }
}
