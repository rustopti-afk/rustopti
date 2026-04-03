import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';

export async function renderDashboard(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('dash.title')}</h2>
      <p class="page-subtitle">${t('dash.subtitle')}</p>
    </div>

    <div id="web-hero-banner" style="display:none; background: linear-gradient(135deg, rgba(77, 255, 145, 0.1) 0%, rgba(0, 0, 0, 0) 100%); border: 1px solid var(--accent-primary); border-radius: var(--radius-md); padding: 24px; margin-bottom: 30px; position: relative; overflow: hidden;">
        <div style="position:relative; z-index:2;">
            <h3 style="color:var(--accent-primary); margin-bottom:8px; font-size:20px;">🚀 Ready to Boost Your FPS?</h3>
            <p style="color:var(--text-main); margin-bottom:16px; max-width:600px;">
                You are currently viewing the <b>Web Demo</b>. Download the full desktop application to unlock real-time RAM clearing, CPU unparking, and advanced registry optimizations.
            </p>
            <a href="https://rustopti.fun/RustOpti_2.2.0_x64-setup.exe" class="btn btn-primary btn-ripple" style="padding:12px 24px; font-weight:600;">
                Download RustOpti Installer (.exe)
            </a>
        </div>
        <div style="position:absolute; right:-20px; top:-20px; font-size:120px; opacity:0.05; font-weight:900; pointer-events:none;">RUST</div>
    </div>

    <div class="card-grid" id="sys-cards">
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

    <div class="section">
      <h3 class="section-title">${t('dash.quick_optimize')}</h3>
      <p style="color:var(--text-muted);font-size:12px;margin-bottom:12px">${t('dash.quick_desc')}</p>
      <div class="btn-group">
        <button class="btn btn-primary btn-ripple" id="btn-quick-optimize">${t('dash.btn.optimize_all')}</button>
        <button class="btn btn-success btn-ripple" id="btn-backup">${t('dash.btn.backup')}</button>
        <button class="btn btn-ripple" id="btn-kill-bloat">${t('dash.btn.kill_bloat')}</button>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">${t('dash.disks')}</h3>
      <div id="dash-disks" class="card-grid"></div>
    </div>
  `;

  // Web Mode Hero Visibility
  if (!window.__TAURI_INTERNALS__) {
      const hero = document.getElementById('web-hero-banner');
      if (hero) hero.style.display = 'block';
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
    const section = btn.closest('.section');
    const originalText = btn.textContent;
    btn.disabled = true;
    btn.style.pointerEvents = 'none';

    const steps = [
      { name: 'Backup', fn: () => api.backupAllBeforeOptimization(), label: 'Creating backup...' },
      { name: 'Registry', fn: () => api.applyRegistryTweaks(), label: 'Applying registry tweaks...' },
      { name: 'GPU', fn: () => api.applyGpuTweaks(), label: 'Applying GPU tweaks...' },
      { name: 'Power', fn: () => api.applyPowerTweaks(), label: 'Applying power tweaks...' },
      { name: 'Network', fn: () => api.applyNetworkTweaks(), label: 'Applying network tweaks...' },
      { name: 'CPU Cores', fn: () => api.unparkAllCores(), label: 'Unparking CPU cores...' },
      { name: 'Timer 0.5ms', fn: () => api.boostTimerResolution(), label: 'Boosting timer resolution...' },
      { name: 'HPET Off', fn: () => api.disableHpet(), label: 'Disabling HPET...' },
    ];

    // Insert progress bar above button
    let progressEl = document.getElementById('optimize-progress');
    if (!progressEl) {
      const progressHtml = `
        <div id="optimize-progress" style="margin-bottom:16px">
          <div style="display:flex;justify-content:space-between;margin-bottom:6px">
            <span id="opt-step-label" style="font-size:12px;color:var(--text-muted)">Starting...</span>
            <span id="opt-step-count" style="font-size:12px;color:var(--accent-primary);font-family:var(--font-mono)">0/${steps.length}</span>
          </div>
          <div class="stat-bar" style="height:8px;border-radius:4px">
            <div id="opt-progress-bar" class="stat-bar-fill" style="width:0%;transition:width 0.4s ease"></div>
          </div>
        </div>
      `;
      btn.closest('.btn-group').insertAdjacentHTML('beforebegin', progressHtml);
    }

    const stepLabel = document.getElementById('opt-step-label');
    const stepCount = document.getElementById('opt-step-count');
    const progressBar = document.getElementById('opt-progress-bar');

    logInfo('Starting full optimization...');
    btn.textContent = '⏳ Working...';

    // Force browser to repaint before starting heavy work
    await new Promise(r => setTimeout(r, 50));

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      const pct = Math.round(((i) / steps.length) * 100);

      // Update progress UI
      stepLabel.textContent = step.label;
      stepCount.textContent = `${i + 1}/${steps.length}`;
      progressBar.style.width = `${pct}%`;

      logInfo(`[${i + 1}/${steps.length}] ${step.label}`);

      // Give UI time to repaint before each step
      await new Promise(r => setTimeout(r, 16));

      try {
        const results = await step.fn();
        if (Array.isArray(results)) {
          results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
        } else if (results && results.message) {
          results.success ? logSuccess(results.message) : logError(results.message);
        }
      } catch (e) {
        logError(`[${step.name}] Failed: ${e}`);
      }
    }

    // Complete
    progressBar.style.width = '100%';
    stepLabel.textContent = 'Complete!';
    stepCount.textContent = `${steps.length}/${steps.length}`;

    logSuccess('✓ Full optimization complete!');

    // Fade out progress after 3s
    setTimeout(() => {
      const el = document.getElementById('optimize-progress');
      if (el) el.style.display = 'none';
    }, 3000);

    btn.textContent = originalText;
    btn.disabled = false;
    btn.style.pointerEvents = 'auto';
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
