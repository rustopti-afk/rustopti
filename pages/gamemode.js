import { invoke } from '@tauri-apps/api/core';
import { t } from '../js/i18n.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';

let pollInterval = null;
let sessionStartTs = null;
let uptimeTimer = null;
let lastDetectedPid = 0;

// ── Entry point ───────────────────────────────────────────────────────────────

export async function renderGameMode(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">🎮 AI Game Mode</h2>
      <p class="page-subtitle">ШІ навчається від кожної сесії і стає розумнішим з часом</p>
    </div>

    <!-- Active session banner (hidden by default) -->
    <div id="gm-active-banner" style="display:none;margin-bottom:16px">
      <div class="card gm-active-card">
        <div style="display:flex;justify-content:space-between;align-items:flex-start;flex-wrap:wrap;gap:12px">
          <div>
            <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px">
              <div class="gm-pulse-dot"></div>
              <span style="font-size:11px;font-weight:700;letter-spacing:0.1em;color:var(--success)">АКТИВНА СЕСІЯ</span>
            </div>
            <div id="gm-game-title" style="font-size:20px;font-weight:800;color:var(--text-1)">—</div>
            <div id="gm-game-pid"   style="font-size:12px;color:var(--text-4);margin-top:2px"></div>
          </div>
          <button class="btn btn-danger btn-ripple" id="gm-btn-deactivate" disabled style="flex-shrink:0">
            ⏹ Деактивувати
          </button>
        </div>

        <!-- Live stats -->
        <div class="gm-stats-grid" style="margin-top:16px">
          <div class="gm-stat-box">
            <div class="gm-stat-val" id="gm-uptime">00:00</div>
            <div class="gm-stat-label">тривалість</div>
          </div>
          <div class="gm-stat-box">
            <div class="gm-stat-val" id="gm-killed" style="color:#f87171">0</div>
            <div class="gm-stat-label">процесів вбито</div>
          </div>
          <div class="gm-stat-box">
            <div class="gm-stat-val" id="gm-ram" style="color:#4ade80">0</div>
            <div class="gm-stat-label">MB звільнено</div>
          </div>
          <div class="gm-stat-box">
            <div class="gm-stat-val" id="gm-samples" style="color:#a78bfa">0</div>
            <div class="gm-stat-label">семплів AI</div>
          </div>
        </div>

        <!-- GPU measurement -->
        <div id="gm-gpu-section" style="display:none;margin-top:14px">
          <div style="font-size:11px;color:var(--text-4);margin-bottom:6px;letter-spacing:0.05em">GPU 3D НАВАНТАЖЕННЯ</div>
          <div style="display:flex;align-items:center;gap:12px">
            <div style="text-align:center">
              <div style="font-size:11px;color:var(--text-4)">До</div>
              <div id="gm-gpu-before-val" style="font-size:22px;font-weight:700;color:#6b7280">—%</div>
            </div>
            <div style="flex:1">
              <div style="height:8px;background:var(--bg-3);border-radius:4px;position:relative;overflow:hidden">
                <div id="gm-gpu-before-bar" style="position:absolute;inset:0;background:#4b5563;border-radius:4px;transition:width 0.6s"></div>
                <div id="gm-gpu-after-bar"  style="position:absolute;inset:0;background:#4ade80;border-radius:4px;opacity:0.8;transition:width 0.6s"></div>
              </div>
            </div>
            <div style="text-align:center">
              <div style="font-size:11px;color:var(--text-4)">Після</div>
              <div id="gm-gpu-after-val" style="font-size:22px;font-weight:700;color:#4ade80">—%</div>
            </div>
            <div style="text-align:center;min-width:56px">
              <div style="font-size:11px;color:var(--text-4)">Буст</div>
              <div id="gm-boost-val" style="font-size:22px;font-weight:700;color:#4ade80">—%</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Idle status card -->
    <div id="gm-idle-card" class="card" style="margin-bottom:16px;text-align:center;padding:24px">
      <div style="font-size:36px;margin-bottom:8px">🎮</div>
      <div style="font-size:16px;font-weight:700;color:var(--text-1);margin-bottom:4px">Очікує запуску гри</div>
      <div style="font-size:13px;color:var(--text-3);margin-bottom:16px">
        AI автоматично активується і оптимізує систему
      </div>
      <div style="display:flex;gap:10px;justify-content:center;flex-wrap:wrap">
        <button class="btn btn-primary btn-ripple" id="gm-btn-scan">🔍 Сканувати зараз</button>
      </div>
      <div id="gm-detected" style="margin-top:12px;font-size:13px;color:var(--text-3)"></div>
    </div>

    <!-- Game Profiles -->
    <div class="section" style="margin-bottom:16px">
      <div style="display:flex;justify-content:space-between;align-items:center;
                  margin-bottom:12px;border-bottom:1px solid var(--border);padding-bottom:10px">
        <h3 class="section-title" style="margin:0;border:none;padding:0">
          🧠 AI Профілі ігор
          <span style="font-size:11px;font-weight:400;color:var(--text-4);margin-left:6px">
            навчається від сесії до сесії
          </span>
        </h3>
        <button class="btn btn-ripple" id="gm-btn-refresh-profiles" style="font-size:12px">Оновити</button>
      </div>
      <div id="gm-profiles-list" style="font-size:13px;color:var(--text-3)">
        <div class="gm-loading">Завантаження...</div>
      </div>
    </div>

    <!-- Harm Scores deep dive -->
    <div class="section" style="margin-bottom:16px">
      <div style="display:flex;justify-content:space-between;align-items:center;
                  margin-bottom:12px;border-bottom:1px solid var(--border);padding-bottom:10px">
        <h3 class="section-title" style="margin:0;border:none;padding:0">📊 Глибокий аналіз: Harm Scores</h3>
        <select id="gm-harm-game-select" style="
          background:var(--bg-3);border:1px solid var(--border);color:var(--text-1);
          padding:4px 8px;border-radius:6px;font-size:12px
        "><option value="">— обери гру —</option></select>
      </div>
      <div id="gm-harm-list" style="font-size:13px;color:var(--text-4)">
        <div style="text-align:center;padding:16px;color:var(--text-4)">
          Обери гру вище щоб побачити повну таблицю harm scores
        </div>
      </div>
    </div>

    <!-- Session History -->
    <div class="section">
      <h3 class="section-title">📅 Історія сесій</h3>
      <div id="gm-sessions-list" style="font-size:13px;color:var(--text-3)">
        <div class="gm-loading">Завантаження...</div>
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
  document.getElementById('gm-harm-game-select')?.addEventListener('change', async (e) => {
    if (e.target.value) await loadHarmScores(e.target.value);
  });
}

// ── Auto-detect loop ──────────────────────────────────────────────────────────

function startAutoDetect() {
  if (pollInterval) clearInterval(pollInterval);
  pollInterval = setInterval(autoDetect, 3000);
  autoDetect();
}

async function autoDetect() {
  // Self-healing: stop if page was navigated away
  if (!document.getElementById('gm-idle-card')) {
    clearInterval(pollInterval); pollInterval = null;
    if (uptimeTimer) { clearInterval(uptimeTimer); uptimeTimer = null; }
    return;
  }

  try {
    const status = await invoke('get_game_mode_status');

    if (status.active) {
      const [, livePid] = await invoke('detect_running_game').catch(() => ['', 0]);
      if (!livePid || livePid !== status.current_pid) {
        await deactivate();
        return;
      }
      showActiveUI(status);

      const learningInfo = await invoke('get_learning_status').catch(() => null);
      if (learningInfo?.active) {
        const el = document.getElementById('gm-samples');
        if (el) el.textContent = learningInfo.samples;
      }
      return;
    }

    const [gameName, pid] = await invoke('detect_running_game').catch(() => ['', 0]);
    if (pid && pid !== lastDetectedPid) {
      lastDetectedPid = pid;
      logInfo(`Game detected: ${gameName} (PID ${pid}) — activating Game Mode`);
      await activate(gameName, pid);
    } else if (!pid) {
      lastDetectedPid = 0;
      showIdleUI();
    }
  } catch (_) {}
}

// ── Activate / Deactivate ─────────────────────────────────────────────────────

async function activate(gameName, pid) {
  try {
    const status = await invoke('ai_activate_game_mode', { gameName, gamePid: pid });
    showActiveUI(status);
    const boostStr = status.boost_pct !== 0
      ? ` | GPU: ${status.gpu_before.toFixed(1)}% → ${status.gpu_after.toFixed(1)}% (${status.boost_pct >= 0 ? '+' : ''}${status.boost_pct.toFixed(1)}%)`
      : '';
    logSuccess(`Game Mode activated for ${gameName} — killed ${status.processes_killed} processes, freed ${status.ram_freed_mb}MB${boostStr}`);
    document.getElementById('gm-btn-deactivate').disabled = false;
  } catch (e) {
    logError(`Game Mode activation failed: ${e}`);
  }
}

async function deactivate() {
  try {
    const msg = await invoke('ai_deactivate_game_mode');
    lastDetectedPid = 0;
    showIdleUI();
    logSuccess(msg);
    document.getElementById('gm-btn-deactivate').disabled = true;
    await loadSessions();
    await loadProfiles(); // refresh profiles with new session data
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
      el.innerHTML = `<span style="color:var(--success)">✓ Знайдено: ${escHtml(gameName)} (PID ${pid})</span>`;
      logInfo(`Found game: ${gameName} — auto-activating...`);
      await activate(gameName, pid);
    } else {
      el.innerHTML = `<span style="color:var(--text-3)">Запущених ігор не знайдено</span>`;
    }
  } catch (e) {
    el.textContent = `Помилка: ${e}`;
  }
}

// ── UI state ──────────────────────────────────────────────────────────────────

function showActiveUI(status) {
  const activeBanner = document.getElementById('gm-active-banner');
  const idleCard     = document.getElementById('gm-idle-card');
  if (activeBanner) activeBanner.style.display = 'block';
  if (idleCard)     idleCard.style.display = 'none';

  const set = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
  set('gm-game-title', `🎮 ${status.current_game}`);
  set('gm-game-pid',   `PID ${status.current_pid}`);
  set('gm-killed', status.processes_killed);
  set('gm-ram',    status.ram_freed_mb);

  // GPU section
  if (status.gpu_before > 0 || status.gpu_after > 0) {
    const gpuSection = document.getElementById('gm-gpu-section');
    if (gpuSection) gpuSection.style.display = 'block';

    set('gm-gpu-before-val', `${status.gpu_before.toFixed(1)}%`);
    set('gm-gpu-after-val',  `${status.gpu_after.toFixed(1)}%`);

    const sign = status.boost_pct >= 0 ? '+' : '';
    const boostEl = document.getElementById('gm-boost-val');
    if (boostEl) {
      boostEl.textContent = `${sign}${status.boost_pct.toFixed(1)}%`;
      boostEl.style.color = status.boost_pct >= 0 ? '#4ade80' : '#f87171';
    }

    const beforeBar = document.getElementById('gm-gpu-before-bar');
    const afterBar  = document.getElementById('gm-gpu-after-bar');
    if (beforeBar) beforeBar.style.width = `${Math.min(status.gpu_before, 100)}%`;
    if (afterBar)  afterBar.style.width  = `${Math.min(status.gpu_after,  100)}%`;
  }

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

function showIdleUI() {
  const activeBanner = document.getElementById('gm-active-banner');
  const idleCard     = document.getElementById('gm-idle-card');
  if (activeBanner) activeBanner.style.display = 'none';
  if (idleCard)     idleCard.style.display = 'block';

  sessionStartTs = null;
  if (uptimeTimer) { clearInterval(uptimeTimer); uptimeTimer = null; }
}

// ── Load profiles ─────────────────────────────────────────────────────────────

async function loadProfiles() {
  try {
    const profiles = await invoke('get_game_profiles');
    const el = document.getElementById('gm-profiles-list');
    if (!el) return;

    // Populate harm scores select
    const select = document.getElementById('gm-harm-game-select');
    if (select) {
      const cur = select.value;
      select.innerHTML = '<option value="">— обери гру —</option>' +
        profiles.map(p => `<option value="${escHtml(p.game_name)}" ${p.game_name === cur ? 'selected' : ''}>${escHtml(p.game_name)}</option>`).join('');
    }

    if (!profiles.length) {
      el.innerHTML = '<div style="text-align:center;padding:20px;color:var(--text-4)">Ще немає профілів. Запусти гру — AI почне навчатися!</div>';
      return;
    }

    // Load harm scores for all profiles to show inline
    const harmData = {};
    for (const p of profiles) {
      try {
        const scores = await invoke('get_harm_scores', { gameName: p.game_name });
        harmData[p.game_name] = scores;
      } catch (_) {}
    }

    el.innerHTML = profiles.map(p => renderProfileCard(p, harmData[p.game_name] || [])).join('');
  } catch (e) {
    const el = document.getElementById('gm-profiles-list');
    if (el) el.textContent = `Помилка: ${e}`;
  }
}

function renderProfileCard(profile, harmScores) {
  const SESSION_CONFIDENCE = 5; // sessions needed for full confidence
  const confidence = Math.min(profile.session_count / SESSION_CONFIDENCE, 1.0);
  const confidencePct = Math.round(confidence * 100);
  const killList = profile.kill_list ? profile.kill_list.split(',').filter(Boolean) : [];

  // Top 3 harmful processes for inline display
  const topHarmful = harmScores
    .filter(s => s.score >= 0.3 && s.sessions >= 1)
    .sort((a, b) => b.score - a.score)
    .slice(0, 3);

  return `
    <div class="gm-profile-card">
      <!-- Header row -->
      <div style="display:flex;justify-content:space-between;align-items:flex-start;gap:12px">
        <div style="flex:1">
          <div style="font-size:15px;font-weight:700;color:var(--text-1)">${escHtml(profile.game_name)}</div>
          <div style="font-size:11px;color:var(--text-4);margin-top:2px">
            ${profile.session_count} сесій · Остання: ${escHtml(profile.last_seen || '—')}
          </div>
        </div>
        <div style="text-align:right;flex-shrink:0">
          <div style="font-size:18px;font-weight:800;color:${confidencePct >= 80 ? '#4ade80' : confidencePct >= 40 ? '#fbbf24' : '#94a3b8'}">${confidencePct}%</div>
          <div style="font-size:10px;color:var(--text-4)">AI впевненість</div>
        </div>
      </div>

      <!-- Learning progress bar -->
      <div style="margin-top:10px">
        <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text-4);margin-bottom:4px">
          <span>${profile.session_count < SESSION_CONFIDENCE ? `Навчається: ${profile.session_count}/${SESSION_CONFIDENCE} сесій` : '🧠 Повністю навчено'}</span>
          ${profile.session_count < SESSION_CONFIDENCE ? `<span>ще ${SESSION_CONFIDENCE - profile.session_count} сесій</span>` : ''}
        </div>
        <div style="height:4px;background:var(--bg-3);border-radius:2px;overflow:hidden">
          <div style="height:100%;width:${confidencePct}%;background:${confidencePct >= 80 ? '#4ade80' : '#6366f1'};border-radius:2px;transition:width 0.5s"></div>
        </div>
      </div>

      <!-- Inline harm indicators -->
      ${topHarmful.length > 0 ? `
      <div style="margin-top:10px">
        <div style="font-size:10px;color:var(--text-4);margin-bottom:6px;letter-spacing:0.05em">ТОП ШКІДЛИВИХ ПРОЦЕСІВ</div>
        <div style="display:flex;flex-wrap:wrap;gap:6px">
          ${topHarmful.map(s => {
            const color = s.score >= 0.65 ? '#ef4444' : s.score >= 0.3 ? '#f97316' : '#eab308';
            const pct   = Math.round(((s.score + 1) / 2) * 100);
            const tag   = s.score >= 0.65 && s.sessions >= 2 ? ' ✗' : '';
            return `<div style="display:flex;align-items:center;gap:5px;
                      background:rgba(255,255,255,0.04);border:1px solid rgba(255,255,255,0.08);
                      border-radius:6px;padding:3px 8px">
              <div style="width:28px;height:3px;background:var(--bg-3);border-radius:2px;overflow:hidden">
                <div style="height:100%;width:${pct}%;background:${color}"></div>
              </div>
              <span style="font-size:11px;color:${color}">${escHtml(s.proc_name)}${tag}</span>
            </div>`;
          }).join('')}
        </div>
      </div>` : profile.session_count > 0 ? `
      <div style="margin-top:8px;font-size:11px;color:var(--text-4)">
        🤖 Збираємо дані... зіграй ще ${Math.max(1, SESSION_CONFIDENCE - profile.session_count)} сесію
      </div>` : ''}

      <!-- Kill list -->
      ${killList.length > 0 ? `
      <div style="margin-top:8px;display:flex;align-items:center;gap:6px;flex-wrap:wrap">
        <span style="font-size:10px;color:var(--text-4)">KILL LIST:</span>
        ${killList.map(p => `
          <span style="font-size:10px;background:rgba(239,68,68,0.1);color:#ef4444;
                       border:1px solid rgba(239,68,68,0.2);padding:1px 6px;border-radius:4px">
            ${escHtml(p.trim())}
          </span>`).join('')}
      </div>` : ''}
    </div>
  `;
}

// ── Harm Scores full table ────────────────────────────────────────────────────

async function loadHarmScores(gameName) {
  const el = document.getElementById('gm-harm-list');
  if (!el) return;
  el.innerHTML = '<div class="gm-loading">Завантаження...</div>';
  try {
    const scores = await invoke('get_harm_scores', { gameName });
    if (!scores.length) {
      el.innerHTML = '<div style="text-align:center;padding:16px;color:var(--text-4)">Ще немає даних. Зіграй хоча б одну сесію!</div>';
      return;
    }

    el.innerHTML = `
      <div style="font-size:11px;color:var(--text-4);margin-bottom:10px;line-height:1.5">
        <b style="color:var(--text-3)">Harm score</b>: +1.0 = процес ЗАВЖДИ присутній під час просідань FPS,
        −1.0 = ніколи не заважає. Kill list: score ≥ 0.65 після ≥ 2 сесій.
      </div>
      <table class="process-table" style="width:100%">
        <thead><tr>
          <th style="text-align:left">Процес</th>
          <th>Score</th>
          <th>Сесій</th>
          <th>Статус</th>
        </tr></thead>
        <tbody>
          ${scores.map(s => {
            const pct   = Math.round(((s.score + 1) / 2) * 100);
            const color = s.score >= 0.65 ? '#ef4444' : s.score >= 0.3 ? '#f97316' : s.score >= 0 ? '#eab308' : '#22c55e';
            const badge = s.score >= 0.65 && s.sessions >= 2
              ? '<span class="gm-badge-kill">KILL LIST</span>'
              : s.score >= 0.3
              ? '<span class="gm-badge-warn">підозрілий</span>'
              : '<span style="color:var(--text-4);font-size:10px">норма</span>';
            return `<tr>
              <td style="color:var(--text-1);font-family:var(--font-mono, monospace);font-size:12px">${escHtml(s.proc_name)}</td>
              <td>
                <div style="display:flex;align-items:center;gap:8px">
                  <div style="width:64px;height:5px;background:var(--bg-3);border-radius:3px;overflow:hidden">
                    <div style="width:${pct}%;height:100%;background:${color};border-radius:3px"></div>
                  </div>
                  <span style="color:${color};font-weight:700;font-size:12px">${s.score.toFixed(3)}</span>
                </div>
              </td>
              <td style="color:var(--text-3);text-align:center">${s.sessions}</td>
              <td>${badge}</td>
            </tr>`;
          }).join('')}
        </tbody>
      </table>
    `;
  } catch (e) {
    el.textContent = `Помилка: ${e}`;
  }
}

// ── Session History ───────────────────────────────────────────────────────────

async function loadSessions() {
  try {
    const sessions = await invoke('get_game_sessions', { limit: 10 });
    const el = document.getElementById('gm-sessions-list');
    if (!el) return;

    if (!sessions.length) {
      el.innerHTML = '<div style="text-align:center;padding:16px;color:var(--text-4)">Ще немає сесій</div>';
      return;
    }

    el.innerHTML = `
      <table class="process-table" style="width:100%">
        <thead><tr>
          <th style="text-align:left">Гра</th>
          <th>Дата</th>
          <th>Тривалість</th>
          <th>Вбито</th>
          <th>RAM</th>
          <th>GPU буст</th>
        </tr></thead>
        <tbody>
          ${sessions.map(s => {
            const boostColor = s.boost_pct > 0 ? '#4ade80' : s.boost_pct < 0 ? '#f87171' : 'var(--text-4)';
            const boostText = s.gpu_before > 0
              ? `<span style="color:${boostColor};font-weight:700">${s.boost_pct >= 0 ? '+' : ''}${s.boost_pct.toFixed(1)}%</span>
                 <span style="color:var(--text-4);font-size:10px"> (${s.gpu_before.toFixed(0)}→${s.gpu_after.toFixed(0)}%)</span>`
              : '<span style="color:var(--text-4)">—</span>';
            return `<tr>
              <td style="color:var(--text-1);font-weight:600">${escHtml(s.game_name)}</td>
              <td style="color:var(--text-3)">${escHtml(s.start_time.slice(0,16))}</td>
              <td style="color:var(--text-3)">${formatDuration(s.duration_secs)}</td>
              <td style="color:#f87171;font-weight:700">${s.processes_killed}</td>
              <td style="color:#4ade80;font-weight:700">${s.ram_freed_mb} MB</td>
              <td>${boostText}</td>
            </tr>`;
          }).join('')}
        </tbody>
      </table>
    `;
  } catch (e) {
    const el = document.getElementById('gm-sessions-list');
    if (el) el.textContent = `Помилка: ${e}`;
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function escHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;')
    .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
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
    .gm-active-card {
      border-color: rgba(74,222,128,0.3) !important;
      background: linear-gradient(135deg, var(--bg-surface) 0%, rgba(74,222,128,0.04) 100%);
    }
    .gm-pulse-dot {
      width:10px; height:10px; border-radius:50%;
      background:var(--success);
      box-shadow: 0 0 0 0 rgba(74,222,128,0.7);
      animation: gm-pulse 1.5s ease-in-out infinite;
    }
    @keyframes gm-pulse {
      0%   { box-shadow: 0 0 0 0 rgba(74,222,128,0.7); }
      70%  { box-shadow: 0 0 0 8px rgba(74,222,128,0); }
      100% { box-shadow: 0 0 0 0 rgba(74,222,128,0); }
    }
    .gm-stats-grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 12px;
    }
    .gm-stat-box {
      background: var(--bg-3);
      border-radius: 8px;
      padding: 10px 8px;
      text-align: center;
    }
    .gm-stat-val {
      font-size: 20px;
      font-weight: 800;
      color: var(--accent-bright);
      line-height: 1;
    }
    .gm-stat-label {
      font-size: 10px;
      color: var(--text-4);
      margin-top: 3px;
    }
    .gm-profile-card {
      background: var(--bg-surface);
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 14px 16px;
      margin-bottom: 10px;
      transition: border-color 0.2s;
    }
    .gm-profile-card:hover { border-color: var(--border-bright); }
    .gm-badge-kill {
      background: rgba(239,68,68,0.12);
      color: #ef4444;
      border: 1px solid rgba(239,68,68,0.25);
      padding: 1px 7px; border-radius: 4px; font-size: 10px; font-weight: 700;
    }
    .gm-badge-warn {
      background: rgba(249,115,22,0.12);
      color: #f97316;
      border: 1px solid rgba(249,115,22,0.25);
      padding: 1px 7px; border-radius: 4px; font-size: 10px;
    }
    .gm-loading { text-align:center; padding:16px; color:var(--text-4); }
    @media (max-width: 480px) {
      .gm-stats-grid { grid-template-columns: repeat(2, 1fr); }
    }
  `;
  document.head.appendChild(style);
}
