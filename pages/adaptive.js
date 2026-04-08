import { invoke } from '@tauri-apps/api/core';
import { logInfo, logSuccess, logError } from '../js/terminal.js';

let pollTimer = null;

const TWEAK_NAMES = [
  'Timer Resolution 0.5ms',
  'CPU Core Unparking',
  'HPET Disable',
  'Nagle Algorithm Off',
  'GPU Shader Preload',
  'MSI Interrupt Mode',
];

// ── Entry point ───────────────────────────────────────────────────────────────

export async function renderAdaptive(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">⚡ Adaptive FPS Tuner</h2>
      <p class="page-subtitle">Автотестує кожне покращення і зберігає тільки те, що реально дає +FPS</p>
    </div>
    <div id="adp-root">
      <div class="card" style="text-align:center;padding:40px 20px;color:var(--text-3)">
        <div class="ai-spinner" style="margin:0 auto 16px"></div>
        Завантаження...
      </div>
    </div>

    <!-- Start modal -->
    <div id="adp-modal" style="display:none;position:fixed;inset:0;background:rgba(0,0,0,0.75);
         z-index:1000;align-items:center;justify-content:center">
      <div style="background:var(--bg-2);border:1px solid var(--border);
                  border-radius:14px;padding:28px;width:360px;max-width:90vw">
        <div style="font-size:16px;font-weight:800;color:var(--text-1);margin-bottom:6px">⚡ Запустити Adaptive Tuner</div>
        <p style="color:var(--text-4);font-size:12px;margin-bottom:20px;line-height:1.6">
          Запусти гру, зайди в матч і натисни "Почати".<br>
          AI виміряє GPU% до і після кожного покращення (~5 хвилин).
        </p>
        <label style="color:var(--text-3);font-size:12px;font-weight:600;display:block;margin-bottom:4px">
          Назва гри
        </label>
        <input id="adp-game-name" placeholder="Rust, CS2, Fortnite..." class="adp-input" style="margin-bottom:12px"/>
        <label style="color:var(--text-3);font-size:12px;font-weight:600;display:block;margin-bottom:4px">
          PID процесу <span style="color:var(--text-4);font-weight:400">(0 = автопошук)</span>
        </label>
        <input id="adp-game-pid" type="number" value="0" class="adp-input" style="margin-bottom:20px"/>
        <div style="display:flex;gap:10px">
          <button class="btn btn-primary btn-ripple" id="adp-confirm" style="flex:1">▶ Почати тюнінг</button>
          <button class="btn btn-ripple" id="adp-cancel">Скасувати</button>
        </div>
      </div>
    </div>
  `;

  addStyles();
  bindModal();
  stopPoll();
  await loadStatus();
}

// ── Modal ─────────────────────────────────────────────────────────────────────

function bindModal() {
  document.getElementById('adp-cancel')?.addEventListener('click', closeModal);
  document.getElementById('adp-confirm')?.addEventListener('click', startSession);
}

function openModal() {
  const modal = document.getElementById('adp-modal');
  if (modal) modal.style.display = 'flex';
  // Pre-fill from active Game Mode
  invoke('get_game_mode_status').then(gs => {
    if (gs?.current_game) {
      const nameEl = document.getElementById('adp-game-name');
      const pidEl  = document.getElementById('adp-game-pid');
      if (nameEl) nameEl.value = gs.current_game;
      if (pidEl && gs.current_pid) pidEl.value = gs.current_pid;
    }
  }).catch(() => {});
}

function closeModal() {
  const modal = document.getElementById('adp-modal');
  if (modal) modal.style.display = 'none';
}

async function startSession() {
  const name = document.getElementById('adp-game-name')?.value.trim();
  const pid  = parseInt(document.getElementById('adp-game-pid')?.value) || 0;
  if (!name) { alert('Введи назву гри'); return; }
  closeModal();
  try {
    await invoke('start_adaptive_session', { gameName: name, gamePid: pid });
    logInfo(`Adaptive Tuner started for ${name}`);
    startPoll();
    await loadStatus();
  } catch (e) {
    logError(`Adaptive start failed: ${e}`);
  }
}

// ── Status polling ────────────────────────────────────────────────────────────

function startPoll() {
  if (pollTimer) return;
  pollTimer = setInterval(loadStatus, 1000);
}

function stopPoll() {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
}

async function loadStatus() {
  if (!document.getElementById('adp-root')) { stopPoll(); return; }
  try {
    const s = await invoke('get_adaptive_status');
    renderUI(s);
    if (s.running) startPoll(); else stopPoll();
  } catch (e) {
    const root = document.getElementById('adp-root');
    if (root) root.innerHTML = `<div class="card" style="color:#f87171;padding:20px">Помилка: ${e}</div>`;
  }
}

// ── Main render ───────────────────────────────────────────────────────────────

function renderUI(s) {
  const root = document.getElementById('adp-root');
  if (!root) return;

  const PHASE_LABEL = {
    idle:     '⏸ Готовий до запуску',
    baseline: '📊 Вимірюємо базовий GPU%',
    testing:  '🔬 Тестуємо покращення',
    done:     '✅ Тюнінг завершено',
  };

  const currentTweakName = s.current_tweak_idx > 0 && s.current_tweak_idx <= TWEAK_NAMES.length
    ? TWEAK_NAMES[s.current_tweak_idx - 1]
    : null;

  let html = '';

  // ── Status card ──
  html += `<div class="card card-enter" style="margin-bottom:16px">
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:14px">
      <div>
        <div style="font-size:17px;font-weight:800;color:var(--text-1)">
          ${s.game_name ? `🎮 ${s.game_name}` : 'Adaptive FPS Tuner'}
        </div>
        <div style="font-size:12px;color:var(--text-4);margin-top:2px">${PHASE_LABEL[s.phase] || s.phase}</div>
      </div>
      ${s.running
        ? `<button class="btn btn-danger btn-ripple" id="adp-stop">⏹ Зупинити</button>`
        : `<button class="btn btn-primary btn-ripple" id="adp-start">▶ Запустити тюнінг</button>`
      }
    </div>`;

  // Message
  if (s.message) {
    html += `<div style="font-size:13px;color:var(--text-3);margin-bottom:12px
             ;padding:10px 12px;background:rgba(255,255,255,0.03);border-radius:7px">
      ${s.message}
    </div>`;
  }

  // ── Step visual indicator ──
  if (s.running || s.phase === 'done') {
    html += renderStepIndicator(s);
  }

  // ── Live GPU bar ──
  if (s.baseline_gpu > 0 || s.current_gpu > 0) {
    const gpuChanged = Math.abs(s.current_gpu - s.baseline_gpu) > 0.5;
    const gpuColor = gpuChanged ? (s.current_gpu > s.baseline_gpu ? '#4ade80' : '#f87171') : 'var(--text-1)';
    html += `<div style="display:flex;align-items:center;gap:14px;margin-top:8px;
                         padding:10px 12px;background:var(--bg-3);border-radius:8px">
      <div>
        <div style="font-size:10px;color:var(--text-4)">БАЗОВИЙ GPU%</div>
        <div style="font-size:22px;font-weight:800;color:var(--text-2)">${s.baseline_gpu.toFixed(1)}%</div>
      </div>
      ${s.running && s.phase === 'testing' ? `
      <div style="font-size:20px;color:var(--text-4)">→</div>
      <div>
        <div style="font-size:10px;color:var(--text-4)">ЗАРАЗ (з твіком)</div>
        <div style="font-size:22px;font-weight:800;color:${gpuColor}">${s.current_gpu.toFixed(1)}%</div>
      </div>` : ''}
      ${s.running && currentTweakName && s.phase === 'testing' ? `
      <div style="margin-left:auto;text-align:right">
        <div style="font-size:10px;color:var(--text-4)">ТЕСТУЄМО</div>
        <div style="font-size:12px;font-weight:600;color:#a78bfa">${currentTweakName}</div>
      </div>` : ''}
    </div>`;
  }

  html += `</div>`;

  // ── Results table ──
  if (s.results && s.results.length > 0) {
    const keptCount = s.results.filter(r => r.kept).length;
    html += `<div class="card card-enter" style="margin-bottom:16px">
      <div style="font-size:13px;font-weight:700;color:var(--text-2);margin-bottom:12px">
        Результати тестів
        ${s.phase === 'done' ? `<span style="color:#4ade80;font-weight:400;margin-left:8px">
          ${keptCount} покращень збережено
        </span>` : ''}
      </div>
      <table style="width:100%;border-collapse:collapse;font-size:12px">
        <thead><tr style="color:var(--text-4);border-bottom:1px solid var(--border)">
          <th style="text-align:left;padding:6px 8px">Покращення</th>
          <th style="text-align:right;padding:6px 8px">До</th>
          <th style="text-align:right;padding:6px 8px">Після</th>
          <th style="text-align:right;padding:6px 8px">Приріст</th>
          <th style="text-align:center;padding:6px 8px">Рішення</th>
        </tr></thead>
        <tbody>
          ${s.results.map((r, i) => {
            const isCurrent = s.running && s.phase === 'testing' && i === (s.current_tweak_idx - 1);
            const gainColor = r.gain_pct >= 2 ? '#4ade80' : r.gain_pct < 0 ? '#f87171' : 'var(--text-3)';
            const rowBg = isCurrent ? 'rgba(99,102,241,0.08)' : 'transparent';
            const badge = r.kept
              ? `<span style="color:#4ade80;font-weight:700">✓ Збережено</span>`
              : `<span style="color:var(--text-4)">✗ Відхилено</span>`;
            return `<tr style="border-bottom:1px solid rgba(255,255,255,0.04);background:${rowBg}">
              <td style="padding:7px 8px;color:${isCurrent ? '#a78bfa' : 'var(--text-2)'};font-weight:${isCurrent ? '700' : '400'}">
                ${isCurrent ? '⚡ ' : ''}${r.name}
              </td>
              <td style="padding:7px 8px;text-align:right;color:var(--text-3)">${r.gpu_before.toFixed(1)}%</td>
              <td style="padding:7px 8px;text-align:right;color:var(--text-3)">${r.gpu_after > 0 ? r.gpu_after.toFixed(1) + '%' : '—'}</td>
              <td style="padding:7px 8px;text-align:right;color:${gainColor};font-weight:700">
                ${r.gpu_after > 0 ? (r.gain_pct >= 0 ? '+' : '') + r.gain_pct.toFixed(1) + '%' : '—'}
              </td>
              <td style="padding:7px 8px;text-align:center;font-size:11px">${r.gpu_after > 0 ? badge : '<span style="color:var(--text-4)">в черзі</span>'}</td>
            </tr>`;
          }).join('')}
        </tbody>
      </table>
    </div>`;
  }

  // ── Saved profile ──
  html += `<div id="adp-profile-section"></div>`;

  // ── Rescan button ──
  if (s.phase === 'done') {
    html += `<div style="text-align:center;margin-top:4px">
      <button class="btn btn-ripple" id="adp-new-test">↺ Новий тест</button>
    </div>`;
  }

  root.innerHTML = html;

  // Bind buttons (proper event listeners, not window._)
  document.getElementById('adp-start')?.addEventListener('click', openModal);
  document.getElementById('adp-stop')?.addEventListener('click', async () => {
    try {
      await invoke('stop_adaptive_session');
      logInfo('Adaptive Tuner stopped');
      stopPoll();
      setTimeout(loadStatus, 600);
    } catch (e) { logError(e); }
  });
  document.getElementById('adp-new-test')?.addEventListener('click', () => {
    stopPoll();
    loadStatus();
  });

  loadSavedProfile(s.game_name);
}

// ── Step indicator ────────────────────────────────────────────────────────────

function renderStepIndicator(s) {
  const total = TWEAK_NAMES.length;
  const current = s.phase === 'baseline' ? 0 : s.current_tweak_idx || 0;

  let html = `<div style="margin-bottom:14px">
    <div style="display:flex;align-items:center;gap:0;margin-bottom:8px;overflow-x:auto;padding-bottom:4px">`;

  for (let i = 0; i < total; i++) {
    const isDone    = i < (s.phase === 'done' ? total : current - 1);
    const isCurrent = s.phase === 'testing' && i === current - 1;
    const result    = s.results && s.results[i];
    const kept      = result?.kept;

    const color = isDone
      ? (kept ? '#4ade80' : 'var(--text-4)')
      : isCurrent ? '#a78bfa'
      : 'var(--bg-3)';
    const border = isCurrent ? '2px solid #a78bfa' : `1px solid ${isDone ? (kept ? '#4ade80' : 'var(--border)') : 'var(--border)'}`;
    const icon = isDone ? (kept ? '✓' : '✗') : isCurrent ? '⚡' : (i + 1);

    html += `<div style="display:flex;align-items:center;flex-shrink:0">
      <div title="${TWEAK_NAMES[i]}" style="
        width:30px; height:30px; border-radius:50%;
        background:${isCurrent ? 'rgba(167,139,250,0.15)' : isDone ? 'rgba(255,255,255,0.04)' : 'var(--bg-3)'};
        border:${border};
        display:flex; align-items:center; justify-content:center;
        font-size:11px; font-weight:700; color:${color};
        ${isCurrent ? 'animation:adp-glow 1s ease-in-out infinite alternate;' : ''}
        transition: all 0.3s;
      ">${icon}</div>
      ${i < total - 1 ? `<div style="width:20px;height:2px;background:${isDone ? (kept ? '#4ade8040' : 'var(--border)') : 'var(--border)'}"></div>` : ''}
    </div>`;
  }

  html += `</div>`;

  // Step labels
  if (s.phase === 'testing' && current > 0) {
    const progressPct = s.progress_pct || Math.round((current / total) * 100);
    html += `<div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text-4);margin-bottom:4px">
      <span>${current}/${total} покращень</span>
      <span>${progressPct}%</span>
    </div>
    <div style="height:3px;background:var(--bg-3);border-radius:2px;overflow:hidden">
      <div style="height:100%;width:${progressPct}%;background:linear-gradient(90deg,#6366f1,#a78bfa);border-radius:2px;transition:width 0.8s"></div>
    </div>`;
  }

  html += `</div>`;
  return html;
}

// ── Saved profile ─────────────────────────────────────────────────────────────

async function loadSavedProfile(gameName) {
  const sec = document.getElementById('adp-profile-section');
  if (!sec || !gameName) return;

  try {
    const profiles = await invoke('get_adaptive_profile', { gameName });
    const enabled = profiles?.filter(p => p.enabled) || [];
    if (!enabled.length) return;

    const totalGain = enabled.reduce((sum, p) => sum + p.fps_gain, 0);

    sec.innerHTML = `
      <div class="card card-enter" style="margin-top:4px">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <div>
            <div style="font-size:13px;font-weight:700;color:var(--text-2)">💾 Збережений профіль</div>
            <div style="font-size:11px;color:var(--text-4);margin-top:2px">
              ${gameName} · ${enabled.length} покращень · сумарний буст
              <span style="color:#4ade80;font-weight:700">+${totalGain.toFixed(1)}% GPU</span>
            </div>
          </div>
          <button class="btn btn-primary btn-ripple" id="adp-apply-profile" style="font-size:12px">
            ▶ Застосувати профіль
          </button>
        </div>
        <div style="display:flex;flex-wrap:wrap;gap:6px">
          ${enabled.map(p => `
            <div style="background:rgba(74,222,128,0.08);border:1px solid rgba(74,222,128,0.2);
                         border-radius:6px;padding:4px 10px;font-size:11px">
              <span style="color:#4ade80;font-weight:700">+${p.fps_gain.toFixed(1)}%</span>
              <span style="color:var(--text-3);margin-left:4px">${p.opt_id}</span>
            </div>`).join('')}
        </div>
      </div>
    `;

    document.getElementById('adp-apply-profile')?.addEventListener('click', async () => {
      try {
        const gs = await invoke('get_game_mode_status');
        const pid = gs?.current_pid || 0;
        const msg = await invoke('apply_adaptive_profile', { gameName, gamePid: pid });
        logSuccess(msg);
        alert(msg);
      } catch (e) {
        logError(`Apply profile failed: ${e}`);
        alert('Помилка: ' + e);
      }
    });
  } catch (_) {}
}

// ── Styles ────────────────────────────────────────────────────────────────────

function addStyles() {
  if (document.getElementById('adp-styles')) return;
  const s = document.createElement('style');
  s.id = 'adp-styles';
  s.textContent = `
    .adp-input {
      width: 100%; box-sizing: border-box;
      background: rgba(255,255,255,0.05);
      border: 1px solid rgba(255,255,255,0.12);
      border-radius: 8px;
      padding: 9px 12px;
      color: var(--text-1);
      font-size: 13px;
      outline: none;
      transition: border-color 0.2s;
      display: block;
    }
    .adp-input:focus { border-color: var(--accent-bright); }
    @keyframes adp-glow {
      from { box-shadow: 0 0 4px rgba(167,139,250,0.4); }
      to   { box-shadow: 0 0 12px rgba(167,139,250,0.8); }
    }
  `;
  document.head.appendChild(s);
}
