use serde::Serialize;
use tauri::State;
use std::process::Command;
use std::os::windows::process::CommandExt;
const CREATE_NO_WINDOW: u32 = 0x08000000;
use crate::utils::registry_helper;
use crate::utils::license_guard::{LicenseState, require_license};
use winreg::enums::*;
use winreg::HKEY;

#[derive(Debug, Serialize)]
pub struct NetworkResult {
    pub name: String,
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub fn get_network_status() -> Result<Vec<NetworkStatusItem>, String> {
    let mut items = Vec::new();

    // Check Nagle Algorithm
    items.push(NetworkStatusItem {
        name: "Nagle Algorithm".to_string(),
        description: "Reduces latency by disabling packet buffering".to_string(),
        optimized: check_nagle_disabled(),
    });

    // Check Network Throttling
    let throttle = registry_helper::get_dword(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
        "NetworkThrottlingIndex",
    );
    items.push(NetworkStatusItem {
        name: "Network Throttling".to_string(),
        description: "Removes Windows bandwidth limitations".to_string(),
        optimized: throttle.map(|v| v == 0xFFFFFFFF).unwrap_or(false),
    });

    Ok(items)
}

#[derive(Debug, Serialize)]
pub struct NetworkStatusItem {
    pub name: String,
    pub description: String,
    pub optimized: bool,
}

#[tauri::command]
pub fn apply_network_tweaks(state: State<'_, LicenseState>) -> Result<Vec<NetworkResult>, String> {
    require_license(&state)?;
    crate::utils::cleanup::register_tweak("network_tweaks");
    let mut results = Vec::new();

    // 1. Disable Nagle Algorithm on all interfaces
    results.push(disable_nagle());

    // 2. Disable Network Throttling
    results.push(apply_net_registry(
        "Disable Network Throttling",
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
        "NetworkThrottlingIndex",
        0xFFFFFFFF,
    ));

    // 3. TCP optimizations
    results.push(apply_net_registry(
        "TCP Auto-Tuning (Normal)",
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\AFD\Parameters",
        "DefaultReceiveWindow", 65536,
    ));

    results.push(apply_net_registry(
        "TCP No Delay",
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
        "TcpNoDelay", 1,
    ));

    // 4. DNS Flush
    results.push(flush_dns());

    // 5. Disable Large Send Offload
    results.push(apply_net_registry(
        "Disable Large Send Offload",
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
        "DisableTaskOffload", 1,
    ));

    Ok(results)
}

fn disable_nagle() -> NetworkResult {
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let interfaces_path = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

    match hklm.open_subkey(interfaces_path) {
        Ok(interfaces_key) => {
            let mut count = 0;
            for iface in interfaces_key.enum_keys() {
                if let Ok(iface_name) = iface {
                    let path = format!(r"{}\{}", interfaces_path, iface_name);
                    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &path, "TcpAckFrequency", 1);
                    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &path, "TCPNoDelay", 1);
                    let _ = registry_helper::set_dword(HKEY_LOCAL_MACHINE, &path, "TcpDelAckTicks", 0);
                    count += 1;
                }
            }
            NetworkResult {
                name: "Disable Nagle Algorithm".to_string(),
                success: true,
                message: format!("✓ Nagle disabled on {} interfaces", count),
            }
        }
        Err(e) => NetworkResult {
            name: "Disable Nagle Algorithm".to_string(),
            success: false,
            message: format!("✗ Failed: {}", e),
        },
    }
}

fn flush_dns() -> NetworkResult {
    match Command::new("ipconfig").args(["/flushdns"]).creation_flags(CREATE_NO_WINDOW).output() {
        Ok(o) if o.status.success() => NetworkResult {
            name: "DNS Flush".to_string(),
            success: true,
            message: "✓ DNS cache flushed".to_string(),
        },
        Ok(_) => NetworkResult {
            name: "DNS Flush".to_string(),
            success: false,
            message: "✗ DNS flush failed".to_string(),
        },
        Err(e) => NetworkResult {
            name: "DNS Flush".to_string(),
            success: false,
            message: format!("✗ {}", e),
        },
    }
}

fn apply_net_registry(name: &str, hkey: HKEY, subkey: &str, value: &str, data: u32) -> NetworkResult {
    match registry_helper::set_dword(hkey, subkey, value, data) {
        Ok(_) => NetworkResult {
            name: name.to_string(),
            success: true,
            message: format!("✓ {}", name),
        },
        Err(e) => NetworkResult {
            name: name.to_string(),
            success: false,
            message: format!("✗ {} — {}", name, e),
        },
    }
}

fn check_nagle_disabled() -> bool {
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let interfaces_path = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

    if let Ok(interfaces_key) = hklm.open_subkey(interfaces_path) {
        for iface in interfaces_key.enum_keys() {
            if let Ok(iface_name) = iface {
                let path = format!(r"{}\{}", interfaces_path, iface_name);
                if let Ok(val) = registry_helper::get_dword(HKEY_LOCAL_MACHINE, &path, "TCPNoDelay") {
                    if val == 1 {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_network_status() {
        let status = get_network_status();
        assert!(status.is_ok());
        let items = status.unwrap();
        assert!(items.len() >= 2);
    }

    #[test]
    fn test_check_nagle() {
        // Just ensure it doesn't panic
        let _ = check_nagle_disabled();
    }
}
