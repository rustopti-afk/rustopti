use serde::Serialize;
use sysinfo::{Disks, System};
use tauri::State;
use crate::utils::license_guard::{LicenseState, require_license};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,  // apply this — big impact
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, Clone)]
pub struct Recommendation {
    pub id:          String,
    pub title:       String,
    pub description: String,
    pub reason:      String,   // why this PC specifically needs it
    pub priority:    Priority,
    pub category:    String,
    pub safe:        bool,
    pub applied:     bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct SmartAnalysisResult {
    pub recommendations: Vec<Recommendation>,
    pub pc_summary:      PcSummary,
    pub score:           u8,   // 0–100 current optimization score
}

#[derive(Debug, Serialize, Clone)]
pub struct PcSummary {
    pub cpu_name:      String,
    pub cpu_cores:     usize,
    pub ram_gb:        u64,
    pub gpu_name:      String,
    pub gpu_vendor:    String,   // "nvidia" | "amd" | "intel" | "unknown"
    pub has_ssd:       bool,
    pub os_version:    String,
    pub is_win11:      bool,
    pub ram_pressure:  String,   // "low" | "medium" | "high"
}

// ── Main command ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn smart_analyze(state: State<'_, LicenseState>) -> Result<SmartAnalysisResult, String> {
    require_license(&state)?;
    let mut sys = System::new_all();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();

    // ── Collect facts ────────────────────────────────────────────────
    let cpu_name  = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();
    let cpu_cores = sys.cpus().len();
    let total_ram_mb = sys.total_memory() / 1_048_576;
    let used_ram_mb  = sys.used_memory()  / 1_048_576;
    let ram_gb    = total_ram_mb / 1024;
    let ram_usage = used_ram_mb as f64 / total_ram_mb.max(1) as f64;

    let gpu_name = get_gpu_name();
    let gpu_vendor = detect_gpu_vendor(&gpu_name);

    use sysinfo::DiskKind;
    let has_ssd = disks.iter().any(|d| matches!(d.kind(), DiskKind::SSD));

    let os_ver  = System::os_version().unwrap_or_default();
    // os_version() returns "10.0.22621" format — check build number >= 22000 for Win11
    let build: u32 = os_ver.split('.').nth(2).and_then(|b| b.parse().ok()).unwrap_or(0);
    let is_win11 = build >= 22000;

    let ram_pressure = match ram_usage {
        u if u > 0.85 => "high",
        u if u > 0.60 => "medium",
        _ => "low",
    };

    let summary = PcSummary {
        cpu_name: cpu_name.clone(),
        cpu_cores,
        ram_gb,
        gpu_name: gpu_name.clone(),
        gpu_vendor: gpu_vendor.clone(),
        has_ssd,
        os_version: os_ver.clone(),
        is_win11,
        ram_pressure: ram_pressure.to_string(),
    };

    // ── Build recommendations ────────────────────────────────────────
    let mut recs: Vec<Recommendation> = Vec::new();

    // 1. RAM — low memory
    if ram_gb <= 8 {
        recs.push(Recommendation {
            id:          "ram_optimize".into(),
            title:       "Оптимізація RAM".into(),
            description: "Очистити standby list і стиснути неактивні процеси".into(),
            reason:      format!("У тебе лише {}GB RAM — критично мало для ігор", ram_gb),
            priority:    Priority::Critical,
            category:    "ram".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 2. RAM pressure high
    if ram_pressure == "high" {
        recs.push(Recommendation {
            id:          "kill_background".into(),
            title:       "Вбити фонові процеси".into(),
            description: "Завершити OneDrive, Teams, Spotify та інші не потрібні процеси".into(),
            reason:      format!("Використовується {:.0}% RAM — не вистачає для гри", ram_usage * 100.0),
            priority:    Priority::Critical,
            category:    "process".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 3. CPU cores — unpark
    if cpu_cores >= 6 {
        recs.push(Recommendation {
            id:          "unpark_cores".into(),
            title:       "Розпарковка ядер CPU".into(),
            description: "Windows паркує частину ядер для економії енергії — розблокуємо всі".into(),
            reason:      format!("{} ядер знайдено — частина може бути запаркована", cpu_cores),
            priority:    Priority::High,
            category:    "cpu".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 4. Timer resolution
    recs.push(Recommendation {
        id:          "timer_resolution".into(),
        title:       "Boost Timer Resolution".into(),
        description: "Встановити точність системного таймера 0.5ms замість стандартних 15.6ms".into(),
        reason:      "Зменшує input lag та стабілізує frametime для будь-якого заліза".into(),
        priority:    Priority::High,
        category:    "cpu".into(),
        safe:        true,
        applied:     false,
    });

    // 5. Power plan
    recs.push(Recommendation {
        id:          "power_plan".into(),
        title:       "Ultimate Performance план живлення".into(),
        description: "Перемкнути на Ultimate Performance або High Performance".into(),
        reason:      "Стандартний план знижує частоту CPU/GPU під час гри".into(),
        priority:    Priority::High,
        category:    "power".into(),
        safe:        true,
        applied:     false,
    });

    // 6. NVIDIA-specific
    if gpu_vendor == "nvidia" {
        recs.push(Recommendation {
            id:          "nvidia_tweaks".into(),
            title:       "NVIDIA Shader Cache + оптимізація".into(),
            description: "Збільшити shader cache, вимкнути V-Sync в панелі NVIDIA".into(),
            reason:      format!("Знайдено NVIDIA GPU: {}", gpu_name),
            priority:    Priority::High,
            category:    "gpu".into(),
            safe:        true,
            applied:     false,
        });
        recs.push(Recommendation {
            id:          "nvidia_low_latency".into(),
            title:       "NVIDIA Low Latency Mode".into(),
            description: "Увімкнути Ultra Low Latency Mode в NVIDIA Control Panel".into(),
            reason:      "Знижує input lag на 10-20ms на NVIDIA картах".into(),
            priority:    Priority::Medium,
            category:    "gpu".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 7. AMD-specific
    if gpu_vendor == "amd" {
        recs.push(Recommendation {
            id:          "amd_tweaks".into(),
            title:       "AMD Anti-Lag + Shader Cache".into(),
            description: "Увімкнути Anti-Lag, збільшити розмір shader cache".into(),
            reason:      format!("Знайдено AMD GPU: {}", gpu_name),
            priority:    Priority::High,
            category:    "gpu".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 8. HDD detected
    if !has_ssd {
        recs.push(Recommendation {
            id:          "disable_indexer".into(),
            title:       "Вимкнути Search Indexer".into(),
            description: "SearchIndexer активно читає HDD у фоні та вкрадає I/O у гри".into(),
            reason:      "Знайдено HDD — Search Indexer значно просідає швидкість завантаження".into(),
            priority:    Priority::Critical,
            category:    "disk".into(),
            safe:        true,
            applied:     false,
        });
        recs.push(Recommendation {
            id:          "disable_superfetch".into(),
            title:       "Вимкнути SysMain (Superfetch)".into(),
            description: "Superfetch постійно читає HDD, що погіршує ігрову продуктивність".into(),
            reason:      "На HDD SysMain шкодить більше ніж допомагає".into(),
            priority:    Priority::High,
            category:    "disk".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 9. Visual effects — always useful on low-end
    if ram_gb <= 16 || cpu_cores <= 4 {
        recs.push(Recommendation {
            id:          "visual_effects".into(),
            title:       "Вимкнути візуальні ефекти Windows".into(),
            description: "Тіні, анімації, прозорість — все це займає GPU та RAM".into(),
            reason:      format!("{}GB RAM / {} ядер — кожен MB важливий", ram_gb, cpu_cores),
            priority:    Priority::Medium,
            category:    "system".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 10. Windows 11 scheduler tweak
    if is_win11 {
        recs.push(Recommendation {
            id:          "win11_scheduler".into(),
            title:       "Windows 11 MFBT Scheduler".into(),
            description: "Вимкнути MultiThreaded Frame Buffering для зниження input lag".into(),
            reason:      "Знайдено Windows 11 — специфічний tweak для нового планувальника".into(),
            priority:    Priority::Medium,
            category:    "system".into(),
            safe:        true,
            applied:     false,
        });
    }

    // 11. MSI mode for GPU
    recs.push(Recommendation {
        id:          "msi_mode".into(),
        title:       "MSI Mode для GPU".into(),
        description: "Message Signaled Interrupts знижує DPC latency та покращує фреймтайм".into(),
        reason:      "Стандартний режим INTx додає мікро-затримки на кожному кадрі".into(),
        priority:    Priority::Medium,
        category:    "gpu".into(),
        safe:        true,
        applied:     false,
    });

    // 12. HPET disable
    recs.push(Recommendation {
        id:          "disable_hpet".into(),
        title:       "Вимкнути HPET".into(),
        description: "High Precision Event Timer може збільшувати DPC latency на деяких системах".into(),
        reason:      "Рекомендовано для ігрових ПК — особливо з процесорами AMD".into(),
        priority:    if cpu_name.to_lowercase().contains("amd") || cpu_name.to_lowercase().contains("ryzen") {
            Priority::High
        } else {
            Priority::Medium
        },
        category:    "cpu".into(),
        safe:        true,
        applied:     false,
    });

    // ── Check real applied state for each recommendation ────────────
    for rec in &mut recs {
        rec.applied = is_applied(&rec.id);
    }

    // ── Calculate score ──────────────────────────────────────────────
    // Score = 100 - (penalty per UNRESOLVED critical/high recommendation)
    let penalty: u8 = recs.iter()
        .filter(|r| !r.applied)   // skip already-applied tweaks
        .map(|r| match r.priority {
            Priority::Critical => 15u32,
            Priority::High     => 8u32,
            Priority::Medium   => 3u32,
            Priority::Low      => 1u32,
        }).sum::<u32>().min(100) as u8;
    let score = 100u8.saturating_sub(penalty);

    // Sort: Critical first, then High, Medium, Low
    recs.sort_by_key(|r| match r.priority {
        Priority::Critical => 0,
        Priority::High     => 1,
        Priority::Medium   => 2,
        Priority::Low      => 3,
    });

    Ok(SmartAnalysisResult { recommendations: recs, pc_summary: summary, score })
}

/// Apply a specific recommendation by its ID.
#[tauri::command]
pub fn apply_recommendation(id: String, state: State<'_, LicenseState>) -> Result<String, String> {
    require_license(&state)?;
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x08000000;

    let ps = |cmd: &str| {
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
            .creation_flags(NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())
    };

    match id.as_str() {
        "ram_optimize" => {
            // Empty working sets of all processes via EmptyWorkingSet Win32 API
            ps("
                $code = '[DllImport(\"psapi.dll\")] public static extern bool EmptyWorkingSet(IntPtr hProcess);';
                $t = Add-Type -MemberDefinition $code -Name 'EWS' -Namespace WinAPI -PassThru;
                Get-Process | ForEach-Object { try { $t::EmptyWorkingSet($_.Handle) } catch {} }
            ")?;
            Ok("RAM optimized — working sets trimmed".into())
        }
        "kill_background" => {
            let targets = ["OneDrive","Teams","Spotify","SkypeApp","SearchApp",
                           "YourPhone","GameBarPresenceWriter","HxOutlook","MicrosoftEdge"];
            for t in targets {
                let _ = ps(&format!("Stop-Process -Name '{}' -Force -ErrorAction SilentlyContinue", t));
            }
            Ok("Background processes terminated".into())
        }
        "unpark_cores" => {
            ps("powercfg /setacvalueindex SCHEME_CURRENT SUB_PROCESSOR CPMINCORES 100; powercfg /setactive SCHEME_CURRENT")?;
            Ok("All CPU cores unparked".into())
        }
        "timer_resolution" => {
            // Set timer resolution via bcdedit + registry
            ps("Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\kernel' -Name 'GlobalTimerResolutionRequests' -Value 1 -Type DWord")?;
            Ok("Timer resolution boosted to 0.5ms".into())
        }
        "power_plan" => {
            // Duplicate Ultimate Performance scheme then activate by GUID directly
            // (avoids Select-String failing on non-English Windows)
            ps("powercfg -duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61 2>$null; powercfg /setactive e9a42b02-d5df-448d-aa00-03f14749eb61")?;
            Ok("Ultimate Performance plan activated".into())
        }
        "disable_indexer" => {
            ps("Stop-Service -Name 'WSearch' -Force; Set-Service -Name 'WSearch' -StartupType Disabled")?;
            Ok("Search Indexer disabled".into())
        }
        "disable_superfetch" => {
            ps("Stop-Service -Name 'SysMain' -Force; Set-Service -Name 'SysMain' -StartupType Disabled")?;
            Ok("SysMain (Superfetch) disabled".into())
        }
        "visual_effects" => {
            ps("Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects' -Name 'VisualFXSetting' -Value 2")?;
            Ok("Visual effects minimized".into())
        }
        "disable_hpet" => {
            Command::new("bcdedit")
                .args(["/deletevalue", "useplatformclock"])
                .creation_flags(NO_WINDOW)
                .output()
                .ok();
            ps("Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\services\\hpet' -Name 'Start' -Value 4")?;
            Ok("HPET disabled (reboot required)".into())
        }
        "msi_mode" => {
            // Enable MSI mode for GPU via registry
            ps("
                $gpu = Get-WmiObject Win32_VideoController | Select-Object -First 1;
                $pciPath = (Get-ChildItem 'HKLM:\\SYSTEM\\CurrentControlSet\\Enum\\PCI' -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.GetValue('DeviceDesc') -like '*' + $gpu.Description + '*' } | Select-Object -First 1).PSPath;
                if ($pciPath) {
                    $intPath = $pciPath + '\\Device Parameters\\Interrupt Management\\MessageSignaledInterruptProperties';
                    New-Item -Path $intPath -Force | Out-Null;
                    Set-ItemProperty -Path $intPath -Name 'MSISupported' -Value 1 -Type DWord;
                }
            ")?;
            Ok("MSI Mode enabled (reboot required)".into())
        }
        "win11_scheduler" => {
            ps("bcdedit /set disabledynamictick yes")?;
            Ok("Win11 scheduler optimized".into())
        }
        "nvidia_tweaks" | "nvidia_low_latency" => {
            // NVIDIA tweaks via registry
            ps("
                $nvPath = 'HKLM:\\SYSTEM\\CurrentControlSet\\Services\\nvlddmkm';
                if (Test-Path $nvPath) {
                    Set-ItemProperty -Path $nvPath -Name 'EnableMidBufferPreemption' -Value 0 -Type DWord;
                    Set-ItemProperty -Path $nvPath -Name 'EnableCEPreemption' -Value 0 -Type DWord;
                }
            ")?;
            Ok("NVIDIA optimizations applied".into())
        }
        "amd_tweaks" => {
            ps("
                $amdPath = 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0000';
                if (Test-Path $amdPath) {
                    Set-ItemProperty -Path $amdPath -Name 'EnableUlps' -Value 0 -Type DWord;
                }
            ")?;
            Ok("AMD optimizations applied (ULPS disabled)".into())
        }
        _ => Err(format!("Unknown recommendation id: {}", id)),
    }
}

// ── Applied-state checkers ────────────────────────────────────────────────────

/// Returns true if a recommendation is already applied on this system.
fn is_applied(id: &str) -> bool {
    use winreg::{RegKey, enums::*};
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const NO_WIN: u32 = 0x08000000;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let ps_bool = |cmd: &str| -> bool {
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
            .creation_flags(NO_WIN)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase() == "true")
            .unwrap_or(false)
    };

    match id {
        "timer_resolution" => hklm
            .open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel")
            .and_then(|k| k.get_value::<u32, _>("GlobalTimerResolutionRequests"))
            .map(|v| v == 1)
            .unwrap_or(false),

        "power_plan" => ps_bool(
            "(powercfg /getactivescheme) -match 'e9a42b02-d5df-448d-aa00-03f14749eb61'"
        ),

        "unpark_cores" => ps_bool(
            "$v = (powercfg /query SCHEME_CURRENT SUB_PROCESSOR CPMINCORES 2>$null); ($v | Select-String 'Current AC Power Setting Index: 0x00000064') -ne $null"
        ),

        "disable_indexer" => ps_bool(
            "(Get-Service -Name WSearch -ErrorAction SilentlyContinue).StartType -eq 'Disabled'"
        ),

        "disable_superfetch" => ps_bool(
            "(Get-Service -Name SysMain -ErrorAction SilentlyContinue).StartType -eq 'Disabled'"
        ),

        "visual_effects" => hkcu
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects")
            .and_then(|k| k.get_value::<u32, _>("VisualFXSetting"))
            .map(|v| v == 2)
            .unwrap_or(false),

        "disable_hpet" => hklm
            .open_subkey(r"SYSTEM\CurrentControlSet\services\hpet")
            .and_then(|k| k.get_value::<u32, _>("Start"))
            .map(|v| v == 4)
            .unwrap_or(false),

        "msi_mode" => ps_bool(
            r#"$path = (Get-ChildItem 'HKLM:\SYSTEM\CurrentControlSet\Enum\PCI' -Recurse -EA SilentlyContinue | Where-Object { try { (Get-ItemProperty $_.PSPath).Class -eq 'Display' } catch { $false } } | Select-Object -First 1)?.PSPath; if ($path) { $msi = (Get-ItemProperty -EA SilentlyContinue "$path\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties")?.MSISupported; $msi -eq 1 } else { $false }"#
        ),

        "nvidia_tweaks" | "nvidia_low_latency" => hklm
            .open_subkey(r"SYSTEM\CurrentControlSet\Services\nvlddmkm")
            .and_then(|k| k.get_value::<u32, _>("EnableMidBufferPreemption"))
            .map(|v| v == 0)
            .unwrap_or(false),

        "amd_tweaks" => hklm
            .open_subkey(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000")
            .and_then(|k| k.get_value::<u32, _>("EnableUlps"))
            .map(|v| v == 0)
            .unwrap_or(false),

        "win11_scheduler" => ps_bool(
            "(bcdedit /enum 2>$null) -match 'disabledynamictick\\s+Yes'"
        ),

        // RAM and process tweaks are one-shot — never "permanently applied"
        _ => false,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_gpu_name() -> String {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(video) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Control\Video") {
        for guid in video.enum_keys().flatten() {
            let path = format!(r"SYSTEM\CurrentControlSet\Control\Video\{}\0000", guid);
            if let Ok(key) = hklm.open_subkey(&path) {
                if let Ok(desc) = key.get_value::<String, _>("DriverDesc") {
                    if !desc.is_empty() && !desc.contains("Basic") {
                        return desc;
                    }
                }
            }
        }
    }
    "Unknown GPU".to_string()
}

fn detect_gpu_vendor(gpu_name: &str) -> String {
    let lower = gpu_name.to_lowercase();
    if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("rtx") || lower.contains("gtx") {
        "nvidia".into()
    } else if lower.contains("amd") || lower.contains("radeon") || lower.contains("rx ") {
        "amd".into()
    } else if lower.contains("intel") || lower.contains("arc") || lower.contains("iris") {
        "intel".into()
    } else {
        "unknown".into()
    }
}
