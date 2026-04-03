use serde::Serialize;
use tauri::State;
use sysinfo::System;
use std::process::Command;
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crate::utils::license_guard::{LicenseState, require_license};

const CREATE_NO_WINDOW: u32 = 0x08000000;

// ═══════════════════════════════════════════════════════════════
// Global ISLC Monitor State (lock-free atomics)
// ═══════════════════════════════════════════════════════════════
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);
static MONITOR_CLEARS: AtomicU64 = AtomicU64::new(0);
static MONITOR_THRESHOLD: AtomicU64 = AtomicU64::new(1024);
pub static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

// ═══════════════════════════════════════════════════════════════
// Data Structures
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct StandbyInfo {
    pub total_ram_mb: u64,
    pub used_ram_mb: u64,
    pub free_ram_mb: u64,
    pub standby_mb: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize)]
pub struct IslcStatus {
    pub monitor_running: bool,
    pub threshold_mb: u64,
    pub total_clears: u64,
}

#[derive(Debug, Serialize)]
pub struct IslcResult {
    pub success: bool,
    pub message: String,
}

// ═══════════════════════════════════════════════════════════════
// PowerShell script to clear Windows Standby List
// Uses NtSetSystemInformation(SystemMemoryListInformation=80,
//   MemoryPurgeStandbyList=4) from ntdll.dll
// Uses RtlAdjustPrivilege (simpler than OpenProcessToken combo)
// Requires Administrator privileges
// ═══════════════════════════════════════════════════════════════

const PS_CLEAR_STANDBY: &str = r#"
$typeName = 'StandbyClr'
if (-not ([System.Management.Automation.PSTypeName]$typeName).Type) {
    Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public class StandbyClr {
    [DllImport("ntdll.dll")] public static extern int RtlAdjustPrivilege(int priv, bool enable, bool thread, ref bool prev);
    [DllImport("ntdll.dll")] public static extern int NtSetSystemInformation(int cls, ref int info, int len);
}
'@ -Language CSharp
}
$prev = $false
$null = [StandbyClr]::RtlAdjustPrivilege(9, $true, $false, [ref]$prev)
$null = [StandbyClr]::RtlAdjustPrivilege(5, $true, $false, [ref]$prev)
$cmd = 4
$r = [StandbyClr]::NtSetSystemInformation(80, [ref]$cmd, 4)
if ($r -eq 0) { Write-Output 'OK' } else { Write-Output "FAIL:$r" }
"#;

// ═══════════════════════════════════════════════════════════════
// Tauri Commands
// ═══════════════════════════════════════════════════════════════

/// Get current RAM and Standby List information
#[tauri::command]
pub fn get_standby_info() -> Result<StandbyInfo, String> {
    let mut sys = System::new();
    sys.refresh_memory();

    let total = sys.total_memory() / 1_048_576;
    let used = sys.used_memory() / 1_048_576;
    let available = sys.available_memory() / 1_048_576;
    let free = sys.free_memory() / 1_048_576;
    let standby = if available > free { available - free } else { 0 };
    let usage = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    Ok(StandbyInfo {
        total_ram_mb: total,
        used_ram_mb: used,
        free_ram_mb: free,
        standby_mb: standby,
        usage_percent: usage,
    })
}

/// Clear Standby List immediately (one-shot)
#[tauri::command]
pub fn clear_standby_now(state: State<'_, LicenseState>) -> Result<IslcResult, String> {
    require_license(&state)?;
    match run_clear_standby() {
        true => {
            MONITOR_CLEARS.fetch_add(1, Ordering::Relaxed);
            Ok(IslcResult {
                success: true,
                message: "✓ Standby List cleared successfully".to_string(),
            })
        }
        false => Ok(IslcResult {
            success: false,
            message: "✗ Failed to clear Standby List (run as Administrator)".to_string(),
        }),
    }
}

/// Start background ISLC monitor that auto-clears when standby exceeds threshold
#[tauri::command]
pub async fn start_islc_monitor(threshold_mb: u64, state: State<'_, LicenseState>) -> Result<IslcResult, String> {
    require_license(&state)?;
    if MONITOR_RUNNING.load(Ordering::SeqCst) {
        return Ok(IslcResult {
            success: false,
            message: "⚠ Monitor is already running".to_string(),
        });
    }

    let threshold = if threshold_mb < 128 { 512 } else { threshold_mb };
    MONITOR_THRESHOLD.store(threshold, Ordering::SeqCst);
    STOP_SIGNAL.store(false, Ordering::SeqCst);
    MONITOR_RUNNING.store(true, Ordering::SeqCst);
    crate::utils::cleanup::register_tweak("islc_monitor");

    tokio::spawn(async move {
        loop {
            if STOP_SIGNAL.load(Ordering::SeqCst) {
                break;
            }

            // Check standby memory size
            let mut sys = System::new();
            sys.refresh_memory();
            let available = sys.available_memory() / 1_048_576;
            let free = sys.free_memory() / 1_048_576;
            let standby = if available > free { available - free } else { 0 };
            let current_threshold = MONITOR_THRESHOLD.load(Ordering::Relaxed);

            if standby > current_threshold {
                if run_clear_standby() {
                    MONITOR_CLEARS.fetch_add(1, Ordering::Relaxed);
                }
            }

            // Sleep 10 seconds between checks
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
        MONITOR_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(IslcResult {
        success: true,
        message: format!(
            "✓ ISLC Monitor started (threshold: {} MB)",
            threshold
        ),
    })
}

/// Stop the background ISLC monitor
#[tauri::command]
pub fn stop_islc_monitor() -> Result<IslcResult, String> {
    if !MONITOR_RUNNING.load(Ordering::SeqCst) {
        return Ok(IslcResult {
            success: false,
            message: "⚠ Monitor is not running".to_string(),
        });
    }

    STOP_SIGNAL.store(true, Ordering::SeqCst);

    Ok(IslcResult {
        success: true,
        message: "✓ ISLC Monitor stopping...".to_string(),
    })
}

/// Get current ISLC monitor status
#[tauri::command]
pub fn get_islc_status() -> Result<IslcStatus, String> {
    Ok(IslcStatus {
        monitor_running: MONITOR_RUNNING.load(Ordering::SeqCst),
        threshold_mb: MONITOR_THRESHOLD.load(Ordering::Relaxed),
        total_clears: MONITOR_CLEARS.load(Ordering::Relaxed),
    })
}

// ═══════════════════════════════════════════════════════════════
// Internal Helpers
// ═══════════════════════════════════════════════════════════════

/// Execute the PowerShell standby list clear script
fn run_clear_standby() -> bool {
    match Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            PS_CLEAR_STANDBY,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.trim().contains("OK")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_standby_info() {
        let info = get_standby_info();
        assert!(info.is_ok());
        let data = info.unwrap();
        assert!(data.total_ram_mb > 0);
        assert!(data.usage_percent >= 0.0 && data.usage_percent <= 100.0);
    }

    #[test]
    fn test_get_islc_status() {
        let status = get_islc_status();
        assert!(status.is_ok());
        let data = status.unwrap();
        assert!(!data.monitor_running);
    }
}
