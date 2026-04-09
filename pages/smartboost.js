import { invoke } from '@tauri-apps/api/core';
import { logInfo, logSuccess, logError } from '../js/terminal.js';

const PRIORITY_COLOR = {
  critical: '#f87171',
  high:     '#fbbf24',
  medium:   '#94a3b8',
  low:      '#64748b',
};
const PRIORITY_LABEL = {
  critical: 'КРИТИЧНО',
  high:     'ВАЖЛИВО',
  medium:   'СЕРЕДНІЙ',
  low:      'НИЗЬКИЙ',
};
const CATEGORY_ICON = {
  ram:     'RAM',
  cpu:     'CPU',
  gpu:     'GPU',
  disk:    'DISK',
  power:   'PWR',
  system:  'SYS',
  process: 'PROC',
};
const IMPACT_LABEL = {
  ram_optimize:       '+5-10 FPS',
  kill_background:    '+5-15 FPS',
  unpark_cores:       '+3-8 FPS',
  timer_resolution:   '-10ms input lag',
  power_plan:         '+5-20 FPS',
  nvidia_tweaks:      '+5-10 FPS',
  nvidia_low_latency: '-15ms lag',
  amd_tweaks:         '+5-10 FPS',
  disable_indexer:    '+швидкість завантаження',
  disable_superfetch: '+вільна RAM',
  visual_effects:     '+2-5 FPS',
  disable_hpet:       '-DPC latency',
  msi_mode:           '-frame stutter',
  win11_scheduler:    '-input lag',
};

// Scanning steps animation
const SCAN_STEPS = [
  'Перевіряємо CPU та ядра...',
  'Аналізуємо RAM та навантаження...',
  'Визначаємо GPU та драйвери...',
  'Скануємо диски та файлову систему...',
  'Перевіряємо план живлення...',
  'Аналізуємо системні налаштування...',
  'Розраховуємо оцінку оптимізації...',
];

let currentResult = null;
let scanStepTimer = null;

const APPLIED_KEY = 'sb_applied_ids';

function getAppliedIds() {
  try { return new Set(JSON.parse(localStorage.getItem(APPLIED_KEY) || '[]')); }
  catch { return new Set(); }
}

function saveAppliedId(id) {
  const ids = getAppliedIds();
  ids.add(id);
  localStorage.setItem(APPLIED_KEY, JSON.stringify([...ids]));
}

export async function renderSmartBoost(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">AI Smart Boost</h2>
      <p class="page-subtitle">ШІ сканує твій ПК і знаходить всі можливості для +FPS</p>
    </div>
    <div id="sb-root"></div>
  `;
  addStyles();
  await runScan();
}

// ── Scan ──────────────────────────────────────────────────────────────────────

async function runScan() {
  showScanningState();
  try {
    logInfo('Smart Boost: scanning PC...');
    const result = await invoke('smart_analyze');
    // Merge localStorage applied state with backend result
    const appliedIds = getAppliedIds();
    result.recommendations.forEach(r => {
      if (appliedIds.has(r.id)) r.applied = true;
    });
    currentResult = result;
    clearScanAnimation();
    renderResults(result);
    logSuccess(`Smart Boost: scan complete — score ${result.score}/100, ${result.recommendations.length} recommendations`);
  } catch (e) {
    clearScanAnimation();
    logError(`Smart Boost scan failed: ${e}`);
    document.getElementById('sb-root').innerHTML = `
      <div class="card" style="text-align:center;padding:32px;color:#f87171">
        <div style="font-size:32px;margin-bottom:12px">[!]</div>
        <div>Помилка сканування: ${e}</div>
        <button class="btn btn-primary btn-ripple" style="margin-top:16px" id="sb-retry">Спробувати знову</button>
      </div>
    `;
    document.getElementById('sb-retry')?.addEventListener('click', runScan);
  }
}

function showScanningState() {
  const root = document.getElementById('sb-root');
  if (!root) return;
  root.innerHTML = `
    <div class="card" style="text-align:center;padding:48px 24px">
      <!-- Animated ring -->
      <div class="sb-scan-ring">
        <svg width="120" height="120" viewBox="0 0 120 120">
          <circle cx="60" cy="60" r="52" fill="none" stroke="var(--border)" stroke-width="4"/>
          <circle cx="60" cy="60" r="52" fill="none" stroke="var(--accent-bright)" stroke-width="4"
            stroke-linecap="round" stroke-dasharray="326" stroke-dashoffset="326"
            class="sb-ring-progress" style="transform-origin:center;transform:rotate(-90deg)"/>
        </svg>
        <div class="sb-scan-icon">[SCAN]</div>
      </div>
      <div style="font-size:16px;font-weight:700;color:var(--text-1);margin-top:20px">Сканування ПК...</div>
      <div id="sb-scan-step" style="font-size:13px;color:var(--text-3);margin-top:8px;height:20px;transition:opacity 0.3s">
        Ініціалізація...
      </div>
    </div>
  `;

  // Animate scanning steps
  let stepIdx = 0;
  const ring = root.querySelector('.sb-ring-progress');
  const stepEl = document.getElementById('sb-scan-step');

  scanStepTimer = setInterval(() => {
    if (!document.getElementById('sb-scan-step')) { clearScanAnimation(); return; }
    stepIdx++;
    const pct = Math.min(stepIdx / SCAN_STEPS.length, 0.9);
    const offset = 326 * (1 - pct);
    if (ring) ring.style.strokeDashoffset = offset;
    if (stepEl && stepIdx <= SCAN_STEPS.length) {
      stepEl.style.opacity = '0';
      setTimeout(() => {
        if (stepEl) { stepEl.textContent = SCAN_STEPS[stepIdx - 1] || ''; stepEl.style.opacity = '1'; }
      }, 150);
    }
  }, 280);
}

function clearScanAnimation() {
  if (scanStepTimer) { clearInterval(scanStepTimer); scanStepTimer = null; }
}

// ── Render results ────────────────────────────────────────────────────────────

function renderResults(result) {
  const root = document.getElementById('sb-root');
  if (!root) return;

  const critRecs  = result.recommendations.filter(r => r.priority === 'critical' && !r.applied);
  const highRecs  = result.recommendations.filter(r => r.priority === 'high'     && !r.applied);
  const medRecs   = result.recommendations.filter(r => r.priority === 'medium'   && !r.applied);
  const lowRecs   = result.recommendations.filter(r => r.priority === 'low'      && !r.applied);
  const doneRecs  = result.recommendations.filter(r => r.applied);
  const totalPending = critRecs.length + highRecs.length + medRecs.length + lowRecs.length;

  root.innerHTML = `
    <!-- Score + summary row -->
    <div style="display:grid;grid-template-columns:auto 1fr;gap:16px;margin-bottom:16px">

      <!-- Circular score -->
      <div class="card" style="padding:20px;text-align:center;min-width:160px">
        <div style="position:relative;width:120px;height:120px;margin:0 auto">
          <svg width="120" height="120" viewBox="0 0 120 120">
            <circle cx="60" cy="60" r="52" fill="none" stroke="var(--border)" stroke-width="6"/>
            <circle cx="60" cy="60" r="52" fill="none"
              stroke="${scoreColor(result.score)}" stroke-width="6"
              stroke-linecap="round"
              stroke-dasharray="326"
              stroke-dashoffset="${326 * (1 - result.score / 100)}"
              style="transform-origin:center;transform:rotate(-90deg);transition:stroke-dashoffset 1s ease"
              id="sb-ring"/>
          </svg>
          <div style="position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center">
            <div id="sb-score-num" style="font-size:36px;font-weight:800;color:${scoreColor(result.score)};line-height:1">${result.score}</div>
            <div style="font-size:11px;color:var(--text-4)">/100</div>
          </div>
        </div>
        <div style="margin-top:12px;font-size:12px;color:var(--text-3);font-weight:600">${scoreLabel(result.score)}</div>
      </div>

      <!-- PC Summary -->
      <div class="card" style="padding:16px">
        <div style="font-size:12px;font-weight:700;color:var(--text-3);letter-spacing:0.05em;margin-bottom:10px">ТВІЙ ПК</div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:6px;font-size:12px">
          ${summaryRow('CPU', result.pc_summary.cpu_name, '')}
          ${summaryRow('Ядра', result.pc_summary.cpu_cores, '')}
          ${summaryRow('RAM', `${result.pc_summary.ram_gb} GB (${ramPressureLabel(result.pc_summary.ram_pressure)})`, '')}
          ${summaryRow('GPU', result.pc_summary.gpu_name, '')}
          ${summaryRow('Диск', result.pc_summary.has_ssd ? 'SSD' : 'HDD', '')}
          ${summaryRow('ОС', result.pc_summary.os_version, '')}
        </div>
      </div>
    </div>

    <!-- Action bar -->
    ${totalPending > 0 ? `
    <div style="display:flex;align-items:center;justify-content:space-between;
                background:rgba(99,102,241,0.08);border:1px solid rgba(99,102,241,0.2);
                border-radius:10px;padding:12px 16px;margin-bottom:16px">
      <div>
        <span style="font-size:14px;font-weight:700;color:var(--text-1)">${totalPending} покращень знайдено</span>
        <span style="font-size:12px;color:var(--text-3);margin-left:8px">
          ${critRecs.length ? `${critRecs.length} критичних · ` : ''}${highRecs.length ? `${highRecs.length} важливих` : ''}
        </span>
      </div>
      <button class="btn btn-primary btn-ripple" id="sb-apply-all" style="white-space:nowrap">
        Застосувати все безпечне
      </button>
    </div>` : ''}

    <!-- Recommendations by priority -->
    <div id="sb-recs-container">
      ${renderPrioritySection('critical', critRecs)}
      ${renderPrioritySection('high',     highRecs)}
      ${renderPrioritySection('medium',   medRecs)}
      ${renderPrioritySection('low',      lowRecs)}
      ${doneRecs.length ? renderDoneSection(doneRecs) : ''}
    </div>

    <!-- Rescan -->
    <div style="text-align:center;margin-top:20px;margin-bottom:8px">
      <button class="btn btn-ripple" id="sb-rescan">Сканувати знову</button>
    </div>
  `;

  // Wire buttons
  document.getElementById('sb-rescan')?.addEventListener('click', runScan);
  document.getElementById('sb-apply-all')?.addEventListener('click', () => applyAll(result.recommendations));

  document.querySelectorAll('.sb-apply-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const id = btn.dataset.id;
      await applySingle(id, btn);
    });
  });

  // Animate ring in
  requestAnimationFrame(() => {
    const ring = document.getElementById('sb-ring');
    if (ring) {
      ring.style.strokeDashoffset = 326;
      requestAnimationFrame(() => {
        ring.style.strokeDashoffset = 326 * (1 - result.score / 100);
      });
    }
  });
}

function renderPrioritySection(priority, recs) {
  if (!recs.length) return '';
  const color = PRIORITY_COLOR[priority];
  return `
    <div class="sb-priority-section" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
        <span style="font-size:12px;font-weight:700;color:${color};letter-spacing:0.05em">
          ${PRIORITY_LABEL[priority]}
        </span>
        <div style="flex:1;height:1px;background:${color}22"></div>
        <span style="font-size:11px;color:var(--text-4)">${recs.length} пунктів</span>
      </div>
      ${recs.map(rec => renderRecCard(rec)).join('')}
    </div>
  `;
}

function renderDoneSection(recs) {
  return `
    <div style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">

        <span style="font-size:12px;font-weight:700;color:var(--success);letter-spacing:0.05em">
          ВЖЕ ЗАСТОСОВАНО
        </span>
        <div style="flex:1;height:1px;background:rgba(74,222,128,0.15)"></div>
        <span style="font-size:11px;color:var(--text-4)">${recs.length} пунктів</span>
      </div>
      ${recs.map(rec => renderRecCard(rec, true)).join('')}
    </div>
  `;
}

function renderRecCard(rec, isApplied = false) {
  const impact = IMPACT_LABEL[rec.id];
  return `
    <div class="sb-rec-card ${isApplied ? 'sb-rec-done' : ''}" id="rec-${rec.id}">
      <div class="sb-rec-header">
        <div class="sb-rec-left">
          <span class="sb-rec-icon">${CATEGORY_ICON[rec.category] || '[SYS]'}</span>
          <div style="flex:1">
            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap">
              <span class="sb-rec-title">${rec.title}</span>
              ${impact ? `<span class="sb-impact-badge">${impact}</span>` : ''}
            </div>
            <div class="sb-rec-reason">${rec.reason}</div>
          </div>
        </div>
        <div class="sb-rec-right">
          ${isApplied
            ? `<span style="font-size:12px;color:var(--success);font-weight:600">Активно</span>`
            : `<button class="btn btn-ripple sb-apply-btn" data-id="${rec.id}" style="font-size:11px;padding:5px 14px">
                Застосувати
              </button>`
          }
        </div>
      </div>
      <div class="sb-rec-desc">${rec.description}</div>
    </div>
  `;
}

function summaryRow(label, value, icon) {
  return `
    <div style="padding:7px 10px;background:var(--bg-surface);border-radius:6px;display:flex;align-items:center;gap:8px">
      <span style="font-size:14px">${icon}</span>
      <div>
        <div style="color:var(--text-4);font-size:10px;line-height:1">${label}</div>
        <div style="color:var(--text-1);font-weight:600;font-size:12px;margin-top:1px">${value}</div>
      </div>
    </div>
  `;
}

// ── Apply ─────────────────────────────────────────────────────────────────────

async function applySingle(id, btn) {
  const origText = btn.textContent;
  btn.disabled = true;
  btn.textContent = '...';
  try {
    const msg = await invoke('apply_recommendation', { id });
    logSuccess(msg);
    saveAppliedId(id); // persist so rescan knows it's done
    btn.textContent = 'Done';
    btn.style.color = 'var(--success)';
    const card = document.getElementById(`rec-${id}`);
    if (card) {
      card.classList.add('sb-rec-done');
      card.querySelector('.sb-rec-right').innerHTML =
        `<span style="font-size:12px;color:var(--success);font-weight:600">Застосовано</span>`;
    }
    // Update score visually (+points)
    updateScoreAfterApply(id);
  } catch (e) {
    logError(`Failed to apply ${id}: ${e}`);
    btn.disabled = false;
    btn.textContent = origText;
  }
}

async function applyAll(recs) {
  const safeRecs = recs.filter(r => r.safe && !r.applied);
  logInfo(`Applying ${safeRecs.length} safe recommendations...`);
  for (const rec of safeRecs) {
    const btn = document.querySelector(`.sb-apply-btn[data-id="${rec.id}"]`);
    if (btn && !btn.disabled) {
      await applySingle(rec.id, btn);
      await new Promise(r => setTimeout(r, 200));
    }
  }
  logSuccess('All safe recommendations applied!');
}

function updateScoreAfterApply(id) {
  if (!currentResult) return;
  const rec = currentResult.recommendations.find(r => r.id === id);
  if (!rec || rec.applied) return;
  rec.applied = true;
  saveAppliedId(id);

  const penalty = { critical: 15, high: 8, medium: 3, low: 1 };
  const gain = penalty[rec.priority] || 0;
  currentResult.score = Math.min(100, currentResult.score + gain);

  const numEl = document.getElementById('sb-score-num');
  const ring   = document.getElementById('sb-ring');
  if (numEl) {
    numEl.textContent = currentResult.score;
    numEl.style.color = scoreColor(currentResult.score);
  }
  if (ring) {
    ring.style.stroke = scoreColor(currentResult.score);
    ring.style.strokeDashoffset = 326 * (1 - currentResult.score / 100);
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function scoreColor(score) {
  if (score >= 75) return '#4ade80';
  if (score >= 50) return '#fbbf24';
  return '#f87171';
}

function scoreLabel(score) {
  if (score >= 85) return 'ПК добре оптимізований';
  if (score >= 65) return 'Є простір для покращення';
  if (score >= 40) return 'Потребує оптимізації';
  return 'КРИТИЧНО — оптимізуй зараз';
}

function ramPressureLabel(p) {
  return p === 'high' ? 'Критично' : p === 'medium' ? 'Помірно' : 'Норма';
}

// ── Styles ────────────────────────────────────────────────────────────────────

function addStyles() {
  if (document.getElementById('sb-styles')) return;
  const s = document.createElement('style');
  s.id = 'sb-styles';
  s.textContent = `
    .sb-scan-ring { position:relative; width:120px; height:120px; margin:0 auto; }
    .sb-scan-icon {
      position:absolute; inset:0; display:flex; align-items:center;
      justify-content:center; font-size:36px;
      animation: sb-pulse 1.2s ease-in-out infinite;
    }
    .sb-ring-progress { animation: sb-spin 1.5s linear infinite; }
    @keyframes sb-spin { to { stroke-dashoffset: 0; } }
    @keyframes sb-pulse { 0%,100% { opacity:1; } 50% { opacity:0.5; } }

    .sb-rec-card {
      background: var(--bg-surface);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      padding: 12px 14px;
      margin-bottom: 8px;
      transition: border-color 0.2s, opacity 0.3s;
    }
    .sb-rec-card:hover { border-color: var(--border-bright); }
    .sb-rec-done { opacity: 0.5; }
    .sb-rec-header { display:flex; justify-content:space-between; align-items:flex-start; gap:12px; }
    .sb-rec-left { display:flex; align-items:flex-start; gap:10px; flex:1; }
    .sb-rec-icon { font-size:18px; flex-shrink:0; margin-top:2px; }
    .sb-rec-title { font-size:13px; font-weight:600; color:var(--text-1); }
    .sb-rec-reason { font-size:11px; color:var(--text-3); margin-top:2px; }
    .sb-rec-right { flex-shrink:0; }
    .sb-rec-desc {
      font-size:11px; color:var(--text-4);
      margin-top:8px; padding-top:8px;
      border-top:1px solid var(--border);
    }
    .sb-impact-badge {
      font-size:10px; font-weight:700;
      background: rgba(99,102,241,0.15);
      color: #a78bfa;
      padding: 1px 7px; border-radius: 20px;
      white-space: nowrap;
    }
  `;
  document.head.appendChild(s);
}
