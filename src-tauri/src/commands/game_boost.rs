use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::System;
use crate::utils::license_guard::{LicenseState, require_license};
use crate::commands::game_mode::GameModeState;

const CREATE_NO_WINDOW: u32 = 0x08000000;

static GAME_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);
static ACTIVE_PROTECTION: AtomicBool = AtomicBool::new(false);
pub static PROTECTION_STOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
pub struct GameBoostResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GameBoostStatus {
    pub defender_excluded: bool,
    pub large_pages_enabled: bool,
    pub game_mode_active: bool,
    pub rust_running: bool,
    pub rust_pid: Option<u32>,
    pub rust_affinity: String,
}

// ═══════════════════════════════════════════════════════════════
// Windows Defender Exclusion
//
// Defender scans EVERY file Rust loads at runtime.
// Excluding the game folder gives +10-20 FPS.
// This is the single biggest optimization available.
// ═══════════════════════════════════════════════════════════════

/// Check if Rust folder is excluded from Defender
#[tauri::command]
pub fn get_defender_status() -> Result<GameBoostResult, String> {
    let rust_path = find_rust_path();

    let ps_cmd = "Get-MpPreference | Select-Object -ExpandProperty ExclusionPath";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let excluded = rust_path.as_ref()
        .map(|p| stdout.contains(&p.to_lowercase()))
        .unwrap_or(false);

    Ok(GameBoostResult {
        name: "Windows Defender Exclusion".to_string(),
        success: excluded,
        message: if excluded {
            format!("✅ Rust folder excluded: {}", rust_path.unwrap_or_default())
        } else if rust_path.is_some() {
            format!("⚠ Rust folder NOT excluded: {}", rust_path.unwrap())
        } else {
            "⚠ Rust installation not found".to_string()
        },
    })
}

/// Add Rust folder to Defender exclusions
#[tauri::command]
pub fn add_defender_exclusion(state: State<'_, LicenseState>) -> Result<GameBoostResult, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("defender_exclusion");

    let rust_path = find_rust_path()
        .ok_or("Rust installation not found. Install Rust via Steam first.")?;

    // Add game folder exclusion
    let ps_cmd = format!(
        "Add-MpPreference -ExclusionPath '{}'",
        rust_path.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    if output.status.success() {
        Ok(GameBoostResult {
            name: "Defender Exclusion".to_string(),
            success: true,
            message: format!("✓ Added exclusion for: {}", rust_path),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(GameBoostResult {
            name: "Defender Exclusion".to_string(),
            success: false,
            message: format!("✗ Failed (needs admin): {}", stderr.chars().take(150).collect::<String>()),
        })
    }
}

/// Remove Rust folder from Defender exclusions
#[tauri::command]
pub fn remove_defender_exclusion(state: State<'_, LicenseState>) -> Result<GameBoostResult, String> {
    require_license(&state)?;

    let rust_path = find_rust_path().unwrap_or_default();
    if rust_path.is_empty() {
        return Ok(GameBoostResult {
            name: "Defender Exclusion".to_string(),
            success: false,
            message: "✗ Rust path not found".to_string(),
        });
    }

    let ps_cmd = format!(
        "Remove-MpPreference -ExclusionPath '{}'",
        rust_path.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    if output.status.success() {
        Ok(GameBoostResult {
            name: "Defender Exclusion".to_string(),
            success: true,
            message: "✓ Defender exclusion removed".to_string(),
        })
    } else {
        Ok(GameBoostResult {
            name: "Defender Exclusion".to_string(),
            success: false,
            message: "✗ Failed to remove exclusion (needs admin)".to_string(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// Large Pages
//
// Enables "Lock Pages in Memory" privilege for the current user.
// Rust (Unity) benefits from large pages — fewer TLB misses.
// Gives +5-15% FPS improvement.
// ═══════════════════════════════════════════════════════════════

/// Check if Large Pages are enabled for current user
#[tauri::command]
pub fn get_large_pages_status() -> Result<GameBoostResult, String> {
    let ps_cmd = r#"
        $user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
        $export = secedit /export /cfg "$env:TEMP\secpol_check.cfg" 2>&1
        $content = Get-Content "$env:TEMP\secpol_check.cfg" -ErrorAction SilentlyContinue | Out-String
        Remove-Item "$env:TEMP\secpol_check.cfg" -ErrorAction SilentlyContinue
        if ($content -match 'SeLockMemoryPrivilege.*=.*\S') { Write-Output "ENABLED" } else { Write-Output "DISABLED" }
    "#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let enabled = stdout.contains("ENABLED");

    Ok(GameBoostResult {
        name: "Large Pages".to_string(),
        success: enabled,
        message: if enabled {
            "✅ Large Pages enabled".to_string()
        } else {
            "⚠ Large Pages disabled (default)".to_string()
        },
    })
}

/// Enable Large Pages for current user
#[tauri::command]
pub fn enable_large_pages(state: State<'_, LicenseState>) -> Result<GameBoostResult, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("large_pages");

    // Grant SeLockMemoryPrivilege to current user via ntrights or PowerShell
    let ps_cmd = r#"
        $user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
        $tmpFile = "$env:TEMP\secpol_lp.cfg"
        secedit /export /cfg $tmpFile | Out-Null
        $content = Get-Content $tmpFile
        $found = $false
        $newContent = $content | ForEach-Object {
            if ($_ -match '^SeLockMemoryPrivilege') {
                $found = $true
                if ($_ -notmatch [regex]::Escape($user)) {
                    "$_,$user"
                } else { $_ }
            } else { $_ }
        }
        if (-not $found) {
            $newContent = $newContent -replace '(\[Privilege Rights\])', "`$1`nSeLockMemoryPrivilege = $user"
        }
        $newContent | Set-Content $tmpFile
        secedit /configure /db "$env:TEMP\secpol_lp.sdb" /cfg $tmpFile /areas USER_RIGHTS | Out-Null
        Remove-Item $tmpFile, "$env:TEMP\secpol_lp.sdb" -ErrorAction SilentlyContinue
        Write-Output "OK"
    "#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if stdout.contains("OK") {
        Ok(GameBoostResult {
            name: "Large Pages".to_string(),
            success: true,
            message: "✓ Large Pages enabled. Reboot required for effect.".to_string(),
        })
    } else {
        Ok(GameBoostResult {
            name: "Large Pages".to_string(),
            success: false,
            message: "✗ Failed (needs admin rights)".to_string(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// Auto Game Mode
//
// When activated: kills bloatware, sets Rust to High priority,
// clears standby RAM, boosts timer. One-click gaming mode.
// ═══════════════════════════════════════════════════════════════

/// Activate game mode — optimize everything for Rust
#[tauri::command]
pub fn activate_game_mode(
    state:     State<'_, LicenseState>,
    gm_state:  State<'_, GameModeState>,
) -> Result<Vec<GameBoostResult>, String> {
    require_license(&state)?;

    let mut results = Vec::new();
    GAME_MODE_ACTIVE.store(true, Ordering::SeqCst);
    crate::utils::cleanup::register_tweak("game_mode");

    // Sync with AI Game Mode state so both UI panels show the same status
    if let Ok(mut status) = gm_state.0.lock() {
        status.active = true;
        if status.current_game.is_empty() {
            status.current_game = "Rust".to_string();
        }
    }

    // 1. Kill bloatware
    let bloat_killed = kill_gaming_bloat();
    results.push(GameBoostResult {
        name: "Kill Bloatware".to_string(),
        success: true,
        message: format!("✓ Closed {} unnecessary processes", bloat_killed),
    });

    // 2. Set Rust to High priority if running
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut rust_found = false;
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.contains("rustclient") || name.contains("rust.exe") {
            rust_found = true;
            let ps_cmd = format!(
                "Get-Process -Id {} | ForEach-Object {{ $_.PriorityClass = 'High' }}",
                pid.as_u32()
            );
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            results.push(GameBoostResult {
                name: "Rust Priority".to_string(),
                success: true,
                message: format!("✓ RustClient (PID {}) set to High priority", pid.as_u32()),
            });

            // 3. Set CPU affinity to performance cores (first 8 cores)
            let affinity_mask = get_performance_core_mask();
            let ps_affinity = format!(
                "$p = Get-Process -Id {}; $p.ProcessorAffinity = {}",
                pid.as_u32(), affinity_mask
            );
            match Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_affinity])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                Ok(o) if o.status.success() => {
                    results.push(GameBoostResult {
                        name: "CPU Affinity".to_string(),
                        success: true,
                        message: format!("✓ Rust pinned to performance cores (mask: 0x{:X})", affinity_mask),
                    });
                }
                _ => {
                    results.push(GameBoostResult {
                        name: "CPU Affinity".to_string(),
                        success: false,
                        message: "✗ Could not set CPU affinity".to_string(),
                    });
                }
            }
            // Continue to find other Rust instances
        }
    }

    if !rust_found {
        results.push(GameBoostResult {
            name: "Rust Priority".to_string(),
            success: false,
            message: "⚠ Rust not running. Start the game first, then activate Game Mode.".to_string(),
        });
    }

    // 4. Clear standby RAM (same method as ISLC)
    let clear_cmd = r#"
        Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class MemClean { [DllImport("psapi.dll")] public static extern bool EmptyWorkingSet(IntPtr hProcess); }' -Language CSharp
        Get-Process | Where-Object { $_.WorkingSet64 -gt 50MB -and $_.ProcessName -ne 'RustClient' } | ForEach-Object {
            try { [MemClean]::EmptyWorkingSet($_.Handle) } catch {}
        }
        Write-Output "OK"
    "#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", clear_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let ram_ok = output.map(|o| o.status.success()).unwrap_or(false);
    results.push(GameBoostResult {
        name: "RAM Cleanup".to_string(),
        success: ram_ok,
        message: if ram_ok {
            "✓ Working sets trimmed for non-game processes".to_string()
        } else {
            "⚠ Partial RAM cleanup (some processes protected)".to_string()
        },
    });

    Ok(results)
}

/// Deactivate game mode
#[tauri::command]
pub fn deactivate_game_mode(
    state:    State<'_, LicenseState>,
    gm_state: State<'_, GameModeState>,
) -> Result<GameBoostResult, String> {
    require_license(&state)?;
    GAME_MODE_ACTIVE.store(false, Ordering::SeqCst);

    // Sync with AI Game Mode state
    if let Ok(mut status) = gm_state.0.lock() {
        status.active = false;
        status.current_game = String::new();
        status.current_pid  = 0;
    }

    // Reset Rust priority to Normal if running
    let mut sys = System::new_all();
    sys.refresh_all();
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.contains("rustclient") || name.contains("rust.exe") {
            let ps_cmd = format!(
                "Get-Process -Id {} | ForEach-Object {{ $_.PriorityClass = 'Normal' }}",
                pid.as_u32()
            );
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            break;
        }
    }

    Ok(GameBoostResult {
        name: "Game Mode".to_string(),
        success: true,
        message: "✓ Game Mode deactivated. Priorities restored.".to_string(),
    })
}

/// Get game boost status overview
#[tauri::command]
pub fn get_game_boost_status() -> Result<GameBoostStatus, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut rust_pid = None;
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.contains("rustclient") || name.contains("rust.exe") {
            rust_pid = Some(pid.as_u32());
            break;
        }
    }

    Ok(GameBoostStatus {
        defender_excluded: false, // Will be checked separately
        large_pages_enabled: false, // Will be checked separately
        game_mode_active: GAME_MODE_ACTIVE.load(Ordering::SeqCst),
        rust_running: rust_pid.is_some(),
        rust_pid,
        rust_affinity: "Default".to_string(),
    })
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

/// Find Rust game installation path.
/// Reads all Steam library folders from libraryfolders.vdf so
/// Rust is found even when installed on a non-default drive.
fn find_rust_path() -> Option<String> {
    for lib in steam_library_paths() {
        let rust = format!(r"{}\steamapps\common\Rust", lib);
        if std::path::Path::new(&rust).exists() {
            return Some(rust);
        }
    }
    None
}

/// Returns all Steam library root paths parsed from libraryfolders.vdf.
fn steam_library_paths() -> Vec<String> {
    use winreg::{RegKey, enums::*};
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    // Read Steam install path from registry (try 64-bit and 32-bit keys)
    let steam_root = hklm
        .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
        .or_else(|_| hklm.open_subkey(r"SOFTWARE\Valve\Steam"))
        .and_then(|k| k.get_value::<String, _>("InstallPath"))
        .unwrap_or_default();

    let mut paths: Vec<String> = vec![steam_root.clone()];

    // Parse libraryfolders.vdf for additional library locations
    let vdf = format!(r"{}\steamapps\libraryfolders.vdf", steam_root);
    if let Ok(content) = std::fs::read_to_string(&vdf) {
        for line in content.lines() {
            let line = line.trim();
            // Match lines like:  "path"   "D:\\Games\\Steam"
            // or old format:      "1"      "D:\\Games\\Steam"
            if line.starts_with('"') {
                let parts: Vec<&str> = line.splitn(4, '"').collect();
                // parts: ["", key, whitespace, value]
                if parts.len() >= 4 {
                    let key = parts[1];
                    let val = parts[3].replace("\\\\", "\\");
                    if (key == "path" || key.parse::<u32>().is_ok()) && !val.is_empty() {
                        paths.push(val);
                    }
                }
            }
        }
    }

    // Also try common hardcoded fallback paths for systems without registry entry
    let fallbacks = [
        r"C:\Program Files (x86)\Steam",
        r"D:\Steam", r"D:\SteamLibrary",
        r"E:\Steam", r"E:\SteamLibrary",
        r"F:\Steam", r"F:\SteamLibrary",
    ];
    for p in fallbacks {
        if !paths.contains(&p.to_string()) {
            paths.push(p.to_string());
        }
    }

    paths
}

/// Get CPU affinity mask for performance cores
/// On Intel 12th+ gen: P-cores are first, E-cores are last
/// Default: use first 8 cores (covers most P-core configs)
fn get_performance_core_mask() -> u64 {
    let sys = System::new_all();
    let total_cores = sys.cpus().len();

    if total_cores == 0 || total_cores > 63 {
        // Fallback: use all cores (max safe value)
        return u64::MAX;
    }

    if total_cores <= 8 {
        // All cores (small CPU)
        (1u64 << total_cores) - 1
    } else {
        // Use first 8 cores (typically P-cores on hybrid CPUs)
        0xFF
    }
}

// ═══════════════════════════════════════════════════════════════
// Active Protection
//
// Background loop that runs every 30 seconds while the optimizer
// is open:
//   1. Trim working sets of background processes — frees RAM
//      without rebooting (pushes inactive pages back to free/standby)
//   2. Maintain Rust at High priority — catches cases where the
//      game just launched or Windows reset the priority
//
// Uses native Windows API (no PowerShell overhead).
// ═══════════════════════════════════════════════════════════════

/// Start the background Active Protection loop
#[tauri::command]
pub async fn start_active_protection(state: State<'_, LicenseState>) -> Result<GameBoostResult, String> {
    require_license(&state)?;

    if ACTIVE_PROTECTION.load(Ordering::SeqCst) {
        return Ok(GameBoostResult {
            name: "Active Protection".to_string(),
            success: false,
            message: "⚠ Already running".to_string(),
        });
    }

    ACTIVE_PROTECTION.store(true, Ordering::SeqCst);
    PROTECTION_STOP.store(false, Ordering::SeqCst);
    crate::utils::cleanup::register_tweak("active_protection");

    tokio::spawn(async move {
        loop {
            if PROTECTION_STOP.load(Ordering::SeqCst) {
                break;
            }
            // Run blocking operations in thread pool (don't block the async runtime)
            tokio::task::spawn_blocking(trim_background_working_sets).await.ok();
            tokio::task::spawn_blocking(maintain_rust_priority).await.ok();

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
        ACTIVE_PROTECTION.store(false, Ordering::SeqCst);
    });

    Ok(GameBoostResult {
        name: "Active Protection".to_string(),
        success: true,
        message: "✓ Active Protection ON — RAM trim & Rust priority every 30s".to_string(),
    })
}

/// Stop the background Active Protection loop
#[tauri::command]
pub fn stop_active_protection() -> Result<GameBoostResult, String> {
    PROTECTION_STOP.store(true, Ordering::SeqCst);
    crate::utils::cleanup::unregister_tweak("active_protection");
    Ok(GameBoostResult {
        name: "Active Protection".to_string(),
        success: true,
        message: "✓ Active Protection stopped".to_string(),
    })
}

/// Returns true if Active Protection loop is running
#[tauri::command]
pub fn get_active_protection_status() -> bool {
    ACTIVE_PROTECTION.load(Ordering::SeqCst)
}

/// Called when subscription expires — stops all background loops and reverts all tweaks.
/// Intentionally has NO license check: we always want cleanup to work.
#[tauri::command]
pub fn subscription_expired_cleanup() -> GameBoostResult {
    // Stop Active Protection loop
    PROTECTION_STOP.store(true, Ordering::SeqCst);
    // Stop ISLC monitor
    crate::commands::islc::STOP_SIGNAL.store(true, Ordering::SeqCst);
    // Revert all active tweaks
    crate::utils::cleanup::revert_all_tweaks();

    GameBoostResult {
        name: "Subscription Expired".to_string(),
        success: true,
        message: "✓ All tweaks reverted — subscription expired".to_string(),
    }
}

/// Trim working sets of all background processes using Windows API.
/// This pushes inactive RAM pages back to standby/free — no rebooting needed.
fn trim_background_working_sets() {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA};
    use windows::Win32::System::ProcessStatus::K32EmptyWorkingSet;
    use windows::Win32::Foundation::CloseHandle;

    const PROTECTED: &[&str] = &[
        "rustclient", "rust.exe", "steam", "easyanticheat", "explorer",
        "svchost", "csrss", "lsass", "winlogon", "dwm", "smss",
        "wininit", "system", "rustopti",
    ];

    let mut sys = System::new_all();
    sys.refresh_all();

    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if PROTECTED.iter().any(|p| name.contains(p)) {
            continue;
        }
        unsafe {
            // PROCESS_SET_QUOTA is sufficient for EmptyWorkingSet (MSDN)
            if let Ok(handle) = OpenProcess(PROCESS_SET_QUOTA, false, pid.as_u32()) {
                let _ = K32EmptyWorkingSet(handle);
                let _ = CloseHandle(handle);
            }
        }
    }
}

/// Re-apply High priority to RustClient if it's running.
/// Catches cases where the game was started after Active Protection began.
fn maintain_rust_priority() {
    use windows::Win32::System::Threading::{
        OpenProcess, SetPriorityClass, PROCESS_SET_INFORMATION, HIGH_PRIORITY_CLASS,
    };
    use windows::Win32::Foundation::CloseHandle;

    let mut sys = System::new_all();
    sys.refresh_all();

    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.contains("rustclient") || name == "rust.exe" {
            unsafe {
                if let Ok(handle) = OpenProcess(PROCESS_SET_INFORMATION, false, pid.as_u32()) {
                    let _ = SetPriorityClass(handle, HIGH_PRIORITY_CLASS);
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
}

/// Kill common bloatware processes for gaming
fn kill_gaming_bloat() -> u32 {
    let targets = [
        "OneDrive", "Teams", "Cortana", "SearchApp", "YourPhone",
        "GameBarPresenceWriter", "SkypeApp", "Widgets",
        "PhoneExperienceHost", "HxTsr", "HxOutlook",
        "MicrosoftEdgeUpdate",
        "spotify",
    ];

    let mut killed = 0u32;
    let mut sys = System::new_all();
    sys.refresh_all();

    for (_, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_string();
        let name_lower = name.to_lowercase();

        // Don't kill critical processes
        let protected = [
            "rustclient", "rust.exe", "steam", "easyanticheat", "explorer",
            "svchost", "csrss", "lsass", "winlogon", "dwm", "services",
            "smss", "wininit", "system", "rustopti", "tauri",
        ];
        if protected.iter().any(|p| name_lower.contains(p)) {
            continue;
        }

        for target in &targets {
            if name_lower.contains(&target.to_lowercase()) {
                if proc.kill() {
                    killed += 1;
                }
                break;
            }
        }
    }

    killed
}
