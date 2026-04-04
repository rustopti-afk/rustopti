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
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Timer Resolution — 0.5ms</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>RAM — очистка пам'яті</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Мережа — Nagle, TCP No Delay</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>CPU — розпарковка ядер</span>
          </div>
          <div class="tweak-item">
            <span class="tweak-dot instant"></span>
            <span>Реєстр — Game DVR, GPU Priority</span>
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
          </div>
          <div class="tweak-item">
            <span class="tweak-dot reboot"></span>
            <span>HPET — вимкнення (+5-10 FPS на AMD)</span>
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
        <span class="upscaling-icon">⬆</span>
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
            <h3 style="color:var(--accent-primary); margin-bottom:8px; font-size:18px;">🚀 Завантаж десктоп додаток</h3>
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
            note.textContent = '✓ ' + msg;
            note.style.color = 'var(--accent-primary)';
          } catch(e) {
            note.textContent = '✗ ' + e;
            note.style.color = '#f87171';
          }
        });

        sharpSlider.addEventListener('input', () => {
          sharpVal.textContent = sharpSlider.value;
        });
        sharpSlider.addEventListener('change', async () => {
          try {
            const msg = await api.setUpscaling(toggle.checked, parseInt(sharpSlider.value));
            note.textContent = '✓ ' + msg;
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
    stepLabel.textContent = '✓ Готово!';
    stepCount.textContent = `${steps.length}/${steps.length}`;
    statusText.textContent = '✓ Систему оптимізовано';
    statusText.style.color = 'var(--accent-primary)';
    rebootNote.style.display = 'block';
    logSuccess('✓ Оптимізацію завершено! Перезавантаж ПК для повного ефекту.');

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
}
