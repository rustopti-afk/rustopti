use std::process::Command;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

use sha2::{Sha256, Digest};
use hex;

// Salt to prevent rainbow-table attacks on HWID hashes
const HWID_SALT: &str = "RustOpti-HWID-v2";

#[tauri::command]
pub fn get_hwid() -> String {
    // Use PowerShell (wmic is deprecated in Windows 11+)
    let cpuid = get_ps_value("(Get-CimInstance Win32_Processor).ProcessorId");
    let baseboard = get_ps_value("(Get-CimInstance Win32_BaseBoard).SerialNumber");
    let disk = get_ps_value("(Get-CimInstance Win32_DiskDrive | Select-Object -First 1).SerialNumber");

    // Fallback to wmic if PowerShell fails
    let cpuid = if cpuid.is_empty() { get_wmic_value("cpu get processorid") } else { cpuid };
    let baseboard = if baseboard.is_empty() { get_wmic_value("baseboard get serialnumber") } else { baseboard };
    let disk = if disk.is_empty() { get_wmic_value("diskdrive get serialnumber") } else { disk };

    let combined = format!("{}{}{}", cpuid, baseboard, disk);
    let clean = combined.replace(|c: char| c.is_whitespace(), "");

    // If hardware serials unavailable, use machine-specific fallback:
    // username + computername — unique per machine, not spoofable easily
    let fallback_combined = if clean.is_empty() {
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "user".to_string());
        let computername = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "pc".to_string());
        format!("FALLBACK-{}-{}", username, computername)
    } else {
        clean.clone()
    };
    let _ = clean; // suppress warning
    let clean = fallback_combined;

    if false {
        "".to_string()
    } else {
        let mut hasher = Sha256::new();
        hasher.update(HWID_SALT.as_bytes());
        hasher.update(clean.as_bytes());
        hex::encode(hasher.finalize())
    }
}

fn get_ps_value(query: &str) -> String {
    match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", query])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => String::new(),
    }
}

fn get_wmic_value(args: &str) -> String {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() { return String::new(); }

    match Command::new("wmic")
        .args(&parts)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            let s = String::from_utf8_lossy(&output.stdout).to_string();
            let lines: Vec<&str> = s.lines().collect();
            if lines.len() > 1 {
                lines[1].trim().to_string()
            } else {
                s.trim().to_string()
            }
        }
        Err(_) => String::new(),
    }
}
