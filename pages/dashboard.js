import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';

export async function renderDashboard(container) {
  container.innerHTML = `
    <!-- System info cards -->
    <div class="card-grid" id="sys-cards" style="margin-bottom:24px">
      <div class="card card-enter hex-bg">
        <div class="card-icon">[CPU]</div>
        <div class="card-title">${t('dash.processor')}</div>
        <div class="card-value" id="dash-cpu">${t('txt.loading')}</div>
        <div class="card-subtitle" id="dash-cpu-name">---</div>
        <div class="stat-bar"><div class="stat-bar-fill" id="dash-cpu-bar" style="width:0%"></div></div>
      </div>
      <div class="card card-enter hex-bg" style="animation-delay:0.1s">
        <div class="card-icon">[RAM]</div>
        <div class="card-title">${t('dash.memory')}</div>
        <div class="card-value" id="dash-ram">${t('txt.loading')}</div>
        <div class="card-subtitle" id="dash-ram-detail">---</div>
        <div class="stat-bar"><div class="stat-bar-fill" id="dash-ram-bar" style="width:0%"></div></div>
      </div>
      <div class="card card-enter hex-bg" style="animation-delay:0.2s">
        <div class="card-icon">[GPU]</div>
        <div class="card-title">${t('dash.graphics')}</div>
        <div class="card-value" id="dash-gpu">${t('txt.loading')}</div>
        <div class="card-subtitle" id="dash-gpu-vendor">---</div>
      </div>
      <div class="card card-enter hex-bg" style="animation-delay:0.3s">
        <div class="card-icon">[OS]</div>
        <div class="card-title">${t('dash.system')}</div>
        <div class="card-value" id="dash-os">${t('txt.loading')}</div>
        <div class="card-subtitle" id="dash-hostname">---</div>
      </div>
    </div>

    <!-- Main optimize block -->
    <div class="optimize-hero">
      <div class="optimize-hero-status" id="opt-status-text">Готово до оптимізації</div>
      <button class="btn-optimize" id="btn-quick-optimize">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
        ОПТИМІЗУВАТИ
      </button>
      <div class="optimize-progress-wrap" id="optimize-progress" style="display:none">
        <div class="optimize-step-label" id="opt-step-label">Підготовка...</div>
        <div class="stat-bar" style="height:6px;border-radius:3px;margin-top:8px">
          <div id="opt-progress-bar" class="stat-bar-fill" style="width:0%;transition:width 0.4s ease"></div>
        </div>
        <div class="optimize-step-count" id="opt-step-count">0/8</div>
      </div>
      <div class="optimize-actions">
        <button class="btn-opt-secondary" id="btn-backup">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
          Зробити бекап
        </button>
        <button class="btn-opt-secondary" id="btn-kill-bloat">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/></svg>
          Вбити фонові процеси
        </button>
      </div>
    </div>

    <!-- AI Analysis panel (auto-loads) -->
    <div class="ai-panel card-enter" id="ai-panel" style="margin-bottom:24px">
      <div class="ai-panel-header">
        <div style="display:flex;align-items:center;gap:10px">
          <span style="font-size:18px">[AI]</span>
          <div>
            <div style="font-weight:700;font-size:14px;color:var(--text-1)">AI Аналіз ПК</div>
            <div style="font-size:11px;color:var(--text-3)" id="ai-subtitle">Сканування...</div>
          </div>
        </div>
        <div style="display:flex;align-items:center;gap:10px">
          <div id="ai-score-badge" style="display:none">
            <span id="ai-score-num" style="font-size:28px;font-weight:800;line-height:1">—</span>
            <span style="font-size:12px;color:var(--text-3)">/100</span>
          </div>
          <button class="btn btn-ripple" id="ai-btn-rescan" style="font-size:11px;padding:4px 10px">
            Оновити
          </button>
        </div>
      </div>

      <!-- Score bar -->
      <div id="ai-score-bar-wrap" style="display:none;margin:10px 0 14px">
        <div style="height:4px;background:var(--border);border-radius:2px">
          <div id="ai-score-bar" style="height:100%;border-radius:2px;transition:width 1s ease;width:0%"></div>
        </div>
      </div>

      <!-- Recommendations list (compact) -->
      <div id="ai-recs-list">
        <div class="ai-loading">
          <div class="ai-spinner"></div>
          <span>Аналізуємо CPU, RAM, GPU, диск...</span>
        </div>
      </div>

      <!-- Apply all -->
      <div id="ai-apply-all-row" style="display:none;margin-top:12px;display:none;justify-content:flex-end">
        <button class="btn btn-primary btn-ripple" id="ai-btn-apply-all" style="font-size:12px">
          Застосувати всі безпечні
        </button>
      </div>
    </div>

    <!-- Instant vs reboot tweaks -->
    <div class="tweaks-split">
      <div class="tweaks-group">
        <div class="tweaks-group-header instant">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
          Діє одразу
        </div>
        <div class="tweaks-list" id="tweaks-instant">
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Power Plan — Максимальна продуктивність</span>
            <span class="tweak-info" data-tip="Перемикає план живлення Windows на Ultimate Performance — процесор завжди працює на максимальній частоті без зниження.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Timer Resolution — 0.5ms</span>
            <span class="tweak-info" data-tip="Зменшує системний таймер з 15.6ms до 0.5ms — ігровий цикл стає точнішим, фреймтайм стабільнішим. +5-15 FPS.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>RAM — очистка пам'яті</span>
            <span class="tweak-info" data-tip="Очищає standby-пам'ять (кеш Windows що займає RAM). Звільняє до 2-4 ГБ для гри.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Мережа — Nagle, TCP No Delay</span>
            <span class="tweak-info" data-tip="Вимикає алгоритм Nagle — пакети надсилаються одразу без буферизації. Зменшує пінг на 5-20ms.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>CPU — розпарковка ядер</span>
            <span class="tweak-info" data-tip="Забороняє Windows вимикати ядра процесора в режимі економії. Всі ядра завжди активні.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Реєстр — Game DVR, GPU Priority</span>
            <span class="tweak-info" data-tip="Вимикає Xbox Game DVR (фонове записування), підвищує пріоритет GPU в реєстрі. +3-5 FPS.">!</span>
          </div>
        </div>
      </div>
      <div class="tweaks-group">
        <div class="tweaks-group-header reboot">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
          Після перезавантаження
        </div>
        <div class="tweaks-list">
          <div class="tweak-item">
            <span class="tweak-dot reboot"></span>
            <span>MSI Mode — зменшення input lag GPU</span>
            <span class="tweak-info" data-tip="Message Signaled Interrupts — GPU надсилає переривання через RAM замість шини PCI. Зменшує input lag на 2-5ms. Потрібне перезавантаження.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot reboot"></span>
            <span>HPET — вимкнення (+5-10 FPS на AMD)</span>
            <span class="tweak-info" data-tip="High Precision Event Timer — на AMD процесорах його вимкнення дає +5-10 FPS. На Intel ефект менший. Потрібне перезавантаження.">!</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot reboot"></span>
            <span>HW GPU Scheduling</span>
          </div>
        </div>
      </div>
    </div>

    <!-- GPU Upscaling Card -->
    <div class="upscaling-card" id="upscaling-card" style="display:none">
      <div class="upscaling-header">
        <span class="upscaling-icon">[UP]</span>
        <div>
          <div class="upscaling-title" id="upscaling-tech">Завантаження...</div>
          <div class="upscaling-sub">Апскейлінг на рівні драйвера — працює в усіх іграх</div>
        </div>
        <label class="toggle-switch" style="margin-left:auto">
          <input type="checkbox" id="upscaling-toggle">
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="upscaling-sharpness">
        <span class="upscaling-sharp-label">Різкість: <b id="upscaling-sharp-val">50</b>%</span>
        <input type="range" id="upscaling-sharpness" min="0" max="100" value="50" class="sharp-slider">
      </div>
      <div class="upscaling-note" id="upscaling-note"></div>
    </div>

    <!-- Web download banner -->
    <div id="web-hero-banner" style="display:none; background: linear-gradient(135deg, rgba(77, 255, 145, 0.1) 0%, rgba(0, 0, 0, 0) 100%); border: 1px solid var(--accent-primary); border-radius: var(--radius-md); padding: 24px; margin-top: 24px; position: relative; overflow: hidden;">
        <div style="position:relative; z-index:2;">
            <h3 style="color:var(--accent-primary); margin-bottom:8px; font-size:18px;">Завантаж десктоп додаток</h3>
            <p style="color:var(--text-main); margin-bottom:16px; max-width:600px; font-size:13px;">Веб-версія — тільки демо. Завантаж програму щоб отримати реальний буст FPS.</p>
            <a href="https://rustopti.fun/RustOpti_2.2.3_x64-setup.exe" class="btn btn-primary btn-ripple" style="padding:10px 20px; font-weight:600; font-size:13px;">
                ↓ Завантажити .EXE — v2.2.2
            </a>
        </div>
    </div>
  `;

  // Web Mode Hero Visibility
  if (!window.__TAURI_INTERNALS__) {
      const hero = document.getElementById('web-hero-banner');
      if (hero) hero.style.display = 'block';
  }

  // GPU Upscaling Card (only in Tauri)
  if (window.__TAURI_INTERNALS__) {
    try {
      const ups = await api.getUpscalingStatus();
      if (ups.vendor === 'NVIDIA' || ups.vendor === 'AMD') {
        const card = document.getElementById('upscaling-card');
        card.style.display = 'block';
        document.getElementById('upscaling-tech').textContent = ups.technology;
        const toggle = document.getElementById('upscaling-toggle');
        const sharpSlider = document.getElementById('upscaling-sharpness');
        const sharpVal = document.getElementById('upscaling-sharp-val');
        const note = document.getElementById('upscaling-note');
        toggle.checked = ups.enabled;
        sharpSlider.value = ups.sharpness;
        sharpVal.textContent = ups.sharpness;

        toggle.addEventListener('change', async () => {
          try {
            const msg = await api.setUpscaling(toggle.checked, parseInt(sharpSlider.value));
            note.textContent = 'OK: ' + msg;
            note.style.color = 'var(--accent-primary)';
          } catch(e) {
            note.textContent = 'Err: ' + e;
            note.style.color = '#f87171';
          }
        });

        sharpSlider.addEventListener('input', () => {
          sharpVal.textContent = sharpSlider.value;
        });
        sharpSlider.addEventListener('change', async () => {
          try {
            const msg = await api.setUpscaling(toggle.checked, parseInt(sharpSlider.value));
            note.textContent = 'OK: ' + msg;
            note.style.color = 'var(--accent-primary)';
          } catch(e) {}
        });
      }
    } catch(e) { /* GPU upscaling not available */ }
  }

  try {
    const info = await api.getSystemInfo();
    logInfo('System info loaded');

    const cpuUsage = info.cpu_usage ?? 0;
    const cpuName = info.cpu_name ?? 'Unknown CPU';
    const usedRam = info.used_ram_mb ?? 0;
    const totalRam = info.total_ram_mb ?? 1;
    const gpuInfo = info.gpu_info || t('txt.unknown');
    const osName = info.os_name ?? 'Unknown OS';
    const osVersion = info.os_version ?? '';
    const hostname = info.hostname ?? '';
    const disks = Array.isArray(info.disks) ? info.disks : [];

    document.getElementById('dash-cpu').textContent = `${cpuUsage.toFixed(1)}%`;
    document.getElementById('dash-cpu-name').textContent = cpuName;
    document.getElementById('dash-cpu-bar').style.width = `${cpuUsage}%`;

    const ramPercent = ((usedRam / totalRam) * 100).toFixed(0);
    document.getElementById('dash-ram').textContent = `${ramPercent}%`;
    document.getElementById('dash-ram-detail').textContent = `${usedRam} / ${totalRam} MB`;
    document.getElementById('dash-ram-bar').style.width = `${ramPercent}%`;

    document.getElementById('dash-gpu').textContent = gpuInfo;
    document.getElementById('dash-os').textContent = `${osName} ${osVersion}`;
    document.getElementById('dash-hostname').textContent = hostname;

    const disksHtml = disks.map(d => `
      <div class="card card-enter">
        <div class="card-icon">[DISK]</div>
        <div class="card-title">${d.mount_point}</div>
        <div class="card-value">${d.free_gb.toFixed(1)} GB ${t('dash.free')}</div>
        <div class="card-subtitle">${d.total_gb.toFixed(1)} GB ${t('dash.total')}</div>
        <div class="stat-bar"><div class="stat-bar-fill" style="width:${((1 - d.free_gb/d.total_gb)*100).toFixed(0)}%"></div></div>
      </div>
    `).join('');
    document.getElementById('dash-disks').innerHTML = disksHtml;
  } catch (e) {
    logError(`Failed to load system info: ${e}`);
  }

  document.getElementById('btn-quick-optimize')?.addEventListener('click', async () => {
    const btn = document.getElementById('btn-quick-optimize');
    const statusText = document.getElementById('opt-status-text');
    const progressWrap = document.getElementById('optimize-progress');
    const stepLabel = document.getElementById('opt-step-label');
    const stepCount = document.getElementById('opt-step-count');
    const progressBar = document.getElementById('opt-progress-bar');
    const rebootNote = document.getElementById('opt-reboot-note');

    btn.disabled = true;

    const steps = [
      { name: 'Backup',   fn: () => api.backupAllBeforeOptimization(), label: 'Створення бекапу...' },
      { name: 'Registry', fn: () => api.applyRegistryTweaks(),         label: 'Оптимізація реєстру...' },
      { name: 'GPU',      fn: () => api.applyGpuTweaks(),              label: 'Налаштування GPU...' },
      { name: 'Power',    fn: () => api.applyPowerTweaks(),            label: 'Режим максимальної продуктивності...' },
      { name: 'Network',  fn: () => api.applyNetworkTweaks(),          label: 'Оптимізація мережі...' },
      { name: 'CPU',      fn: () => api.unparkAllCores(),              label: 'Розпарковка ядер CPU...' },
      { name: 'Timer',    fn: () => api.boostTimerResolution(),        label: 'Прискорення таймера...' },
      { name: 'HPET',     fn: () => api.disableHpet(),                 label: 'Вимкнення HPET...' },
    ];

    progressWrap.style.display = 'block';
    stepCount.textContent = `0/${steps.length}`;
    logInfo('Починаємо оптимізацію...');

    await new Promise(r => setTimeout(r, 50));

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      stepLabel.textContent = step.label;
      stepCount.textContent = `${i + 1}/${steps.length}`;
      progressBar.style.width = `${Math.round((i / steps.length) * 100)}%`;
      logInfo(`[${i + 1}/${steps.length}] ${step.label}`);
      await new Promise(r => setTimeout(r, 16));
      try {
        const results = await step.fn();
        if (Array.isArray(results)) {
          results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
        } else if (results?.message) {
          results.success ? logSuccess(results.message) : logError(results.message);
        }
      } catch (e) {
        logError(`[${step.name}] Failed: ${e}`);
      }
    }

    progressBar.style.width = '100%';
    stepLabel.textContent = 'Готово!';
    stepCount.textContent = `${steps.length}/${steps.length}`;
    statusText.textContent = 'Систему оптимізовано';
    statusText.style.color = 'var(--accent-primary)';
    rebootNote.style.display = 'block';
    logSuccess('Оптимізацію завершено! Перезавантаж ПК для повного ефекту.');

    setTimeout(() => {
      progressWrap.style.display = 'none';
    }, 4000);

    btn.disabled = false;
  });

  document.getElementById('btn-backup')?.addEventListener('click', async () => {
    logInfo('Creating backup...');
    try {
      const results = await api.backupAllBeforeOptimization();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
    } catch (e) {
      logError(`Backup failed: ${e}`);
    }
  });

  document.getElementById('btn-kill-bloat')?.addEventListener('click', async () => {
    logInfo('Killing bloatware processes...');
    try {
      const results = await api.killBloatware();
      results.forEach(r => logSuccess(r));
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  // Auto-run AI scan on dashboard load (only in Tauri)
  if (window.__TAURI_INTERNALS__) {
    runAiScan();
    document.getElementById('ai-btn-rescan')?.addEventListener('click', runAiScan);
  } else {
    document.getElementById('ai-panel').style.display = 'none';
  }
}

// ── AI Panel ──────────────────────────────────────────────────────────────────

const AI_PRIORITY_COLOR = { critical:'#f87171', high:'#fbbf24', medium:'#a0a0a0', low:'#606060' };
const AI_PRIORITY_LABEL = { critical:'КРИТИЧНО', high:'ВАЖЛИВО', medium:'СЕРЕДНІЙ', low:'НИЗЬКИЙ' };
const AI_CAT_ICON = { ram:'', cpu:'', gpu:'', disk:'', power:'', system:'', process:'' };

let _aiRecs = [];

async function runAiScan() {
  const list    = document.getElementById('ai-recs-list');
  const subtitle = document.getElementById('ai-subtitle');
  const scoreBadge = document.getElementById('ai-score-badge');
  const scoreBar   = document.getElementById('ai-score-bar-wrap');
  const applyRow   = document.getElementById('ai-apply-all-row');
  const rescanBtn  = document.getElementById('ai-btn-rescan');

  if (!list) return;

  if (rescanBtn) { rescanBtn.disabled = true; rescanBtn.textContent = '...'; }
  if (subtitle) subtitle.textContent = 'Сканування...';
  if (scoreBadge) scoreBadge.style.display = 'none';
  if (scoreBar) scoreBar.style.display = 'none';
  if (applyRow) applyRow.style.display = 'none';

  list.innerHTML = `<div class="ai-loading"><div class="ai-spinner"></div><span>Аналізуємо CPU, RAM, GPU, диск...</span></div>`;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const result = await invoke('smart_analyze');
    _aiRecs = result.recommendations || [];
    renderAiResults(result);
    logInfo(`AI: score ${result.score}/100, ${_aiRecs.length} рекомендацій`);
  } catch (e) {
    list.innerHTML = `<div style="color:var(--text-3);font-size:13px;padding:12px 0">Не вдалося запустити аналіз: ${e}</div>`;
  } finally {
    if (rescanBtn) { rescanBtn.disabled = false; rescanBtn.textContent = 'Оновити'; }
  }
}

function renderAiResults(result) {
  const score = result.score ?? 0;
  const recs  = result.recommendations ?? [];

  // Score
  const scoreNum = document.getElementById('ai-score-num');
  const scoreBadge = document.getElementById('ai-score-badge');
  const scoreBar   = document.getElementById('ai-score-bar');
  const scoreBarWrap = document.getElementById('ai-score-bar-wrap');
  const subtitle   = document.getElementById('ai-subtitle');

  const color = score >= 75 ? 'var(--success)' : score >= 50 ? '#fbbf24' : '#f87171';
  if (scoreNum)   { scoreNum.textContent = score; scoreNum.style.color = color; }
  if (scoreBadge) scoreBadge.style.display = 'flex';
  if (scoreBar)   { scoreBar.style.width = score + '%'; scoreBar.style.background = color; }
  if (scoreBarWrap) scoreBarWrap.style.display = 'block';
  if (subtitle) {
    const critical = recs.filter(r => r.priority === 'critical').length;
    const high     = recs.filter(r => r.priority === 'high').length;
    subtitle.textContent = critical
      ? `${critical} критичних проблем · ${high} важливих`
      : high
      ? `${high} важливих покращень`
      : recs.length ? `${recs.length} рекомендацій` : 'Система оптимізована';
  }

  // Render recs (show all, sorted by priority)
  const order = { critical: 0, high: 1, medium: 2, low: 3 };
  const sorted = [...recs].sort((a, b) => (order[a.priority] ?? 4) - (order[b.priority] ?? 4));

  const list = document.getElementById('ai-recs-list');
  if (!list) return;

  if (!sorted.length) {
    list.innerHTML = `<div style="color:var(--success);font-size:13px;padding:12px 0;text-align:center">
      Система оптимізована! Рекомендацій немає.
    </div>`;
    return;
  }

  list.innerHTML = sorted.map(rec => `
    <div class="ai-rec-row" id="ai-rec-${rec.id}">
      <div style="display:flex;align-items:center;gap:8px;flex:1;min-width:0">
        <span style="font-size:16px;flex-shrink:0">${AI_CAT_ICON[rec.category] || '[SYS]'}</span>
        <div style="min-width:0">
          <div style="font-size:13px;font-weight:600;color:var(--text-1);white-space:nowrap;overflow:hidden;text-overflow:ellipsis">
            <span style="color:${AI_PRIORITY_COLOR[rec.priority]};margin-right:5px;font-size:10px;font-weight:700">${AI_PRIORITY_LABEL ? AI_PRIORITY_LABEL[rec.priority] : ''}</span>${rec.title}
          </div>
          <div style="font-size:11px;color:var(--text-3);margin-top:1px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${rec.reason}</div>
        </div>
      </div>
      <button class="ai-apply-btn btn btn-ripple"
              data-id="${rec.id}"
              style="font-size:11px;padding:3px 10px;flex-shrink:0;margin-left:8px">
        Виправити
      </button>
    </div>
  `).join('');

  list.querySelectorAll('.ai-apply-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const id = btn.dataset.id;
      const origText = btn.textContent;
      btn.disabled = true; btn.textContent = '...';
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const msg = await invoke('apply_recommendation', { id });
        logSuccess(msg);
        btn.textContent = 'OK';
        btn.style.color = 'var(--success)';
        const row = document.getElementById(`ai-rec-${id}`);
        if (row) { row.style.opacity = '0.4'; row.style.pointerEvents = 'none'; }
      } catch (e) {
        logError(`Failed: ${e}`);
        btn.disabled = false; btn.textContent = origText;
      }
    });
  });

  const applyRow = document.getElementById('ai-apply-all-row');
  if (applyRow && sorted.length) {
    applyRow.style.display = 'flex';
    document.getElementById('ai-btn-apply-all')?.addEventListener('click', async () => {
      const safe = _aiRecs.filter(r => r.safe && !r.applied);
      for (const rec of safe) {
        const btn = document.querySelector(`[data-id="${rec.id}"]`);
        if (btn && !btn.disabled) btn.click();
        await new Promise(r => setTimeout(r, 300));
      }
    });
  }
}
