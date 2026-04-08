import { invoke } from '@tauri-apps/api/core';

let adaptivePollTimer = null;

export async function renderAdaptive(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">⚡ Adaptive FPS Tuner</h2>
      <p class="page-subtitle">Автоматично тестує кожне покращення і зберігає тільки те, що дає реальний приріст FPS</p>
    </div>

    <div id="adaptive-container">
      <div class="card" style="text-align:center;padding:40px 20px;color:var(--text-3)">
        <div class="ai-spinner" style="margin:0 auto 16px"></div>
        Завантаження...
      </div>
    </div>
  `;

  stopAdaptivePoll();
  await loadAdaptiveStatus();
}

async function loadAdaptiveStatus() {
  try {
    const s = await invoke('get_adaptive_status');
    renderAdaptiveUI(s);
    if (s.running) startAdaptivePoll();
    else stopAdaptivePoll();
  } catch (e) {
    const c = document.getElementById('adaptive-container');
    if (c) c.innerHTML = `<div class="card" style="color:#f87171;padding:20px">Помилка: ${e}</div>`;
  }
}

function startAdaptivePoll() {
  if (adaptivePollTimer) return;
  adaptivePollTimer = setInterval(loadAdaptiveStatus, 1000);
}

function stopAdaptivePoll() {
  if (adaptivePollTimer) { clearInterval(adaptivePollTimer); adaptivePollTimer = null; }
}

function renderAdaptiveUI(s) {
  const c = document.getElementById('adaptive-container');
  if (!c) return;

  const phaseLabel = {
    idle: '⏸ Очікування',
    baseline: '📊 Вимірюємо базовий FPS',
    testing: '🔬 Тестуємо покращення',
    done: '✅ Завершено'
  };

  let html = `<div class="card card-enter" style="margin-bottom:16px">`;

  // Header row
  html += `<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
    <div style="font-size:16px;font-weight:700;color:var(--text-1)">
      ${s.game_name ? `🎮 ${s.game_name}` : 'Готовий до запуску'}
    </div>
    <span style="font-size:11px;padding:3px 10px;border-radius:20px;background:rgba(99,102,241,0.15);color:#818cf8">
      ${phaseLabel[s.phase] || s.phase}
    </span>
  </div>`;

  // Message
  if (s.message) {
    html += `<div style="font-size:13px;color:var(--text-3);margin-bottom:12px">${s.message}</div>`;
  }

  // Progress bar
  if (s.running || s.phase === 'done') {
    const pct = s.progress_pct || 0;
    html += `<div style="background:rgba(255,255,255,0.06);border-radius:4px;height:6px;margin-bottom:8px;overflow:hidden">
      <div style="height:100%;width:${pct}%;background:linear-gradient(90deg,#6366f1,#a78bfa);border-radius:4px;transition:width 0.5s"></div>
    </div>
    <div style="font-size:11px;color:var(--text-4);margin-bottom:12px">
      ${s.current_tweak_idx || 0} / ${s.total_tweaks || 7} покращень
    </div>`;
  }

  // GPU bar
  if (s.baseline_gpu > 0) {
    html += `<div style="display:flex;gap:16px;margin-bottom:12px;font-size:12px;color:var(--text-3)">
      <span>GPU baseline: <b style="color:var(--text-1)">${s.baseline_gpu.toFixed(1)}%</b></span>
      ${s.current_gpu !== s.baseline_gpu
        ? `<span>→ зараз: <b style="color:#a78bfa">${s.current_gpu.toFixed(1)}%</b></span>`
        : ''}
    </div>`;
  }

  // Control buttons
  html += `<div style="display:flex;gap:8px">`;
  if (!s.running) {
    html += `<button class="primary-btn" style="font-size:13px" onclick="window._adaptiveStart()">▶ Запустити тюнінг</button>`;
    if (s.phase === 'done') {
      html += `<button class="secondary-btn" style="font-size:13px" onclick="window._adaptiveReset()">↺ Новий тест</button>`;
    }
  } else {
    html += `<button class="secondary-btn" style="font-size:13px;color:#f87171;border-color:rgba(248,113,113,0.3)" onclick="window._adaptiveStop()">⏹ Зупинити</button>`;
  }
  html += `</div></div>`;

  // Results table
  if (s.results && s.results.length > 0) {
    const kept = s.results.filter(r => r.kept).length;
    html += `<div class="card card-enter" style="margin-bottom:16px">
      <div style="font-size:13px;font-weight:600;color:var(--text-2);margin-bottom:12px">
        Результати тестів
        ${s.phase === 'done' ? `<span style="color:#4ade80;margin-left:8px">${kept} покращень збережено</span>` : ''}
      </div>
      <table style="width:100%;border-collapse:collapse;font-size:12px">
        <thead>
          <tr style="color:var(--text-4);border-bottom:1px solid rgba(255,255,255,0.06)">
            <th style="text-align:left;padding:6px 8px">Покращення</th>
            <th style="text-align:right;padding:6px 8px">До</th>
            <th style="text-align:right;padding:6px 8px">Після</th>
            <th style="text-align:right;padding:6px 8px">Приріст</th>
            <th style="text-align:center;padding:6px 8px">Статус</th>
          </tr>
        </thead><tbody>`;

    for (const r of s.results) {
      const badge = r.kept
        ? `<span style="color:#4ade80;font-size:11px">✓ Збережено</span>`
        : `<span style="color:var(--text-4);font-size:11px">✗ Відхилено</span>`;
      const gainColor = r.gain_pct >= 2 ? '#4ade80' : (r.gain_pct < 0 ? '#f87171' : 'var(--text-3)');
      html += `<tr style="border-bottom:1px solid rgba(255,255,255,0.04)">
        <td style="padding:7px 8px;color:var(--text-2)">${r.name}</td>
        <td style="padding:7px 8px;text-align:right;color:var(--text-3)">${r.gpu_before.toFixed(1)}%</td>
        <td style="padding:7px 8px;text-align:right;color:var(--text-3)">${r.gpu_after.toFixed(1)}%</td>
        <td style="padding:7px 8px;text-align:right;color:${gainColor};font-weight:600">${r.gain_pct >= 0 ? '+' : ''}${r.gain_pct.toFixed(1)}%</td>
        <td style="padding:7px 8px;text-align:center">${badge}</td>
      </tr>`;
    }
    html += `</tbody></table></div>`;
  }

  // Saved profiles
  html += `<div id="adaptive-profiles-section"></div>`;

  // Modal for start
  html += `<div id="adaptive-modal" style="
    display:none;position:fixed;inset:0;background:rgba(0,0,0,0.7);
    z-index:1000;align-items:center;justify-content:center
  ">
    <div style="background:var(--bg-2);border:1px solid rgba(255,255,255,0.1);border-radius:12px;padding:24px;width:340px">
      <div style="font-size:15px;font-weight:700;color:var(--text-1);margin-bottom:8px">⚡ Запустити Adaptive Tuner</div>
      <p style="color:var(--text-4);font-size:12px;margin-bottom:16px;line-height:1.5">
        Запусти гру, зайди в матч і одразу натисни "Почати".<br>
        Tuner виміряє FPS до/після кожного покращення (~5 хв).
      </p>
      <label style="color:var(--text-3);font-size:12px;display:block;margin-bottom:4px">Назва гри</label>
      <input id="am-game-name" placeholder="Rust, CS2, Fortnite..." style="
        width:100%;box-sizing:border-box;background:rgba(255,255,255,0.05);
        border:1px solid rgba(255,255,255,0.1);border-radius:6px;
        padding:8px 10px;color:var(--text-1);font-size:13px;margin-bottom:10px
      " />
      <label style="color:var(--text-3);font-size:12px;display:block;margin-bottom:4px">PID процесу (0 = автопошук)</label>
      <input id="am-game-pid" type="number" value="0" style="
        width:100%;box-sizing:border-box;background:rgba(255,255,255,0.05);
        border:1px solid rgba(255,255,255,0.1);border-radius:6px;
        padding:8px 10px;color:var(--text-1);font-size:13px;margin-bottom:16px
      " />
      <div style="display:flex;gap:8px">
        <button class="primary-btn" style="flex:1;font-size:13px" onclick="window._adaptiveConfirm()">▶ Почати</button>
        <button class="secondary-btn" style="font-size:13px" onclick="window._adaptiveCloseModal()">Скасувати</button>
      </div>
    </div>
  </div>`;

  c.innerHTML = html;

  // Bind window functions
  window._adaptiveStart = () => {
    const modal = document.getElementById('adaptive-modal');
    if (modal) modal.style.display = 'flex';
    // Pre-fill from game mode
    invoke('get_game_mode_status').then(gs => {
      if (gs && gs.game_name) {
        const nameInput = document.getElementById('am-game-name');
        const pidInput  = document.getElementById('am-game-pid');
        if (nameInput) nameInput.value = gs.game_name;
        if (pidInput && gs.game_pid) pidInput.value = gs.game_pid;
      }
    }).catch(() => {});
  };

  window._adaptiveCloseModal = () => {
    const m = document.getElementById('adaptive-modal');
    if (m) m.style.display = 'none';
  };

  window._adaptiveConfirm = async () => {
    const name = document.getElementById('am-game-name')?.value.trim();
    const pid  = parseInt(document.getElementById('am-game-pid')?.value) || 0;
    if (!name) { alert('Введи назву гри'); return; }
    window._adaptiveCloseModal();
    try {
      await invoke('start_adaptive_session', { gameName: name, gamePid: pid });
      startAdaptivePoll();
      await loadAdaptiveStatus();
    } catch (e) { alert('Помилка: ' + e); }
  };

  window._adaptiveStop = async () => {
    try {
      await invoke('stop_adaptive_session');
      stopAdaptivePoll();
      setTimeout(loadAdaptiveStatus, 600);
    } catch (e) { console.error(e); }
  };

  window._adaptiveReset = () => {
    stopAdaptivePoll();
    loadAdaptiveStatus();
  };

  // Load saved profiles
  loadAdaptiveProfiles();
}

async function loadAdaptiveProfiles() {
  const sec = document.getElementById('adaptive-profiles-section');
  if (!sec) return;

  try {
    const gs = await invoke('get_game_mode_status');
    if (!gs || !gs.game_name) return;

    const profiles = await invoke('get_adaptive_profile', { gameName: gs.game_name });
    if (!profiles || profiles.length === 0) return;

    const enabled = profiles.filter(p => p.enabled);
    if (enabled.length === 0) return;

    let html = `<div class="card card-enter">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
        <div style="font-size:13px;font-weight:600;color:var(--text-2)">
          💾 Збережений профіль для "${gs.game_name}"
        </div>
        <button class="primary-btn" style="font-size:12px;padding:4px 12px"
          onclick="window._applyProfile('${gs.game_name}', ${gs.game_pid || 0})">
          ▶ Застосувати
        </button>
      </div>
      <table style="width:100%;border-collapse:collapse;font-size:12px">
        <thead>
          <tr style="color:var(--text-4);border-bottom:1px solid rgba(255,255,255,0.06)">
            <th style="text-align:left;padding:5px 8px">Покращення</th>
            <th style="text-align:right;padding:5px 8px">Приріст GPU</th>
            <th style="text-align:center;padding:5px 8px">Статус</th>
          </tr>
        </thead><tbody>`;

    for (const p of profiles) {
      const badge = p.enabled
        ? `<span style="color:#4ade80;font-size:11px">✓ Активно</span>`
        : `<span style="color:var(--text-4);font-size:11px">✗ Вимкнено</span>`;
      const gainColor = p.fps_gain >= 2 ? '#4ade80' : 'var(--text-3)';
      html += `<tr style="border-bottom:1px solid rgba(255,255,255,0.04)">
        <td style="padding:6px 8px;color:var(--text-2)">${p.opt_id}</td>
        <td style="padding:6px 8px;text-align:right;color:${gainColor};font-weight:600">
          ${p.fps_gain >= 0 ? '+' : ''}${p.fps_gain.toFixed(1)}%
        </td>
        <td style="padding:6px 8px;text-align:center">${badge}</td>
      </tr>`;
    }
    html += `</tbody></table></div>`;
    sec.innerHTML = html;

    window._applyProfile = async (name, pid) => {
      try {
        const msg = await invoke('apply_adaptive_profile', { gameName: name, gamePid: pid });
        alert(msg);
      } catch (e) { alert('Помилка: ' + e); }
    };
  } catch (e) {
    // no profile yet — that's fine
  }
}
