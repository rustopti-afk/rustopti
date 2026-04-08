use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::State;
use crate::utils::license_guard::{LicenseState, require_license};

const NO_WIN: u32 = 0x08000000;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TweakResult {
    pub id:         String,
    pub name:       String,
    pub gpu_before: f64,
    pub gpu_after:  f64,
    pub gain_pct:   f64,
    pub kept:       bool,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct AdaptiveStatus {
    pub running:            bool,
    pub phase:              String,   // "idle"|"baseline"|"testing"|"done"
    pub current_tweak_name: String,
    pub current_tweak_idx:  usize,
    pub total_tweaks:       usize,
    pub baseline_gpu:       f64,
    pub current_gpu:        f64,
    pub progress_pct:       u8,
    pub results:            Vec<TweakResult>,
    pub game_name:          String,
    pub applied_count:      usize,
    pub message:            String,
}

// AdaptiveState holds Arc so it can be cloned into threads safely (no raw pointers)
pub struct AdaptiveState {
    pub status: Arc<Mutex<AdaptiveStatus>>,
    pub stop:   Arc<AtomicBool>,
}

impl Default for AdaptiveState {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(AdaptiveStatus::default())),
            stop:   Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdaptiveProfile {
    pub game_name: String,
    pub opt_id:    String,
    pub enabled:   bool,
    pub fps_gain:  f64,
}

// ── Tweaks registry ───────────────────────────────────────────────────────────
// Each entry: (id, display_name)
// All tweaks here MUST be fully reversible (no process kills, no reboots)

struct Tweak {
    id:    &'static str,
    name:  &'static str,
    apply:  fn(u32) -> Result<(), String>,   // game_pid passed in case tweak needs it
    revert: fn()    -> Result<(), String>,
}

fn ps(cmd: &str) -> Result<(), String> {
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
        .creation_flags(NO_WIN)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        if !err.trim().is_empty() { return Err(err); }
    }
    Ok(())
}

fn tweaks_list() -> Vec<Tweak> {
    vec![
        Tweak {
            id:   "game_priority",
            name: "Пріоритет процесу HIGH",
            apply: |pid| ps(&format!(
                "(Get-Process -Id {pid}).PriorityClass = 'High'"
            )),
            revert: || Ok(()), // priority resets when game closes
        },
        Tweak {
            id:   "timer_resolution",
            name: "Таймер 0.5ms",
            apply: |_| ps(
                "Set-ItemProperty -Path \
                'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\kernel' \
                -Name GlobalTimerResolutionRequests -Value 1 -Type DWord -Force"
            ),
            revert: || ps(
                "Set-ItemProperty -Path \
                'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\kernel' \
                -Name GlobalTimerResolutionRequests -Value 0 -Type DWord -Force"
            ),
        },
        Tweak {
            id:   "power_plan",
            name: "Ultimate Performance",
            apply: |_| ps(
                "powercfg -duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61 2>$null; \
                 powercfg /setactive e9a42b02-d5df-448d-aa00-03f14749eb61"
            ),
            revert: || ps(
                "powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e"
            ),
        },
        Tweak {
            id:   "unpark_cores",
            name: "CPU Unpark ядер",
            apply: |_| ps(
                "powercfg /setacvalueindex SCHEME_CURRENT SUB_PROCESSOR CPMINCORES 100; \
                 powercfg /setactive SCHEME_CURRENT"
            ),
            revert: || ps(
                "powercfg /setacvalueindex SCHEME_CURRENT SUB_PROCESSOR CPMINCORES 5; \
                 powercfg /setactive SCHEME_CURRENT"
            ),
        },
        Tweak {
            id:   "disable_sysmain",
            name: "Вимкнути SysMain",
            apply: |_| ps(
                "Stop-Service -Name SysMain -Force -ErrorAction SilentlyContinue"
            ),
            revert: || ps(
                "Start-Service -Name SysMain -ErrorAction SilentlyContinue"
            ),
        },
        Tweak {
            id:   "visual_effects",
            name: "Мінімум візуальних ефектів",
            apply: |_| ps(
                "Set-ItemProperty -Path \
                'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects' \
                -Name VisualFXSetting -Value 2 -Force"
            ),
            revert: || ps(
                "Set-ItemProperty -Path \
                'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects' \
                -Name VisualFXSetting -Value 0 -Force"
            ),
        },
        Tweak {
            id:   "registry_gaming",
            name: "Реєстр: GameDVR / GPU Priority",
            apply: |_| ps(
                "Set-ItemProperty -Path \
                'HKCU:\\System\\GameConfigStore' \
                -Name GameDVR_Enabled -Value 0 -Type DWord -Force; \
                Set-ItemProperty -Path \
                'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games' \
                -Name 'GPU Priority' -Value 8 -Type DWord -Force; \
                Set-ItemProperty -Path \
                'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games' \
                -Name 'Priority' -Value 6 -Type DWord -Force"
            ),
            revert: || ps(
                "Set-ItemProperty -Path \
                'HKCU:\\System\\GameConfigStore' \
                -Name GameDVR_Enabled -Value 1 -Type DWord -Force; \
                Set-ItemProperty -Path \
                'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games' \
                -Name 'GPU Priority' -Value 2 -Type DWord -Force; \
                Set-ItemProperty -Path \
                'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games' \
                -Name 'Priority' -Value 2 -Type DWord -Force"
            ),
        },
    ]
}

// ── GPU util measurement (proxy for FPS) ──────────────────────────────────────

fn measure_gpu(game_pid: u32, secs: u32) -> f64 {
    let script = format!(
        r#"try {{
            $samples = (Get-Counter '\GPU Engine(*engtype_3D)\Utilization Percentage' `
                -SampleInterval 1 -MaxSamples {secs} -ErrorAction SilentlyContinue).CounterSamples
            $pid_samples = $samples | Where-Object {{ $_.Path -like '*pid_{pid}*' }}
            if ($pid_samples) {{
                [math]::Round(($pid_samples | Measure-Object -Property CookedValue -Average).Average, 2)
            }} else {{
                [math]::Round(($samples | Measure-Object -Property CookedValue -Average).Average, 2)
            }}
        }} catch {{ 0 }}"#,
        secs = secs,
        pid = game_pid
    );
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .creation_flags(NO_WIN)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

// ── DB ────────────────────────────────────────────────────────────────────────

fn db_path() -> String {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    format!("{}\\NexOpti\\gamemode.db", appdata)
}

fn open_db() -> Result<Connection, String> {
    let path = db_path();
    if let Some(p) = std::path::Path::new(&path).parent() { let _ = std::fs::create_dir_all(p); }
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS adaptive_profiles (
            game_name TEXT NOT NULL,
            opt_id    TEXT NOT NULL,
            enabled   INTEGER DEFAULT 0,
            fps_gain  REAL DEFAULT 0,
            PRIMARY KEY (game_name, opt_id)
        );"
    ).map_err(|e| e.to_string())?;
    Ok(conn)
}

fn save_profile(game_name: &str, results: &[TweakResult]) -> Result<(), String> {
    let conn = open_db()?;
    for r in results {
        conn.execute(
            "INSERT INTO adaptive_profiles (game_name, opt_id, enabled, fps_gain)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(game_name, opt_id) DO UPDATE SET enabled=?3, fps_gain=?4",
            params![game_name, r.id, r.kept as i64, r.gain_pct],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_profile_db(game_name: &str) -> Result<Vec<AdaptiveProfile>, String> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT game_name, opt_id, enabled, fps_gain
         FROM adaptive_profiles WHERE game_name = ?1 ORDER BY fps_gain DESC"
    ).map_err(|e| e.to_string())?;

    let profiles = stmt.query_map(params![game_name], |row| {
        Ok(AdaptiveProfile {
            game_name: row.get(0)?,
            opt_id:    row.get(1)?,
            enabled:   row.get::<_, i64>(2)? == 1,
            fps_gain:  row.get(3)?,
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(profiles)
}

// ── Adaptive session thread ───────────────────────────────────────────────────

fn adaptive_session(
    game_name: String,
    game_pid: u32,
    status: Arc<Mutex<AdaptiveStatus>>,
    stop: Arc<AtomicBool>,
) {
    let tweaks = tweaks_list();
    let total = tweaks.len();

    macro_rules! update {
        ($($field:ident : $val:expr),* $(,)?) => {{
            if let Ok(mut s) = status.lock() {
                $(s.$field = $val;)*
            }
        }};
    }

    macro_rules! stopped { () => { stop.load(Ordering::Relaxed) } }

    update!(
        running: true,
        phase: "baseline".into(),
        game_name: game_name.clone(),
        total_tweaks: total,
        message: "Вимірюємо базовий FPS...".into()
    );

    // Measure baseline (15s)
    if stopped!() { cleanup_and_exit(&status); return; }
    let baseline = measure_gpu(game_pid, 15);
    update!(baseline_gpu: baseline, current_gpu: baseline,
            message: format!("Baseline: {:.1}%", baseline));

    if baseline < 1.0 {
        update!(running: false, phase: "done".into(),
                message: "Не вдалося виміряти GPU — запусти гру і спробуй ще раз".into());
        return;
    }

    let mut results: Vec<TweakResult> = Vec::new();

    for (i, tweak) in tweaks.iter().enumerate() {
        if stopped!() { break; }

        let progress = ((i as f32 / total as f32) * 100.0) as u8;
        update!(
            phase: "testing".into(),
            current_tweak_name: tweak.name.to_string(),
            current_tweak_idx: i + 1,
            progress_pct: progress,
            message: format!("Тестуємо: {}...", tweak.name)
        );

        // Apply tweak
        let _ = (tweak.apply)(game_pid);

        // Wait 8s to stabilize
        for _ in 0..80 {
            if stopped!() { break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if stopped!() { break; }

        // Measure with tweak (10s)
        let after = measure_gpu(game_pid, 10);
        update!(current_gpu: after);

        let gain = if baseline > 0.1 {
            ((after - baseline) / baseline * 100.0 * 10.0).round() / 10.0
        } else { 0.0 };

        let kept = gain >= 2.0; // keep if +2% or more GPU utilization

        if !kept {
            // Revert — this tweak didn't help
            let _ = (tweak.revert)();
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        let result = TweakResult {
            id:         tweak.id.to_string(),
            name:       tweak.name.to_string(),
            gpu_before: baseline,
            gpu_after:  after,
            gain_pct:   gain,
            kept,
        };

        if let Ok(mut s) = status.lock() {
            s.results.push(result.clone());
            s.applied_count = s.results.iter().filter(|r| r.kept).count();
        }

        results.push(result);

        // Small pause between tweaks
        for _ in 0..20 {
            if stopped!() { break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    // Save profile
    let _ = save_profile(&game_name, &results);

    let applied = results.iter().filter(|r| r.kept).count();
    update!(
        running: false,
        phase: "done".into(),
        progress_pct: 100,
        message: format!(
            "Готово! {} з {} покращень дали приріст FPS — профіль збережено для {}",
            applied, total, game_name
        )
    );
}

fn cleanup_and_exit(status: &Arc<Mutex<AdaptiveStatus>>) {
    if let Ok(mut s) = status.lock() {
        s.running = false;
        s.phase = "idle".into();
        s.message = "Зупинено".into();
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Start adaptive FPS tuning for a running game.
/// Runs in background — poll get_adaptive_status() for progress.
#[tauri::command]
pub fn start_adaptive_session(
    game_name: String,
    game_pid: u32,
    state: State<'_, AdaptiveState>,
    license: State<'_, LicenseState>,
) -> Result<String, String> {
    require_license(&license)?;

    if state.status.lock().map_err(|e| e.to_string())?.running {
        return Err("Adaptive session already running".into());
    }

    // Reset stop flag and status
    state.stop.store(false, Ordering::Relaxed);
    *state.status.lock().map_err(|e| e.to_string())? = AdaptiveStatus {
        game_name: game_name.clone(),
        total_tweaks: tweaks_list().len(),
        ..Default::default()
    };

    // Clone Arcs for the thread — no raw pointers needed
    let status_arc = Arc::clone(&state.status);
    let stop_arc   = Arc::clone(&state.stop);

    std::thread::spawn(move || {
        adaptive_session(game_name, game_pid, status_arc, stop_arc);
    });

    Ok(format!("Adaptive tuning started"))
}

/// Stop adaptive session.
#[tauri::command]
pub fn stop_adaptive_session(state: State<'_, AdaptiveState>) -> String {
    state.stop.store(true, Ordering::Relaxed);
    if let Ok(mut s) = state.status.lock() {
        s.message = "Зупинено користувачем".into();
    }
    "Stopped".into()
}

/// Get current adaptive session status (call every 1s from UI).
#[tauri::command]
pub fn get_adaptive_status(state: State<'_, AdaptiveState>) -> AdaptiveStatus {
    state.status.lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

/// Get saved adaptive profile for a game.
#[tauri::command]
pub fn get_adaptive_profile(
    game_name: String,
    license: State<'_, LicenseState>,
) -> Result<Vec<AdaptiveProfile>, String> {
    require_license(&license)?;
    load_profile_db(&game_name)
}

/// Apply saved adaptive profile for a game (apply only tweaks that helped).
#[tauri::command]
pub fn apply_adaptive_profile(
    game_name: String,
    game_pid: u32,
    license: State<'_, LicenseState>,
) -> Result<String, String> {
    require_license(&license)?;
    let profiles = load_profile_db(&game_name)?;

    let enabled: Vec<&AdaptiveProfile> = profiles.iter().filter(|p| p.enabled).collect();
    if enabled.is_empty() {
        return Ok(format!("No saved profile for {} — run adaptive tuning first", game_name));
    }

    let tweaks = tweaks_list();
    let mut applied = 0;

    for profile in &enabled {
        if let Some(tweak) = tweaks.iter().find(|t| t.id == profile.opt_id) {
            let _ = (tweak.apply)(game_pid);
            applied += 1;
        }
    }

    Ok(format!("Applied {} optimizations for {} (saved profile)", applied, game_name))
}
