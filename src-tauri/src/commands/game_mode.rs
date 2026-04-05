use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::os::windows::process::CommandExt;
use sysinfo::System;
use tauri::State;
use crate::utils::license_guard::{LicenseState, require_license};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameProfile {
    pub id:            i64,
    pub game_name:     String,
    pub session_count: i64,
    pub last_seen:     String,
    /// Comma-separated list of auto-learned processes to kill
    pub kill_list:     String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionRecord {
    pub id:              i64,
    pub game_name:       String,
    pub start_time:      String,
    pub duration_secs:   i64,
    pub processes_killed: i64,
    pub ram_freed_mb:    i64,
    pub killed_names:    String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameModeStatus {
    pub active:          bool,
    pub current_game:    String,
    pub current_pid:     u32,
    pub processes_killed: u32,
    pub ram_freed_mb:    u64,
    pub start_time:      String,
}

impl Default for GameModeStatus {
    fn default() -> Self {
        Self {
            active:          false,
            current_game:    String::new(),
            current_pid:     0,
            processes_killed: 0,
            ram_freed_mb:    0,
            start_time:      String::new(),
        }
    }
}

pub struct GameModeState(pub Mutex<GameModeStatus>);

// ── Well-known game executables ───────────────────────────────────────────────

const KNOWN_GAMES: &[&str] = &[
    "RustClient", "rust",
    "cs2", "csgo",
    "VALORANT-Win64-Shipping",
    "r5apex",               // Apex Legends
    "EscapeFromTarkov",
    "DayZ",
    "pubg", "TslGame",
    "FortniteClient-Win64-Shipping",
    "GTA5", "GTAV",
    "Cyberpunk2077",
    "eldenring",
    "bf2042",
    "ModernWarfare",
    "Warzone2",
    "RainbowSix",
];

// Processes that must NEVER be killed
const PROTECTED: &[&str] = &[
    "System", "Idle", "Registry", "smss", "csrss", "wininit",
    "winlogon", "lsass", "services", "svchost", "dwm",
    "audiodg", "fontdrvhost", "taskhostw", "sihost", "ctfmon",
    "RuntimeBroker", "SecurityHealthService",
    // Anti-cheat
    "EasyAntiCheat", "BEService", "vgc", "FaceIT",
    // GPU drivers
    "nvcontainer", "nvdisplay.container",
    // Voice
    "Discord", "TeamSpeak3",
    // Our app
    "RustOpti", "rustopti",
];

// Processes to kill when a game is running (aggressive mode)
const BACKGROUND_KILLERS: &[&str] = &[
    "OneDrive", "Teams", "Slack", "Spotify", "Discord_overlay",
    "SearchApp", "SearchIndexer", "Cortana", "YourPhone",
    "GameBarPresenceWriter", "SkypeApp", "HxOutlook",
    "MicrosoftEdgeUpdate", "GoogleUpdate", "AdobeARM",
    "CCleaner", "CCleaner64",
    "taskmgr",   // task manager itself eats CPU while gaming
    "chrome",    // optional — only if in kill list
    "msedge",
];

// ── DB helpers ────────────────────────────────────────────────────────────────

fn db_path() -> String {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    format!("{}\\RustOpti\\gamemode.db", appdata)
}

fn open_db() -> Result<Connection, String> {
    let path = db_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Connection::open(&path).map_err(|e| e.to_string())
}

fn ensure_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS game_profiles (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            game_name     TEXT UNIQUE NOT NULL,
            session_count INTEGER DEFAULT 0,
            last_seen     TEXT,
            kill_list     TEXT DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS sessions (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            game_name        TEXT,
            start_time       TEXT,
            duration_secs    INTEGER,
            processes_killed INTEGER DEFAULT 0,
            ram_freed_mb     INTEGER DEFAULT 0,
            killed_names     TEXT DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS harm_scores (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            game_name   TEXT,
            proc_name   TEXT,
            score       REAL DEFAULT 0.0,
            sessions    INTEGER DEFAULT 0,
            UNIQUE(game_name, proc_name)
        );
    ").map_err(|e| e.to_string())
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Check if any known game is currently running.
/// Returns (game_name, pid) or ("", 0) if none found.
#[tauri::command]
pub fn detect_running_game(license: State<'_, LicenseState>) -> Result<(String, u32), String> {
    require_license(&license)?;
    let mut sys = System::new_all();
    sys.refresh_all();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();
        let name_no_ext = name.trim_end_matches(".exe");
        if KNOWN_GAMES.iter().any(|g| g.eq_ignore_ascii_case(name_no_ext)) {
            return Ok((name_no_ext.to_string(), pid.as_u32()));
        }
    }
    Ok((String::new(), 0))
}

/// Activate Game Mode for a specific game PID.
/// Kills background processes and boosts game priority.
#[tauri::command]
pub fn ai_activate_game_mode(
    game_name: String,
    game_pid:  u32,
    state:     State<'_, GameModeState>,
    license:   State<'_, LicenseState>,
) -> Result<GameModeStatus, String> {
    require_license(&license)?;
    let mut status = state.0.lock().map_err(|e| e.to_string())?;
    if status.active {
        return Err("Game Mode already active".to_string());
    }

    let conn = open_db()?;
    ensure_tables(&conn)?;

    // Get learned kill list for this game
    let extra_kills: Vec<String> = conn.query_row(
        "SELECT kill_list FROM game_profiles WHERE game_name = ?1",
        params![game_name],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_default()
    .split(',')
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .collect();

    // Build full kill list
    let mut all_kills: Vec<String> = BACKGROUND_KILLERS.iter().map(|s| s.to_string()).collect();
    all_kills.extend(extra_kills);

    let mut sys = System::new_all();
    sys.refresh_all();

    let mut killed      = 0u32;
    let mut ram_freed   = 0u64;
    let mut killed_names = Vec::new();

    for (pid, process) in sys.processes() {
        if pid.as_u32() == game_pid { continue; }

        let name = process.name().to_string_lossy().to_string();
        let name_clean = name.trim_end_matches(".exe");

        // Skip protected
        if PROTECTED.iter().any(|p| p.eq_ignore_ascii_case(name_clean)) { continue; }

        if all_kills.iter().any(|k| k.eq_ignore_ascii_case(name_clean)) {
            let mem = process.memory() / 1_048_576;
            if process.kill() {
                ram_freed   += mem;
                killed      += 1;
                killed_names.push(name_clean.to_string());
            }
        }
    }

    // Boost game process priority via PowerShell
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            &format!("(Get-Process -Id {game_pid}).PriorityClass = 'High'")])
        .creation_flags(0x08000000)
        .spawn();

    // Upsert profile
    conn.execute(
        "INSERT INTO game_profiles (game_name, session_count, last_seen)
         VALUES (?1, 1, datetime('now'))
         ON CONFLICT(game_name) DO UPDATE SET
             session_count = session_count + 1,
             last_seen = datetime('now')",
        params![game_name],
    ).map_err(|e| e.to_string())?;

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    *status = GameModeStatus {
        active:           true,
        current_game:     game_name.clone(),
        current_pid:      game_pid,
        processes_killed: killed,
        ram_freed_mb:     ram_freed,
        start_time:       now,
    };

    // Log to sessions table (will update duration on deactivate)
    conn.execute(
        "INSERT INTO sessions (game_name, start_time, processes_killed, ram_freed_mb, killed_names)
         VALUES (?1, datetime('now'), ?2, ?3, ?4)",
        params![game_name, killed, ram_freed as i64, killed_names.join(", ")],
    ).map_err(|e| e.to_string())?;

    Ok(status.clone())
}

/// Deactivate Game Mode — record session duration.
#[tauri::command]
pub fn ai_deactivate_game_mode(state: State<'_, GameModeState>, license: State<'_, LicenseState>) -> Result<String, String> {
    require_license(&license)?;
    let mut status = state.0.lock().map_err(|e| e.to_string())?;
    if !status.active {
        return Ok("Game Mode was not active".to_string());
    }

    let conn = open_db()?;
    ensure_tables(&conn)?;

    // Update duration of last session for this game
    conn.execute(
        "UPDATE sessions SET duration_secs =
            CAST((julianday('now') - julianday(start_time)) * 86400 AS INTEGER)
         WHERE game_name = ?1 AND duration_secs IS NULL
         ORDER BY id DESC LIMIT 1",
        params![status.current_game],
    ).ok();

    let game = status.current_game.clone();
    *status = GameModeStatus::default();

    Ok(format!("Game Mode deactivated for {}", game))
}

/// Get current Game Mode status.
#[tauri::command]
pub fn get_game_mode_status(state: State<'_, GameModeState>) -> GameModeStatus {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Get all game profiles from SQLite.
#[tauri::command]
pub fn get_game_profiles(license: State<'_, LicenseState>) -> Result<Vec<GameProfile>, String> {
    require_license(&license)?;
    let conn = open_db()?;
    ensure_tables(&conn)?;

    let mut stmt = conn.prepare(
        "SELECT id, game_name, session_count, last_seen, kill_list
         FROM game_profiles ORDER BY last_seen DESC"
    ).map_err(|e| e.to_string())?;

    let profiles = stmt.query_map([], |row| {
        Ok(GameProfile {
            id:            row.get(0)?,
            game_name:     row.get(1)?,
            session_count: row.get(2)?,
            last_seen:     row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            kill_list:     row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(profiles)
}

/// Get recent session history.
#[tauri::command]
pub fn get_game_sessions(limit: i64, license: State<'_, LicenseState>) -> Result<Vec<SessionRecord>, String> {
    require_license(&license)?;
    let conn = open_db()?;
    ensure_tables(&conn)?;

    let mut stmt = conn.prepare(
        "SELECT id, game_name, start_time, COALESCE(duration_secs,0),
                processes_killed, ram_freed_mb, killed_names
         FROM sessions ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;

    let sessions = stmt.query_map(params![limit], |row| {
        Ok(SessionRecord {
            id:               row.get(0)?,
            game_name:        row.get(1)?,
            start_time:       row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            duration_secs:    row.get(3)?,
            processes_killed: row.get(4)?,
            ram_freed_mb:     row.get(5)?,
            killed_names:     row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(sessions)
}

/// Add a process to the learned kill list for a game.
#[tauri::command]
pub fn add_to_kill_list(game_name: String, proc_name: String, license: State<'_, LicenseState>) -> Result<String, String> {
    require_license(&license)?;
    let conn = open_db()?;
    ensure_tables(&conn)?;

    // Get current kill list
    let current: String = conn.query_row(
        "SELECT kill_list FROM game_profiles WHERE game_name = ?1",
        params![game_name],
        |row| row.get(0),
    ).unwrap_or_default();

    let mut list: Vec<String> = current.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if !list.iter().any(|p| p.eq_ignore_ascii_case(&proc_name)) {
        list.push(proc_name.clone());
    }

    conn.execute(
        "UPDATE game_profiles SET kill_list = ?1 WHERE game_name = ?2",
        params![list.join(", "), game_name],
    ).map_err(|e| e.to_string())?;

    Ok(format!("Added {} to kill list for {}", proc_name, game_name))
}

/// Get known game list (for UI display).
#[tauri::command]
pub fn get_known_games() -> Vec<String> {
    KNOWN_GAMES.iter().map(|s| s.to_string()).collect()
}
