use serde::Serialize;
use sysinfo::{System, Disks, Components};

#[derive(Debug, Serialize, Clone)]
pub struct SystemInfoData {
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub cpu_usage: f32,
    pub total_ram_mb: u64,
    pub used_ram_mb: u64,
    pub gpu_info: String,
    pub disks: Vec<DiskInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub free_gb: f64,
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfoData, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_name = if !sys.cpus().is_empty() {
        sys.cpus()[0].brand().to_string()
    } else {
        "Unknown".to_string()
    };

    let cpu_usage: f32 = if !sys.cpus().is_empty() {
        sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32
    } else {
        0.0
    };

    let disks_data = Disks::new_with_refreshed_list();
    let disks: Vec<DiskInfo> = disks_data
        .iter()
        .map(|d| DiskInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            total_gb: d.total_space() as f64 / 1_073_741_824.0,
            free_gb: d.available_space() as f64 / 1_073_741_824.0,
        })
        .collect();

    // Try to get GPU info from Windows registry
    let gpu_info = get_gpu_info_from_registry().unwrap_or_else(|_| "Unknown GPU".to_string());

    Ok(SystemInfoData {
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        cpu_name,
        cpu_cores: sys.cpus().len(),
        cpu_usage,
        total_ram_mb: sys.total_memory() / 1_048_576,
        used_ram_mb: sys.used_memory() / 1_048_576,
        gpu_info,
        disks,
    })
}

#[tauri::command]
pub fn get_realtime_stats() -> Result<RealtimeStats, String> {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpu_usage: f32 = if !sys.cpus().is_empty() {
        sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32
    } else {
        0.0
    };

    // Try to read CPU temperature from system components
    let cpu_temp_c = get_cpu_temp();

    Ok(RealtimeStats {
        cpu_usage,
        ram_used_mb: sys.used_memory() / 1_048_576,
        ram_total_mb: sys.total_memory() / 1_048_576,
        cpu_temp_c,
    })
}

#[derive(Debug, Serialize)]
pub struct RealtimeStats {
    pub cpu_usage: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub cpu_temp_c: Option<f32>,
}

fn get_cpu_temp() -> Option<f32> {
    let components = Components::new_with_refreshed_list();
    let cpu_temps: Vec<f32> = components
        .iter()
        .filter(|c| {
            let label = c.label().to_lowercase();
            label.contains("cpu") || label.contains("core") || label.contains("tdie") || label.contains("tctl")
        })
        .map(|c| c.temperature())
        .collect();

    if cpu_temps.is_empty() {
        None
    } else {
        // Return the max temperature across all CPU cores
        Some(cpu_temps.iter().cloned().fold(f32::NEG_INFINITY, f32::max))
    }
}

fn get_gpu_info_from_registry() -> Result<String, String> {
    // Try registry first (fast, no subprocess)
    if let Ok(gpu) = get_gpu_from_registry_inner() {
        return Ok(gpu);
    }
    // Fallback: wmic — works on all Windows 10/11
    get_gpu_from_wmic()
}

fn get_gpu_from_registry_inner() -> Result<String, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let video_key = hklm
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Video")
        .map_err(|e| e.to_string())?;

    for guid in video_key.enum_keys() {
        if let Ok(guid) = guid {
            let subkey_path = format!(r"SYSTEM\CurrentControlSet\Control\Video\{}\0000", guid);
            if let Ok(device_key) = hklm.open_subkey(&subkey_path) {
                if let Ok(desc) = device_key.get_value::<String, _>("DriverDesc") {
                    if !desc.is_empty() && !desc.contains("Basic") {
                        return Ok(desc);
                    }
                }
            }
        }
    }
    Err("No GPU found in registry".to_string())
}

fn get_gpu_from_wmic() -> Result<String, String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name", "/format:value"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(name) = line.strip_prefix("Name=") {
            let name = name.trim();
            if !name.is_empty() && !name.contains("Basic") {
                return Ok(name.to_string());
            }
        }
    }
    Err("No GPU found via WMIC".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_info() {
        let info = get_system_info();
        assert!(info.is_ok(), "Failed to get system info: {:?}", info.err());
        let data = info.unwrap();
        assert!(!data.os_name.is_empty());
        assert!(data.cpu_cores > 0);
        assert!(data.total_ram_mb > 0);
    }

    #[test]
    fn test_get_realtime_stats() {
        let stats = get_realtime_stats();
        assert!(stats.is_ok(), "Failed to get stats: {:?}", stats.err());
        let data = stats.unwrap();
        assert!(data.ram_total_mb > 0);
    }

    #[test]
    fn test_gpu_info_from_registry() {
        // This may fail on CI but should work on any Windows machine with a GPU
        let result = get_gpu_info_from_registry();
        // We just check it doesn't panic; the result depends on hardware
        println!("GPU result: {:?}", result);
    }
}
