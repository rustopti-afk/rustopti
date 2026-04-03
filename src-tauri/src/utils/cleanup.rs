use std::sync::Mutex;
use std::process::Command;
use std::os::windows::process::CommandExt;
use std::collections::HashSet;

const CREATE_NO_WINDOW: u32 = 0x08000000;

static ACTIVE_TWEAKS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn tweaks_file_path() -> Option<std::path::PathBuf> {
    std::env::var("APPDATA").ok().map(|a| {
        std::path::PathBuf::from(a).join("RustOpti").join(".active_tweaks")
    })
}

fn persist_tweaks(tweaks: &HashSet<String>) {
    if let Some(path) = tweaks_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = tweaks.iter().cloned().collect::<Vec<_>>().join("\n");
        let _ = std::fs::write(&path, data);
        // Also persist to registry as backup (can't easily delete)
        let _ = persist_to_registry(tweaks);
    }
}

fn persist_to_registry(tweaks: &HashSet<String>) -> Result<(), String> {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    let data = tweaks.iter().cloned().collect::<Vec<_>>().join(",");
    registry_helper::set_string(HKEY_CURRENT_USER, r"Software\RustOpti", "ActiveTweaks", &data)
}

fn load_persisted_tweaks() -> HashSet<String> {
    // Try file first
    let from_file: HashSet<String> = tweaks_file_path()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect())
        .unwrap_or_default();

    if !from_file.is_empty() {
        return from_file;
    }

    // Fallback: try registry backup
    load_from_registry()
}

fn load_from_registry() -> HashSet<String> {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    match registry_helper::get_string(HKEY_CURRENT_USER, r"Software\RustOpti", "ActiveTweaks") {
        Ok(s) => s.split(',').filter(|l| !l.is_empty()).map(|l| l.to_string()).collect(),
        Err(_) => HashSet::new(),
    }
}

fn clear_persisted_tweaks() {
    // Delete file
    if let Some(path) = tweaks_file_path() {
        let _ = std::fs::remove_file(&path);
    }
    // Delete registry backup
    use winreg::RegKey;
    use winreg::enums::*;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(r"Software\RustOpti", winreg::enums::KEY_WRITE) {
        let _ = key.delete_value("ActiveTweaks");
    }
}

pub fn revert_leftover_tweaks() {
    let leftover = load_persisted_tweaks();
    if !leftover.is_empty() {
        eprintln!("[RustOpti] Found {} leftover tweaks from previous session, reverting...", leftover.len());
        let mut guard = ACTIVE_TWEAKS.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(leftover);
        drop(guard);
        revert_all_tweaks();
    }
}

pub fn register_tweak(name: &str) {
    let mut guard = ACTIVE_TWEAKS.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(HashSet::new());
    }
    if let Some(set) = guard.as_mut() {
        set.insert(name.to_string());
        persist_tweaks(set);
    }
}

pub fn unregister_tweak(name: &str) {
    let mut guard = ACTIVE_TWEAKS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(set) = guard.as_mut() {
        set.remove(name);
        persist_tweaks(set);
    }
}

pub fn revert_all_tweaks() {
    let tweaks = {
        let mut guard = ACTIVE_TWEAKS.lock().unwrap_or_else(|e| e.into_inner());
        guard.take().unwrap_or_default()
    };

    if tweaks.is_empty() {
        return;
    }

    eprintln!("[RustOpti] Reverting {} tweaks...", tweaks.len());

    let mut failed = HashSet::new();

    for tweak in &tweaks {
        let result = match tweak.as_str() {
            "timer_resolution" => revert_timer(),
            "sysmain" => revert_sysmain(),
            "visual_effects" => revert_visual_effects(),
            "core_unpark" => revert_core_parking(),
            "game_mode" => revert_game_mode(),
            "registry_tweaks" => revert_registry(),
            "network_tweaks" => revert_network(),
            "gpu_tweaks" => revert_gpu(),
            "power_tweaks" => revert_power(),
            "hpet" => revert_hpet(),
            "msi_mode" => revert_msi(),
            "islc_monitor" => revert_islc(),
            "active_protection" => revert_active_protection(),
            "defender_exclusion" => revert_defender(),
            "large_pages" => revert_large_pages(),
            "rust_tweaks" => revert_rust_tweaks(),
            _ => true,
        };
        if !result {
            failed.insert(tweak.clone());
        }
        eprintln!("[RustOpti] Revert {}: {}", tweak, if result { "OK" } else { "FAILED" });
    }

    if failed.is_empty() {
        clear_persisted_tweaks();
    } else {
        // Re-persist only failed tweaks for next startup retry
        persist_tweaks(&failed);
    }
}

// ═══════════════════════════════════════════════════════════════
// COMPLETE revert functions — every value that was changed
// ═══════════════════════════════════════════════════════════════

fn revert_timer() -> bool {
    let ps = r#"
        Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class WR { [DllImport("winmm.dll")] public static extern uint timeEndPeriod(uint p); [DllImport("ntdll.dll")] public static extern int NtSetTimerResolution(int r, bool s, out int c); }' -Language CSharp
        [WR]::timeEndPeriod(1)
        $c=0; [WR]::NtSetTimerResolution(156250, $false, [ref]$c)
    "#;
    let _ = run_ps(ps);
    // Also remove GlobalTimerResolutionRequests registry key
    let _ = run_cmd("reg", &["delete", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel",
        "/v", "GlobalTimerResolutionRequests", "/f"]);
    true
}

fn revert_sysmain() -> bool {
    let _ = run_cmd("sc", &["config", "SysMain", "start=", "auto"]);
    run_cmd("sc", &["start", "SysMain"])
}

fn revert_visual_effects() -> bool {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    // Revert ALL 5 values
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects", "VisualFXSetting", 0);
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "EnableTransparency", 1);
    let _ = registry_helper::set_string(HKEY_CURRENT_USER,
        r"Control Panel\Desktop\WindowMetrics", "MinAnimate", "1");
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"Control Panel\Desktop", "SmoothScroll", 1);
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "ListviewShadow", 1);
    true
}

fn revert_core_parking() -> bool {
    // Revert CPMINCORES, CPMAXCORES, IDLEDISABLE, and registry ValueMin
    let _ = run_cmd("powercfg", &["/setacvalueindex", "scheme_current", "sub_processor", "CPMINCORES", "50"]);
    let _ = run_cmd("powercfg", &["/setacvalueindex", "scheme_current", "sub_processor", "CPMAXCORES", "100"]);
    let _ = run_cmd("powercfg", &["/setacvalueindex", "scheme_current", "sub_processor", "IDLEDISABLE", "0"]);
    let _ = run_cmd("powercfg", &["/setactive", "scheme_current"]);
    // Registry persistence
    use crate::utils::registry_helper;
    use winreg::enums::*;
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Power\PowerSettings\54533251-82be-4824-96c1-47b60b740d00\0cc5b647-c1df-4637-891a-dec35c318583",
        "ValueMin", 50);
    true
}

fn revert_game_mode() -> bool {
    let ps = r#"
        Get-Process | Where-Object { $_.ProcessName -match 'RustClient|rust' } | ForEach-Object {
            try { $_.PriorityClass = 'Normal'; $_.ProcessorAffinity = [IntPtr]::new(-1) } catch {}
        }
    "#;
    run_ps(ps)
}

fn revert_registry() -> bool {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    // Revert ALL 7 registry tweaks
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 1);
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"SOFTWARE\Microsoft\GameBar", "AllowAutoGameMode", 0);
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"System\GameConfigStore", "GameDVR_FSEBehavior", 0);
    let _ = registry_helper::set_dword(HKEY_CURRENT_USER,
        r"Control Panel\Mouse", "MouseSpeed", 1);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
        "NetworkThrottlingIndex", 10);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
        "GPU Priority", 2);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
        "Priority", 2);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", 1);
    true
}

fn revert_network() -> bool {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    // Revert NetworkThrottlingIndex
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
        "NetworkThrottlingIndex", 10);
    // Revert TCP settings
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "TcpNoDelay", 0);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "DisableTaskOffload", 0);
    // Revert Nagle on all interfaces
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let ifaces_path = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";
    if let Ok(ifaces_key) = hklm.open_subkey(ifaces_path) {
        for iface in ifaces_key.enum_keys().filter_map(|k| k.ok()) {
            let path = format!(r"{}\{}", ifaces_path, iface);
            let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &path, "TcpAckFrequency", 2);
            let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &path, "TCPNoDelay", 0);
            let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &path, "TcpDelAckTicks", 1);
        }
    }
    // Delete DefaultReceiveWindow (restore OS default)
    let _ = run_cmd("reg", &["delete",
        r"HKLM\SYSTEM\CurrentControlSet\Services\AFD\Parameters",
        "/v", "DefaultReceiveWindow", "/f"]);
    true
}

fn revert_gpu() -> bool {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    // Revert ALL 4 NVIDIA tweaks
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\NVIDIA Corporation\Global\NVTweak", "LowLatencyMode", 0);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\NVIDIA Corporation\Global\NVTweak", "PreRenderedFrames", 3);
    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE,
        r"SOFTWARE\NVIDIA Corporation\Global\NVTweak", "ThreadedOptimization", 0);
    // Delete ShaderCacheSize (restore default)
    let _ = run_cmd("reg", &["delete",
        r"HKLM\SOFTWARE\NVIDIA Corporation\Global\NVTweak",
        "/v", "ShaderCacheSize", "/f"]);
    true
}

fn revert_power() -> bool {
    // Restore Balanced power plan
    let _ = run_cmd("powercfg", &["/setactive", "381b4222-f694-41f0-9685-ff5bb260df2e"]);
    // Restore CPU min state
    let _ = run_cmd("powercfg", &["/setacvalueindex", "scheme_current", "sub_processor", "PROCTHROTTLEMIN", "5"]);
    // Re-enable USB selective suspend
    let _ = run_cmd("powercfg", &["/setacvalueindex", "scheme_current",
        "2a737441-1930-4402-8d77-b2bebba308a3", "48e6b7a6-50f5-4782-a5d4-53bb8f07e226", "1"]);
    let _ = run_cmd("powercfg", &["/setactive", "scheme_current"]);
    // Delete the duplicated Ultimate Performance plan (find by name)
    let _ = run_ps(r#"
        $plans = powercfg /list
        $plans | ForEach-Object {
            if ($_ -match 'Ultimate Performance' -and $_ -match '([0-9a-f-]{36})') {
                $guid = $Matches[1]
                powercfg /delete $guid 2>$null
            }
        }
    "#);
    true
}

fn revert_hpet() -> bool {
    let _ = run_cmd("bcdedit", &["/deletevalue", "useplatformtick"]);
    let _ = run_cmd("bcdedit", &["/deletevalue", "useplatformclock"]);
    true
}

fn revert_msi() -> bool {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    // Find and disable MSI for all Display devices
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let enum_path = r"SYSTEM\CurrentControlSet\Enum\PCI";
    if let Ok(pci_key) = hklm.open_subkey(enum_path) {
        for device_id in pci_key.enum_keys().filter_map(|k| k.ok()) {
            let device_path = format!(r"{}\{}", enum_path, device_id);
            if let Ok(device_key) = hklm.open_subkey(&device_path) {
                for instance in device_key.enum_keys().filter_map(|k| k.ok()) {
                    let instance_path = format!(r"{}\{}", device_path, instance);
                    if let Ok(inst_key) = hklm.open_subkey(&instance_path) {
                        let class: String = inst_key.get_value("Class").unwrap_or_default();
                        let class_guid: String = inst_key.get_value("ClassGUID").unwrap_or_default();
                        let desc: String = inst_key.get_value("DeviceDesc").unwrap_or_default();
                        let desc_lower = desc.to_lowercase();
                        let is_display = class.eq_ignore_ascii_case("Display")
                            || class_guid.eq_ignore_ascii_case("{4d36e968-e325-11ce-bfc1-08002be10318}")
                            || desc_lower.contains("nvidia") || desc_lower.contains("geforce")
                            || desc_lower.contains("radeon") || desc_lower.contains("amd")
                            || (desc_lower.contains("intel") && desc_lower.contains("graphics"));
                        if is_display {
                            let msi_path = format!(r"{}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties", instance_path);
                            let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &msi_path, "MSISupported", 0);
                        }
                    }
                }
            }
        }
    }
    true
}

fn revert_islc() -> bool {
    use std::sync::atomic::Ordering;
    crate::commands::islc::STOP_SIGNAL.store(true, Ordering::SeqCst);
    true
}

fn revert_defender() -> bool {
    let ps = r#"
        $paths = (Get-MpPreference).ExclusionPath
        if ($paths) {
            $paths | Where-Object { $_ -match 'Rust|Steam' } | ForEach-Object {
                Remove-MpPreference -ExclusionPath $_
            }
        }
    "#;
    run_ps(ps)
}

fn revert_rust_tweaks() -> bool {
    use crate::utils::registry_helper;
    use winreg::enums::*;
    // Remove compatibility flags for Rust
    let _ = run_cmd("reg", &["delete",
        r"HKCU\Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers",
        "/v", r"C:\Program Files (x86)\Steam\steamapps\common\Rust\RustClient.exe", "/f"]);
    true
}

fn revert_large_pages() -> bool {
    let ps = r#"
        $user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
        $tmpFile = "$env:TEMP\secpol_revert.cfg"
        secedit /export /cfg $tmpFile | Out-Null
        $content = Get-Content $tmpFile
        $newContent = $content | ForEach-Object {
            if ($_ -match '^SeLockMemoryPrivilege') {
                $val = $_ -replace [regex]::Escape(",$user"), '' -replace [regex]::Escape($user), ''
                if ($val -match '=\s*$') { $null } else { $val }
            } else { $_ }
        } | Where-Object { $_ -ne $null }
        $newContent | Set-Content $tmpFile
        secedit /configure /db "$env:TEMP\secpol_revert.sdb" /cfg $tmpFile /areas USER_RIGHTS 2>$null | Out-Null
        Remove-Item $tmpFile, "$env:TEMP\secpol_revert.sdb" -ErrorAction SilentlyContinue
    "#;
    run_ps(ps)
}

fn revert_active_protection() -> bool {
    // Stop the background loop signal
    use std::sync::atomic::Ordering;
    crate::commands::game_boost::PROTECTION_STOP.store(true, Ordering::SeqCst);
    // Reset Rust priority back to Normal (same as revert_game_mode)
    let ps = r#"
        Get-Process | Where-Object { $_.ProcessName -match 'RustClient|rust' } | ForEach-Object {
            try { $_.PriorityClass = 'Normal' } catch {}
        }
    "#;
    run_ps(ps)
}

// ═══════════════════════════════════════════════════════════════
fn run_ps(cmd: &str) -> bool {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_cmd(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

