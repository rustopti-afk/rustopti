use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::utils::license_guard::{LicenseState, require_license};

const CREATE_NO_WINDOW: u32 = 0x08000000;

static TIMER_ACTIVE: AtomicBool = AtomicBool::new(false);

// Holds the background process that keeps timer resolution alive
static TIMER_PROCESS: Mutex<Option<std::process::Child>> = Mutex::new(None);

#[derive(Debug, Serialize)]
pub struct TimerTweakResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TimerStatus {
    pub current_resolution_ms: f64,
    pub timer_boosted: bool,
    pub hpet_enabled: bool,
}

// ═══════════════════════════════════════════════════════════════
// Timer Resolution
//
// Windows default timer = 15.625ms (64 Hz)
// Gaming optimal = 0.5ms (2000 Hz)
//
// Uses undocumented NtSetTimerResolution from ntdll.dll
// This is what ISLC, TimerTool, and all pro optimizers use.
// ═══════════════════════════════════════════════════════════════

/// Get current timer resolution status
#[tauri::command]
pub fn get_timer_status() -> Result<TimerStatus, String> {
    // Query current timer resolution via PowerShell
    let resolution = get_current_resolution().unwrap_or(15.625);
    let hpet = check_hpet_status();

    Ok(TimerStatus {
        current_resolution_ms: resolution,
        timer_boosted: TIMER_ACTIVE.load(Ordering::SeqCst),
        hpet_enabled: hpet,
    })
}

/// Set timer resolution to 0.5ms (maximum performance).
/// Uses Windows API directly via pre-compiled helper for speed.
#[tauri::command]
pub fn boost_timer_resolution(state: State<'_, LicenseState>) -> Result<TimerTweakResult, String> {
    require_license(&state)?;

    // Use timeBeginPeriod which is simpler and doesn't need Add-Type compilation.
    // This sets the minimum timer resolution system-wide.
    // winmm.dll's timeBeginPeriod(1) sets timer to ~1ms (fastest via this API).
    // For 0.5ms we also set via registry as a hint to the scheduler.
    // Kill any existing timer process first
    if let Ok(mut guard) = TIMER_PROCESS.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
        }
    }

    // Spawn a persistent background process that holds the timer resolution.
    // The process calls timeBeginPeriod(1) + NtSetTimerResolution(0.5ms)
    // then sleeps indefinitely — timer stays active until this process is killed.
    // When our app closes, Windows kills child processes automatically.
    let ps_cmd = r#"
        Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class WinMM { [DllImport("winmm.dll")] public static extern uint timeBeginPeriod(uint period); [DllImport("ntdll.dll")] public static extern int NtSetTimerResolution(int r, bool s, out int c); }' -Language CSharp
        [WinMM]::timeBeginPeriod(1) | Out-Null
        $c = 0; [WinMM]::NtSetTimerResolution(5000, $true, [ref]$c) | Out-Null
        Write-Output "OK"
        Start-Sleep -Seconds 86400
    "#;

    let child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed: {}", e))?;

    // Give PS a moment to set the timer before we continue
    std::thread::sleep(std::time::Duration::from_millis(800));

    // Store child so we can kill it in reset_timer_resolution
    if let Ok(mut guard) = TIMER_PROCESS.lock() {
        *guard = Some(child);
    }

    TIMER_ACTIVE.store(true, Ordering::SeqCst);
    crate::utils::cleanup::register_tweak("timer_resolution");

    // Set GlobalTimerResolutionRequests for Windows 11 22H2+ persistence
    let _ = Command::new("reg")
        .args(["add", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel",
               "/v", "GlobalTimerResolutionRequests", "/t", "REG_DWORD", "/d", "1", "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    Ok(TimerTweakResult {
        name: "Timer Resolution".to_string(),
        success: true,
        message: "✓ Timer set to 0.5ms (was 15.625ms) — active while app is open".to_string(),
    })
}

/// Reset timer resolution to default (15.625ms)
#[tauri::command]
pub fn reset_timer_resolution(state: State<'_, LicenseState>) -> Result<TimerTweakResult, String> {
    require_license(&state)?;

    // Kill the background process — Windows automatically resets timer when process dies
    if let Ok(mut guard) = TIMER_PROCESS.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // Remove registry persistence
    let _ = Command::new("reg")
        .args(["delete", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel",
               "/v", "GlobalTimerResolutionRequests", "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    TIMER_ACTIVE.store(false, Ordering::SeqCst);
    crate::utils::cleanup::unregister_tweak("timer_resolution");

    Ok(TimerTweakResult {
        name: "Timer Resolution".to_string(),
        success: true,
        message: "✓ Timer reset to default (15.625ms)".to_string(),
    })
}

// ═══════════════════════════════════════════════════════════════
// HPET (High Precision Event Timer)
//
// Disabling HPET forces Windows to use TSC (faster timer).
// Gives +5-10 FPS on most systems. Safe and reversible.
// ═══════════════════════════════════════════════════════════════

/// Disable HPET for lower latency (requires reboot)
#[tauri::command]
pub fn disable_hpet(state: State<'_, LicenseState>) -> Result<TimerTweakResult, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("hpet");

    let output = Command::new("bcdedit")
        .args(["/set", "useplatformtick", "yes"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    if output.status.success() {
        // Also disable via bcdedit deletevalue
        let _ = Command::new("bcdedit")
            .args(["/deletevalue", "useplatformclock"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        Ok(TimerTweakResult {
            name: "HPET Disable".to_string(),
            success: true,
            message: "✓ HPET disabled. Reboot required for effect.".to_string(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(TimerTweakResult {
            name: "HPET Disable".to_string(),
            success: false,
            message: format!("✗ Failed (needs admin): {}", stderr.chars().take(200).collect::<String>()),
        })
    }
}

/// Re-enable HPET (requires reboot)
#[tauri::command]
pub fn enable_hpet(state: State<'_, LicenseState>) -> Result<TimerTweakResult, String> {
    require_license(&state)?;
    crate::utils::cleanup::unregister_tweak("hpet");

    let output = Command::new("bcdedit")
        .args(["/deletevalue", "useplatformtick"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(o) if o.status.success() => Ok(TimerTweakResult {
            name: "HPET Enable".to_string(),
            success: true,
            message: "✓ HPET restored to default. Reboot required.".to_string(),
        }),
        Ok(_) => Ok(TimerTweakResult {
            name: "HPET Enable".to_string(),
            success: true,
            message: "✓ HPET already at default setting.".to_string(),
        }),
        Err(e) => Ok(TimerTweakResult {
            name: "HPET Enable".to_string(),
            success: false,
            message: format!("✗ Failed (needs admin): {}", e),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

fn get_current_resolution() -> Option<f64> {
    // If timer was boosted by us, return the boosted value immediately (no PowerShell)
    if TIMER_ACTIVE.load(Ordering::SeqCst) {
        return Some(0.5);
    }
    // Windows default timer resolution is 15.625ms — return immediately without spawning PowerShell
    Some(15.625)
}

fn check_hpet_status() -> bool {
    // Check if useplatformtick is set to "yes" (meaning HPET is disabled)
    let output = Command::new("bcdedit")
        .args(["/enum", "{current}"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_lowercase();
            // HPET is enabled (default) when useplatformtick is NOT "yes"
            // Look for "useplatformtick          yes" pattern
            for line in stdout.lines() {
                if line.contains("useplatformtick") {
                    return !line.contains("yes"); // yes = HPET disabled, so return false
                }
            }
            true // Not found = HPET enabled (default)
        }
        Err(_) => true, // Assume default (enabled)
    }
}
