import { invoke } from '@tauri-apps/api/core';
import { t } from '../js/i18n.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';

let pollInterval = null;

export async function renderGameMode(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">🎮 Game Mode AI</h2>
      <p class="page-subtitle">Автоматична оптимізація при запуску гри</p>
    </div>

    <!-- Status Card -->
    <div class="card card-enter" id="gm-status-card" style="margin-bottom:16px">
      <div style="display:flex;justify-content:space-between;align-items:center">
        <div>
          <div style="font-size:13px;color:var(--text-3);margin-bottom:4px">СТАТУС</div>
          <div id="gm-status-text" style="font-size:18px;font-weight:700;color:var(--text-1)">
            Очікування гри...
          </div>
          <div id="gm-game-name" style="font-size:13px;color:var(--text-3);margin-top:4px"></div>
        </div>
        <div id="gm-status-dot" style="
          width:14px;height:14px;border-radius:50%;
          background:var(--text-4);
          box-shadow:0 0 8px var(--text-4);
          transition:all 0.3s
        "></div>
      </div>

      <!-- Active session stats -->
      <div id="gm-session-stats" style="display:none;margin-top:16px;
           display:none;gap:16px;flex-wrap:wrap" class="gm-stats-row">
        <div class="gm-stat">
          <span class="gm-stat-val" id="gm-killed">0</span>
          <span class="gm-stat-label">процесів вбито</span>
        </div>
        <div class="gm-stat">
          <span class="gm-stat-val" id="gm-ram">0</span>
          <span class="gm-stat-label">MB звільнено</span>
        </div>
        <div class="gm-stat">
          <span class="gm-stat-val" id="gm-uptime">00:00</span>
          <span class="gm-stat-label">тривалість</span>
        </div>
      </div>
    </div>

    <!-- Manual control -->
    <div class="section" style="margin-bottom:16px">
      <h3 class="section-title">Ручне керування</h3>
      <div style="display:flex;gap:10px;flex-wrap:wrap">
        <button class="btn btn-primary btn-ripple" id="gm-btn-scan">
          🔍 Сканувати ігри
        </button>
        <button class="btn btn-danger btn-ripple" id="gm-btn-deactivate" disabled>
          ⏹ Деактивувати
        </button>
      </div>
      <div id="gm-detected" style="margin-top:12px;font-size:13px;color:var(--text-3)"></div>
    </div>

    <!-- Game Profiles -->
    <div class="section" style="margin-bottom:16px">
      <div style="display:flex;justify-content:space-between;align-items:center;
                  margin-bottom:12px;border-bottom:1px solid var(--border);padding-bottom:10px">
        <h3 class="section-title" style="margin:0;border:none;padding:0">Профілі ігор</h3>
        <button class="btn btn-ripple" id="gm-btn-refresh-profiles" style="font-size:12px">
          Оновити
        </button>
      </div>
      <div id="gm-profiles-list" style="font-size:13px;color:var(--text-3)">
        Завантаження...
      </div>
    </div>

    <!-- Session History -->
    <div class="section">
      <h3 class="section-title">Історія сесій</h3>
      <div id="gm-sessions-list" style="font-size:13px;color:var(--text-3)">
        Завантаження...
      </div>
    </div>
  `;

  addStyles();
  await loadProfiles();
  await loadSessions();
  startAutoDetect();

  document.getElementById('gm-btn-scan')?.addEventListener('click', scanForGames);
  document.getElementById('gm-btn-deactivate')?.addEventListener('click', deactivate);
  document.getElementById('gm-btn-refresh-profiles')?.addEventListener('click', loadProfiles);
}

// ── Auto-detect loop ──────────────────────────────────────────────────────────

function startAutoDetect() {
  if (pollInterval) clearInterval(pollInterval);
  pollInterval = setInterval(autoDetect, 3000);
  autoDetect(); // immediate first check
}

let lastDetectedPid = 0;

async function autoDetect() {
  try {
    const status = await invoke('get_game_mode_status');

    if (status.active) {
      setActiveUI(status);
      return;
    }

    const [gameName, pid] = await invoke('detect_running_game').catch(() => ['', 0]);

    if (pid && pid !== lastDetectedPid) {
      lastDetectedPid = pid;
      logInfo(`Game detected: ${gameName} (PID ${pid}) — activating Game Mode`);
      await activate(gameName, pid);
    } else if (!pid) {
      lastDetectedPid = 0;
      setIdleUI();
    }
  } catch (e) {
    // silently ignore — polling runs frequently
  }
}

// ── Activate / Deactivate ─────────────────────────────────────────────────────

async function activate(gameName, pid) {
  try {
    const status = await invoke('ai_activate_game_mode', {
      gameName, gamePid: pid
    });
    setActiveUI(status);
    logSuccess(`Game Mode activated for ${gameName} — killed ${status.processes_killed} processes, freed ${status.ram_freed_mb}MB`);
    document.getElementById('gm-btn-deactivate').disabled = false;
  } catch (e) {
    logError(`Game Mode activation failed: ${e}`);
  }
}

async function deactivate() {
  try {
    const msg = await invoke('ai_deactivate_game_mode');
    lastDetectedPid = 0;
    setIdleUI();
    logSuccess(msg);
    document.getElementById('gm-btn-deactivate').disabled = true;
    await loadSessions();
  } catch (e) {
    logError(`Deactivation failed: ${e}`);
  }
}

async function scanForGames() {
  const el = document.getElementById('gm-detected');
  el.textContent = 'Сканування...';
  try {
    const [gameName, pid] = await invoke('detect_running_game').catch(() => ['', 0]);
    if (pid) {
      el.innerHTML = `<span style="color:var(--success)">✓ Знайдено: ${gameName} (PID ${pid})</span>`;
      logInfo(`Found game: ${gameName} — click Activate or wait for auto-detect`);
    } else {
      el.innerHTML = `<span style="color:var(--text-3)">Запущених ігор не знайдено</span>`;
    }
  } catch (e) {
    el.textContent = `Помилка: ${e}`;
  }
}

// ── UI state ──────────────────────────────────────────────────────────────────

let sessionStartTs = null;
let uptimeTimer = null;

function setActiveUI(status) {
  const dot  = document.getElementById('gm-status-dot');
  const text = document.getElementById('gm-status-text');
  const name = document.getElementById('gm-game-name');
  const stats = document.getElementById('gm-session-stats');

  if (dot)  { dot.style.background = 'var(--success)'; dot.style.boxShadow = '0 0 12px var(--success)'; }
  if (text) text.textContent = 'ACTIVE';
  if (name) name.textContent = `🎮 ${status.current_game} (PID ${status.current_pid})`;

  document.getElementById('gm-killed').textContent = status.processes_killed;
  document.getElementById('gm-ram').textContent    = status.ram_freed_mb;
  if (stats) stats.style.display = 'flex';

  document.getElementById('gm-btn-deactivate').disabled = false;

  // Start uptime counter
  if (!sessionStartTs) {
    sessionStartTs = new Date(status.start_time) || new Date();
    uptimeTimer = setInterval(() => {
      const secs = Math.floor((Date.now() - sessionStartTs) / 1000);
      const m = String(Math.floor(secs / 60)).padStart(2, '0');
      const s = String(secs % 60).padStart(2, '0');
      const el = document.getElementById('gm-uptime');
      if (el) el.textContent = `${m}:${s}`;
    }, 1000);
  }
}

function setIdleUI() {
  const dot  = document.getElementById('gm-status-dot');
  const text = document.getElementById('gm-status-text');
  const name = document.getElementById('gm-game-name');
  const stats = document.getElementById('gm-session-stats');

  if (dot)  { dot.style.background = 'var(--text-4)'; dot.style.boxShadow = 'none'; }
  if (text) text.textContent = 'Очікування гри...';
  if (name) name.textContent = 'Автоматично активується при запуску гри';
  if (stats) stats.style.display = 'none';

  sessionStartTs = null;
  if (uptimeTimer) { clearInterval(uptimeTimer); uptimeTimer = null; }
}

// ── Load profiles / sessions ──────────────────────────────────────────────────

async function loadProfiles() {
  try {
    const profiles = await invoke('get_game_profiles');
    const el = document.getElementById('gm-profiles-list');
    if (!el) return;

    if (!profiles.length) {
      el.innerHTML = '<span style="color:var(--text-4)">Ще немає профілів. Запусти гру!</span>';
      return;
    }

    el.innerHTML = profiles.map(p => `
      <div class="gm-profile-row">
        <div style="flex:1">
          <span style="color:var(--text-1);font-weight:600">${p.game_name}</span>
          <span style="color:var(--text-3);margin-left:10px;font-size:12px">
            ${p.session_count} сесій · ${p.last_seen}
          </span>
          ${p.kill_list ? `<div style="color:var(--text-3);font-size:11px;margin-top:2px">
            Kill list: ${p.kill_list}
          </div>` : ''}
        </div>
      </div>
    `).join('');
  } catch (e) {
    document.getElementById('gm-profiles-list').textContent = `Помилка: ${e}`;
  }
}

async function loadSessions() {
  try {
    const sessions = await invoke('get_game_sessions', { limit: 10 });
    const el = document.getElementById('gm-sessions-list');
    if (!el) return;

    if (!sessions.length) {
      el.innerHTML = '<span style="color:var(--text-4)">Ще немає сесій</span>';
      return;
    }

    el.innerHTML = `
      <table class="process-table" style="width:100%">
        <thead><tr>
          <th>Гра</th><th>Дата</th><th>Тривалість</th>
          <th>Вбито</th><th>RAM</th>
        </tr></thead>
        <tbody>
          ${sessions.map(s => `
            <tr>
              <td style="color:var(--text-1)">${s.game_name}</td>
              <td>${s.start_time.slice(0,16)}</td>
              <td>${formatDuration(s.duration_secs)}</td>
              <td style="color:var(--danger)">${s.processes_killed}</td>
              <td style="color:var(--success)">${s.ram_freed_mb} MB</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    `;
  } catch (e) {
    document.getElementById('gm-sessions-list').textContent = `Помилка: ${e}`;
  }
}

function formatDuration(secs) {
  if (!secs) return '—';
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return m > 0 ? `${m}хв ${s}с` : `${s}с`;
}

// ── Styles ────────────────────────────────────────────────────────────────────

function addStyles() {
  if (document.getElementById('gm-styles')) return;
  const style = document.createElement('style');
  style.id = 'gm-styles';
  style.textContent = `
    .gm-stats-row { display:flex; gap:24px; flex-wrap:wrap; margin-top:16px; }
    .gm-stat { display:flex; flex-direction:column; align-items:center; }
    .gm-stat-val { font-size:22px; font-weight:700; color:var(--accent-bright); }
    .gm-stat-label { font-size:11px; color:var(--text-3); margin-top:2px; }
    .gm-profile-row {
      display:flex; align-items:center; padding:10px 12px;
      border-bottom:1px solid var(--border);
    }
    .gm-profile-row:last-child { border-bottom:none; }
  `;
  document.head.appendChild(style);
}
