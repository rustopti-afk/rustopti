use serde::Serialize;
use tauri::State;
use sysinfo::{System, Pid};
use std::os::windows::process::CommandExt;
use crate::utils::license_guard::{LicenseState, require_license};
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: u64,
}

/// Well-known bloatware processes safe to kill for gaming
const BLOATWARE: &[&str] = &[
    "OneDrive.exe", "Cortana.exe", "SearchApp.exe", "YourPhone.exe",
    "GameBarPresenceWriter.exe", "GameBar.exe", "SkypeApp.exe",
    "MicrosoftEdgeUpdate.exe", "Teams.exe", "Widgets.exe",
    "PhoneExperienceHost.exe", "HxTsr.exe", "HxOutlook.exe",
];

#[tauri::command]
pub fn get_process_list() -> Result<Vec<ProcessInfo>, String> {
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(100));
    sys.refresh_all();

    let mut procs: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, p)| ProcessInfo {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_mb: p.memory() / 1_048_576,
        })
        .collect();

    procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    Ok(procs)
}

#[tauri::command]
pub fn kill_process(pid: u32, state: State<'_, LicenseState>) -> Result<String, String> {
    require_license(&state)?;
    let sys = System::new_all();
    let pid_obj = Pid::from_u32(pid);
    if let Some(process) = sys.process(pid_obj) {
        let name = process.name().to_string_lossy().to_string();
        if process.kill() {
            Ok(format!("✓ Killed {} (PID: {})", name, pid))
        } else {
            Err(format!("✗ Failed to kill {} (PID: {})", name, pid))
        }
    } else {
        Err(format!("✗ Process with PID {} not found", pid))
    }
}

#[tauri::command]
pub fn kill_bloatware(state: State<'_, LicenseState>) -> Result<Vec<String>, String> {
    require_license(&state)?;
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut log = Vec::new();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();
        if BLOATWARE.contains(&name.as_str()) {
            if process.kill() {
                log.push(format!("✓ Killed {} (PID: {})", name, pid.as_u32()));
            } else {
                log.push(format!("✗ Failed: {} (PID: {})", name, pid.as_u32()));
            }
        }
    }

    if log.is_empty() {
        log.push("✓ No bloatware processes found".to_string());
    }
    Ok(log)
}

#[tauri::command]
pub fn set_process_priority(pid: u32, priority: String, state: State<'_, LicenseState>) -> Result<String, String> {
    require_license(&state)?;
    use std::process::Command;

    let _priority_value = match priority.to_lowercase().as_str() {
        "realtime" => "256",
        "high" => "128",
        "above_normal" => "32768",
        "normal" => "32",
        "below_normal" => "16384",
        "idle" | "low" => "64",
        _ => return Err("Invalid priority. Use: realtime, high, above_normal, normal, below_normal, idle".to_string()),
    };

    let ps_cmd = format!(
        "Get-Process -Id {} | ForEach-Object {{ $_.PriorityClass = '{}' }}",
        pid,
        match priority.to_lowercase().as_str() {
            "realtime" => "RealTime",
            "high" => "High",
            "above_normal" => "AboveNormal",
            "normal" => "Normal",
            "below_normal" => "BelowNormal",
            "idle" | "low" => "Idle",
            _ => "Normal",
        }
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed: {}", e))?;

    if output.status.success() {
        Ok(format!("✓ PID {} priority set to {}", pid, priority))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("✗ Failed to set priority: {}", stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_process_list() {
        let procs = get_process_list();
        assert!(procs.is_ok());
        let list = procs.unwrap();
        assert!(!list.is_empty(), "Process list should not be empty");
    }

    #[test]
    fn test_bloatware_list_not_empty() {
        assert!(!BLOATWARE.is_empty());
    }

    #[test]
    fn test_kill_nonexistent_process() {
        let result = kill_process(999999);
        assert!(result.is_err());
    }
}
