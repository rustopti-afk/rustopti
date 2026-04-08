// RustOpti Background Service
// Keeps Timer Resolution active, monitors subscription, cleans up if uninstalled.
// No UI, no WebView2 — runs silently in background (~3-5MB RAM).

#![windows_subsystem = "windows"] // No console window

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use std::process::Command;
use std::os::windows::process::CommandExt;
use winreg::RegKey;
use winreg::enums::*;
use serde_json;

const CREATE_NO_WINDOW: u32 = 0x08000000;

// How often to check subscription (24 hours)
const CHECK_INTERVAL_SECS: u64 = 86400;

// API endpoint to check subscription
const API_URL: &str = "https://rustopti.fun/api/keys/validate";

fn main() {
    // Check if already running (single instance)
    if is_already_running() {
        return;
    }

    // Register in autostart
    register_autostart();

    // Boost timer resolution immediately
    boost_timer();

    let mut last_check = Instant::now() - Duration::from_secs(CHECK_INTERVAL_SECS);

    loop {
        // Check if main app was uninstalled
        if !main_exe_exists() {
            revert_all_tweaks();
            remove_autostart();
            std::process::exit(0);
        }

        // Check subscription once per day
        if last_check.elapsed().as_secs() >= CHECK_INTERVAL_SECS {
            if !is_subscription_valid() {
                revert_all_tweaks();
                remove_autostart();
                std::process::exit(0);
            }
            last_check = Instant::now();
        }

        // Sleep 1 hour between loop iterations
        thread::sleep(Duration::from_secs(3600));
    }
}

// ── Timer Resolution ──────────────────────────────────────────

fn boost_timer() {
    // Set timer resolution to ~1ms via PowerShell (persistent while this process lives)
    let ps_cmd = r#"
        Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class SvcWinMM { [DllImport("winmm.dll")] public static extern uint timeBeginPeriod(uint p); }' -Language CSharp -ErrorAction SilentlyContinue
        [SvcWinMM]::timeBeginPeriod(1) | Out-Null
    "#;
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

// ── Main exe check ────────────────────────────────────────────

fn main_exe_exists() -> bool {
    get_main_exe_path()
        .map(|p| p.exists())
        .unwrap_or(true) // if can't determine, assume exists
}

fn get_main_exe_path() -> Option<PathBuf> {
    // Read install path from registry (set by NSIS installer)
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(r"SOFTWARE\RustOpti").ok()?;
    let path: String = key.get_value("InstallPath").ok()?;
    Some(PathBuf::from(path).join("RustOpti.exe"))
}

// ── Subscription check ────────────────────────────────────────

fn is_subscription_valid() -> bool {
    // Read cached key from registry
    let key = match read_cached_key() {
        Some(k) => k,
        None => return true, // no key cached = not our business
    };

    let hwid = get_hwid();

    // Call API directly with reqwest (no PowerShell dependency)
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return true, // client build failed = keep active
    };

    let body = serde_json::json!({ "key": key, "hwid": hwid });

    match client.post(API_URL).json(&body).send() {
        Ok(resp) => {
            // Parse JSON response — if success=true → valid
            match resp.json::<serde_json::Value>() {
                Ok(json) => json.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                Err(_) => true, // parse error = treat as network issue, keep active
            }
        }
        Err(_) => true, // network unavailable = grace period, keep active
    }
}

fn read_cached_key() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"SOFTWARE\RustOpti").ok()?;
    let val: String = key.get_value("LicenseKey").ok()?;
    if val.is_empty() { None } else { Some(val) }
}

// Must match exactly the logic in src-tauri/src/utils/hwid.rs
const HWID_SALT: &str = "RustOpti-HWID-v2";

fn get_hwid() -> String {
    use sha2::{Sha256, Digest};

    let cpuid    = get_ps_value("(Get-CimInstance Win32_Processor).ProcessorId");
    let baseboard = get_ps_value("(Get-CimInstance Win32_BaseBoard).SerialNumber");
    let disk     = get_ps_value("(Get-CimInstance Win32_DiskDrive | Select-Object -First 1).SerialNumber");

    let combined = format!("{}{}{}", cpuid, baseboard, disk);
    let clean: String = combined.chars().filter(|c| !c.is_whitespace()).collect();

    let seed = if clean.is_empty() {
        let username     = std::env::var("USERNAME").unwrap_or_else(|_| "user".to_string());
        let computername = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "pc".to_string());
        format!("FALLBACK-{}-{}", username, computername)
    } else {
        clean
    };

    let mut hasher = Sha256::new();
    hasher.update(HWID_SALT.as_bytes());
    hasher.update(seed.as_bytes());
    hex::encode(hasher.finalize())
}

fn get_ps_value(query: &str) -> String {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", query])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

// ── Revert tweaks ─────────────────────────────────────────────

fn revert_all_tweaks() {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Restore GPU Scheduling
    if let Ok(key) = hklm.open_subkey_with_flags(
        r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", KEY_WRITE
    ) {
        let _ = key.set_value("HwSchMode", &1u32);
    }

    // Restore Network Throttling
    if let Ok(key) = hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile", KEY_WRITE
    ) {
        let _ = key.set_value("NetworkThrottlingIndex", &10u32);
    }

    // Re-enable HPET
    let _ = Command::new("bcdedit")
        .args(["/deletevalue", "useplatformtick"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    // Remove GlobalTimerResolutionRequests
    let _ = Command::new("reg")
        .args(["delete",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel",
            "/v", "GlobalTimerResolutionRequests", "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

// ── Autostart ─────────────────────────────────────────────────

fn register_autostart() {
    let exe_path = std::env::current_exe().unwrap_or_default();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE
    ) {
        let _ = key.set_value("RustOptiService", &exe_path.to_string_lossy().as_ref());
    }
}

fn remove_autostart() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE
    ) {
        let _ = key.delete_value("RustOptiService");
    }
}

// ── Single instance ───────────────────────────────────────────

fn is_already_running() -> bool {
    // Check for mutex via registry flag (simple approach)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(r"SOFTWARE\RustOpti", KEY_READ | KEY_WRITE) {
        let running: u32 = key.get_value("ServicePid").unwrap_or(0);
        if running != 0 {
            // Check if that PID is actually alive
            let output = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", running), "/NH"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .ok();
            if let Some(o) = output {
                let s = String::from_utf8_lossy(&o.stdout);
                if s.contains(&running.to_string()) {
                    return true; // already running
                }
            }
        }
        // Register our PID
        let pid = std::process::id();
        let _ = key.set_value("ServicePid", &pid);
    }
    false
}
