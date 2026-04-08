import { invoke } from '@tauri-apps/api/core';
import { logInfo, logSuccess, logError } from '../js/terminal.js';

const PRIORITY_COLOR = {
  critical: '#f87171',
  high:     '#fbbf24',
  medium:   '#a0a0a0',
  low:      '#606060',
};
const PRIORITY_LABEL = {
  critical: '🔴 КРИТИЧНО',
  high:     '🟡 ВАЖЛИВО',
  medium:   '⚪ СЕРЕДНІЙ',
  low:      '⚫ НИЗЬКИЙ',
};
const CATEGORY_ICON = {
  ram:     '💾',
  cpu:     '⚡',
  gpu:     '🎮',
  disk:    '💿',
  power:   '🔋',
  system:  '🖥',
  process: '🔪',
};

export async function renderSmartBoost(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">🤖 AI Аналіз</h2>
      <p class="page-subtitle">ШІ сканує твій ПК і автоматично підбирає найкращий профіль</p>
    </div>

    <!-- Scan button -->
    <div style="text-align:center;padding:32px 0" id="sb-scan-area">
      <button class="btn-optimize" id="sb-btn-scan" style="font-size:16px;padding:16px 48px">
        🔍 Сканувати ПК
      </button>
      <p style="color:var(--text-3);font-size:13px;margin-top:12px">
        Аналізуємо CPU, GPU, RAM, диск та ОС — займає ~2 секунди
      </p>
    </div>

    <!-- Results (hidden until scan) -->
    <div id="sb-results" style="display:none">

      <!-- Score card -->
      <div class="card card-enter" id="sb-score-card"
           style="margin-bottom:16px;text-align:center;padding:24px">
        <div style="font-size:13px;color:var(--text-3);margin-bottom:8px">
          ПОТОЧНИЙ РІВЕНЬ ОПТИМІЗАЦІЇ
        </div>
        <div id="sb-score-value" style="font-size:56px;font-weight:800;line-height:1"></div>
        <div style="font-size:13px;color:var(--text-3);margin-top:4px">/100</div>
        <!-- Score bar -->
        <div style="height:6px;background:var(--border);border-radius:3px;margin:16px 0 0">
          <div id="sb-score-bar" style="height:100%;border-radius:3px;transition:width 1s ease"></div>
        </div>
      </div>

      <!-- PC Summary -->
      <div class="section" style="margin-bottom:16px" id="sb-summary-section">
        <h3 class="section-title">Твій ПК</h3>
        <div id="sb-summary" style="display:grid;grid-template-columns:1fr 1fr;gap:8px;font-size:13px"></div>
      </div>

      <!-- Apply All -->
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
        <div style="font-size:15px;font-weight:600;color:var(--text-1)">
          Рекомендації
          <span id="sb-count" style="font-size:12px;color:var(--text-3);margin-left:6px"></span>
        </div>
        <button class="btn btn-primary btn-ripple" id="sb-btn-apply-all">
          ✅ Застосувати все безпечне
        </button>
      </div>

      <!-- Recommendations list -->
      <div id="sb-recs"></div>

      <!-- Re-scan -->
      <div style="text-align:center;margin-top:24px">
        <button class="btn btn-ripple" id="sb-btn-rescan">🔄 Сканувати знову</button>
      </div>
    </div>
  `;

  addStyles();
  document.getElementById('sb-btn-scan')?.addEventListener('click', runScan);
  document.getElementById('sb-btn-rescan')?.addEventListener('click', runScan);
}

// ── Scan ──────────────────────────────────────────────────────────────────────

async function runScan() {
  const btn = document.getElementById('sb-btn-scan');
  if (btn) { btn.disabled = true; btn.textContent = '⏳ Сканування...'; }

  try {
    logInfo('Smart Boost: scanning PC...');
    const result = await invoke('smart_analyze');
    renderResults(result);
    logSuccess(`Smart Boost: scan complete — score ${result.score}/100, ${result.recommendations.length} recommendations`);
  } catch (e) {
    logError(`Smart Boost scan failed: ${e}`);
    const btn2 = document.getElementById('sb-btn-scan');
    if (btn2) { btn2.disabled = false; btn2.textContent = '🔍 Сканувати ПК'; }
  }
}

// ── Render results ────────────────────────────────────────────────────────────

function renderResults(result) {
  document.getElementById('sb-scan-area').style.display = 'none';
  document.getElementById('sb-results').style.display  = 'block';

  renderScore(result.score);
  renderSummary(result.pc_summary);
  renderRecs(result.recommendations);

  document.getElementById('sb-count').textContent =
    `(${result.recommendations.length} пунктів)`;

  document.getElementById('sb-btn-apply-all')?.addEventListener('click', () =>
    applyAll(result.recommendations));
  document.getElementById('sb-btn-rescan')?.addEventListener('click', () => {
    document.getElementById('sb-scan-area').style.display = 'block';
    document.getElementById('sb-results').style.display  = 'none';
    const btn = document.getElementById('sb-btn-scan');
    if (btn) { btn.disabled = false; btn.textContent = '🔍 Сканувати ПК'; }
  });
}

function renderScore(score) {
  const el  = document.getElementById('sb-score-value');
  const bar = document.getElementById('sb-score-bar');
  const color = score >= 75 ? 'var(--success)' : score >= 50 ? '#fbbf24' : '#f87171';

  if (el)  { el.textContent = score; el.style.color = color; }
  if (bar) { bar.style.width = score + '%'; bar.style.background = color; }
}

function renderSummary(pc) {
  const el = document.getElementById('sb-summary');
  if (!el) return;

  const rows = [
    ['CPU',    pc.cpu_name || 'Unknown'],
    ['Ядра',   pc.cpu_cores],
    ['RAM',    `${pc.ram_gb} GB (${pc.ram_pressure})`],
    ['GPU',    pc.gpu_name || 'Unknown'],
    ['Диск',   pc.has_ssd ? '✅ SSD' : '⚠️ HDD — знайдено повільний диск'],
    ['ОС',     pc.os_version],
  ];

  el.innerHTML = rows.map(([k, v]) => `
    <div style="padding:8px 12px;background:var(--bg-surface);border-radius:6px">
      <div style="color:var(--text-3);font-size:11px">${k}</div>
      <div style="color:var(--text-1);font-weight:600;margin-top:2px">${v}</div>
    </div>
  `).join('');
}

function renderRecs(recs) {
  const el = document.getElementById('sb-recs');
  if (!el) return;

  el.innerHTML = recs.map(rec => `
    <div class="sb-rec-card" id="rec-${rec.id}" data-applied="${rec.applied}">
      <div class="sb-rec-header">
        <div class="sb-rec-left">
          <span class="sb-rec-icon">${CATEGORY_ICON[rec.category] || '⚙️'}</span>
          <div>
            <div class="sb-rec-title">${rec.title}</div>
            <div class="sb-rec-reason">${rec.reason}</div>
          </div>
        </div>
        <div class="sb-rec-right">
          <span class="sb-priority-badge" style="color:${PRIORITY_COLOR[rec.priority]}">
            ${PRIORITY_LABEL[rec.priority]}
          </span>
          <button class="btn btn-ripple sb-apply-btn"
                  data-id="${rec.id}"
                  style="font-size:11px;padding:4px 12px;margin-top:6px">
            Застосувати
          </button>
        </div>
      </div>
      <div class="sb-rec-desc">${rec.description}</div>
    </div>
  `).join('');

  // Wire apply buttons
  el.querySelectorAll('.sb-apply-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const id = btn.dataset.id;
      await applySingle(id, btn);
    });
  });
}

// ── Apply ─────────────────────────────────────────────────────────────────────

async function applySingle(id, btn) {
  const origText = btn.textContent;
  btn.disabled = true;
  btn.textContent = '⏳';

  try {
    const msg = await invoke('apply_recommendation', { id });
    logSuccess(msg);
    btn.textContent = '✅ Застосовано';
    btn.style.color = 'var(--success)';
    const card = document.getElementById(`rec-${id}`);
    if (card) card.style.opacity = '0.5';
  } catch (e) {
    logError(`Failed to apply ${id}: ${e}`);
    btn.disabled = false;
    btn.textContent = origText;
  }
}

async function applyAll(recs) {
  const safeRecs = recs.filter(r => r.safe && !r.applied);
  logInfo(`Applying ${safeRecs.length} recommendations...`);

  for (const rec of safeRecs) {
    const btn = document.querySelector(`[data-id="${rec.id}"]`);
    await applySingle(rec.id, btn || document.createElement('button'));
    await new Promise(r => setTimeout(r, 300)); // small delay between applies
  }

  logSuccess('All safe recommendations applied!');
}

// ── Styles ────────────────────────────────────────────────────────────────────

function addStyles() {
  if (document.getElementById('sb-styles')) return;
  const s = document.createElement('style');
  s.id = 'sb-styles';
  s.textContent = `
    .sb-rec-card {
      background: var(--bg-surface);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      padding: 14px 16px;
      margin-bottom: 10px;
      transition: border-color 0.2s, opacity 0.3s;
    }
    .sb-rec-card:hover { border-color: var(--border-bright); }
    .sb-rec-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 12px;
    }
    .sb-rec-left { display:flex; align-items:flex-start; gap:12px; flex:1; }
    .sb-rec-icon { font-size:20px; flex-shrink:0; margin-top:2px; }
    .sb-rec-title { font-size:14px; font-weight:600; color:var(--text-1); }
    .sb-rec-reason { font-size:12px; color:var(--text-3); margin-top:2px; }
    .sb-rec-right { display:flex; flex-direction:column; align-items:flex-end; flex-shrink:0; }
    .sb-priority-badge { font-size:11px; font-weight:700; white-space:nowrap; }
    .sb-rec-desc {
      font-size:12px; color:var(--text-3);
      margin-top:10px; padding-top:10px;
      border-top:1px solid var(--border);
    }
  `;
  document.head.appendChild(s);
}
