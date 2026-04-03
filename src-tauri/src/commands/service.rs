use std::process::Command;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Launch rustopti-service.exe in background.
/// Service keeps Timer Resolution active and monitors subscription.
#[tauri::command]
pub fn launch_service() -> Result<(), String> {
    // Service exe is next to main exe
    let exe_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot find exe dir")?
        .to_path_buf();

    let service_path = exe_dir.join("rustopti-service.exe");

    if !service_path.exists() {
        return Err("Service executable not found".to_string());
    }

    Command::new(&service_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to launch service: {}", e))?;

    Ok(())
}
