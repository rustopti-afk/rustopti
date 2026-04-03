use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
use crate::utils::registry_helper;
use crate::utils::license_guard::{LicenseState, require_license};
use winreg::enums::*;

const CREATE_NO_WINDOW: u32 = 0x08000000;

// ═══════════════════════════════════════════════════════════════
// Data Structures
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct CoreParkingStatus {
    pub min_cores_percent: u32,
    pub cores_parked: bool,
    pub total_cores: usize,
}

#[derive(Debug, Serialize)]
pub struct CoreParkResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════
// Tauri Commands
// ═══════════════════════════════════════════════════════════════

/// Check current CPU core parking status via powercfg
#[tauri::command]
pub fn get_core_parking_status() -> Result<CoreParkingStatus, String> {
    let total_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Query current CPMINCORES value via powercfg
    let output = Command::new("powercfg")
        .args(["/qh", "SCHEME_CURRENT", "SUB_PROCESSOR", "CPMINCORES"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run powercfg: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut min_cores: u32 = 0;

    // Parse "Current AC Power Setting Index: 0x000000XX"
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Current AC Power Setting Index:") {
            if let Some(hex_pos) = trimmed.rfind("0x") {
                let hex_str = trimmed[hex_pos + 2..].trim();
                if let Ok(val) = u32::from_str_radix(hex_str, 16) {
                    min_cores = val;
                }
            }
        }
    }

    let cores_parked = min_cores < 100;

    Ok(CoreParkingStatus {
        min_cores_percent: min_cores,
        cores_parked,
        total_cores,
    })
}

/// Unpark all CPU cores — forces 100% cores active
#[tauri::command]
pub fn unpark_all_cores(state: State<'_, LicenseState>) -> Result<Vec<CoreParkResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("core_unpark");
    let mut results = Vec::new();

    // 1. Set minimum cores to 100% (AC — plugged in)
    results.push(run_cmd(
        "Min Cores → 100% (AC)",
        &[
            "powercfg",
            "/setacvalueindex",
            "scheme_current",
            "sub_processor",
            "CPMINCORES",
            "100",
        ],
    ));

    // 2. Set minimum cores to 100% (DC — battery)
    results.push(run_cmd(
        "Min Cores → 100% (DC)",
        &[
            "powercfg",
            "/setdcvalueindex",
            "scheme_current",
            "sub_processor",
            "CPMINCORES",
            "100",
        ],
    ));

    // 3. Max cores = 100%
    results.push(run_cmd(
        "Max Cores → 100% (AC)",
        &[
            "powercfg",
            "/setacvalueindex",
            "scheme_current",
            "sub_processor",
            "CPMAXCORES",
            "100",
        ],
    ));

    // 4. Disable processor idle (keep all cores awake)
    results.push(run_cmd(
        "Disable Processor Idle",
        &[
            "powercfg",
            "/setacvalueindex",
            "scheme_current",
            "sub_processor",
            "IDLEDISABLE",
            "1",
        ],
    ));

    // 5. Apply the scheme
    results.push(run_cmd(
        "Apply Power Scheme",
        &["powercfg", "/setactive", "scheme_current"],
    ));

    // 6. Registry persistence for min cores
    match registry_helper::set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Power\PowerSettings\54533251-82be-4824-96c1-47b60b740d00\0cc5b647-c1df-4637-891a-dec35c318583",
        "ValueMin",
        100,
    ) {
        Ok(_) => results.push(CoreParkResult {
            name: "Registry Persistence".to_string(),
            success: true,
            message: "✓ Core unparking saved to registry".to_string(),
        }),
        Err(e) => results.push(CoreParkResult {
            name: "Registry Persistence".to_string(),
            success: false,
            message: format!("✗ Registry failed (needs admin): {}", e),
        }),
    }

    Ok(results)
}

/// Restore default core parking (50% min cores)
#[tauri::command]
pub fn repark_cores(state: State<'_, LicenseState>) -> Result<Vec<CoreParkResult>, String> {
    require_license(&state)?;
    let mut results = Vec::new();

    results.push(run_cmd(
        "Restore Min Cores → 50% (AC)",
        &[
            "powercfg",
            "/setacvalueindex",
            "scheme_current",
            "sub_processor",
            "CPMINCORES",
            "50",
        ],
    ));

    results.push(run_cmd(
        "Restore Min Cores → 50% (DC)",
        &[
            "powercfg",
            "/setdcvalueindex",
            "scheme_current",
            "sub_processor",
            "CPMINCORES",
            "50",
        ],
    ));

    results.push(run_cmd(
        "Enable Processor Idle",
        &[
            "powercfg",
            "/setacvalueindex",
            "scheme_current",
            "sub_processor",
            "IDLEDISABLE",
            "0",
        ],
    ));

    results.push(run_cmd(
        "Apply Power Scheme",
        &["powercfg", "/setactive", "scheme_current"],
    ));

    Ok(results)
}

// ═══════════════════════════════════════════════════════════════
// Internal Helpers
// ═══════════════════════════════════════════════════════════════

fn run_cmd(name: &str, args: &[&str]) -> CoreParkResult {
    match Command::new(args[0])
        .args(&args[1..])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) if o.status.success() => CoreParkResult {
            name: name.to_string(),
            success: true,
            message: format!("✓ {}", name),
        },
        Ok(o) => CoreParkResult {
            name: name.to_string(),
            success: false,
            message: format!("✗ {} — exit code: {:?}", name, o.status.code()),
        },
        Err(e) => CoreParkResult {
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
    fn test_get_core_parking_status() {
        let status = get_core_parking_status();
        assert!(status.is_ok());
        let data = status.unwrap();
        assert!(data.total_cores > 0);
    }
}
