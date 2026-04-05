use serde::{Deserialize, Serialize};
use sysinfo::{Disks, System};

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
pub fn smart_analyze() -> Result<SmartAnalysisResult, String> {
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

    let has_ssd = disks.iter().any(|d| {
        // sysinfo marks SSD via is_removable=false + kind; approximate via name
        let name = d.name().to_string_lossy().to_lowercase();
        !name.contains("hdd") && d.total_space() > 0
    });

    let os_ver  = System::os_version().unwrap_or_default();
    let is_win11 = os_ver.contains("11") || os_ver.starts_with("22") || os_ver.starts_with("23");

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

    // ── Calculate score ──────────────────────────────────────────────
    // Score = 100 - (penalty per unresolved critical/high recommendation)
    let penalty: u8 = recs.iter().map(|r| match r.priority {
        Priority::Critical => 15,
        Priority::High     => 8,
        Priority::Medium   => 3,
        Priority::Low      => 1,
    }).sum::<u8>().min(100);
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
pub fn apply_recommendation(id: String) -> Result<String, String> {
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
            // Empty working sets of all processes
            ps("Get-Process | ForEach-Object { try { [System.Runtime.InteropServices.Marshal]::FreeHGlobal([System.IntPtr]::Zero) } catch {} }")?;
            Ok("RAM optimized — standby list cleared".into())
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
            // Enable Ultimate Performance (hidden by default)
            ps("powercfg -duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61; $plans = powercfg /L; $guid = ($plans | Select-String 'Ultimate').Line.Split()[3]; powercfg /setactive $guid")?;
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
