import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

let islcRefreshInterval = null;

export async function renderDeepTweaks(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('deep.title')}</h2>
      <p class="page-subtitle">${t('deep.subtitle')}</p>
    </div>

    <!-- ISLC Section -->
    <div class="section">
      <h3 class="section-title">⚡ ${t('deep.islc_title')}</h3>
      <p class="page-subtitle" style="margin-bottom:16px">${t('deep.islc_subtitle')}</p>

      <div id="standby-info" class="tweak-list">
        <div class="loading-spinner"></div>
      </div>

      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-clear-standby">${t('deep.btn_clear_now')}</button>
        <button class="btn btn-ripple" id="btn-refresh-standby">${t('btn.refresh')}</button>
      </div>

      <div style="margin-top:24px; padding:16px; background:rgba(0,0,0,0.15); border-radius:var(--radius-sm); border:1px solid var(--border)">
        <h4 style="margin:0 0 12px; color:var(--text-main); font-size:14px">
          ${t('deep.auto_monitor')}
        </h4>
        <div style="display:flex; align-items:center; gap:12px; margin-bottom:12px">
          <label style="color:var(--text-dim); font-size:13px; min-width:80px">${t('deep.threshold')}:</label>
          <input type="range" id="islc-threshold" min="256" max="4096" step="128" value="1024"
            style="flex:1; accent-color:var(--accent)">
          <span id="islc-threshold-val" style="color:var(--accent); font-family:var(--font-mono); font-size:13px; min-width:70px">1024 MB</span>
        </div>
        <div id="islc-monitor-status" class="tweak-list" style="margin-bottom:12px">
          <div class="tweak-item">
            <span class="tweak-name">${t('deep.monitor_status')}</span>
            <span class="tweak-status not-applied">${t('deep.stopped')}</span>
          </div>
        </div>
        <div class="btn-group">
          <button class="btn btn-primary btn-ripple" id="btn-start-islc">${t('deep.btn_start')}</button>
          <button class="btn btn-ripple" id="btn-stop-islc">${t('deep.btn_stop')}</button>
        </div>
      </div>
    </div>

    <!-- Core Unparking Section -->
    <div class="section" style="margin-top:32px">
      <h3 class="section-title">🔓 ${t('deep.core_title')}</h3>
      <p class="page-subtitle" style="margin-bottom:16px">${t('deep.core_subtitle')}</p>

      <div id="core-status" class="tweak-list">
        <div class="loading-spinner"></div>
      </div>

      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-unpark">${t('deep.btn_unpark')}</button>
        <button class="btn btn-ripple" id="btn-repark">${t('deep.btn_restore')}</button>
      </div>
    </div>

    <!-- Timer Resolution Section -->
    <div class="section" style="margin-top:32px">
      <h3 class="section-title">⏱ Timer Resolution (0.5ms)</h3>
      <p class="page-subtitle" style="margin-bottom:16px">Windows default: 15.6ms. Gaming optimal: 0.5ms. Gives +5-15 FPS and smoother input in all games.</p>

      <div id="timer-status" class="tweak-list">
        <div class="loading-spinner"></div>
      </div>

      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-boost-timer">Boost to 0.5ms</button>
        <button class="btn btn-ripple" id="btn-reset-timer">Reset Default</button>
        <button class="btn btn-ripple" id="btn-refresh-timer">${t('btn.refresh')}</button>
      </div>
    </div>

    <!-- HPET Section -->
    <div class="section" style="margin-top:32px">
      <h3 class="section-title">🔧 HPET (High Precision Event Timer)</h3>
      <p class="page-subtitle" style="margin-bottom:16px">Disabling HPET forces Windows to use TSC — a faster timer. Gives +5-10 FPS. Safe and reversible. Requires reboot.</p>

      <div id="hpet-status" class="tweak-list">
        <div class="loading-spinner"></div>
      </div>

      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-disable-hpet">Disable HPET</button>
        <button class="btn btn-ripple" id="btn-enable-hpet">Restore HPET</button>
      </div>

      <div style="margin-top:12px; padding:12px; background:rgba(245,158,11,0.1); border:1px solid rgba(245,158,11,0.3); border-radius:var(--radius-sm)">
        <p style="color:var(--warning); font-size:12px">⚠ HPET changes require a system reboot to take effect.</p>
      </div>
    </div>
  `;

  // Stop any previous auto-refresh when re-rendering
  stopAutoRefresh();

  // Clean up interval when navigating away
  const observer = new MutationObserver(() => {
    if (!document.getElementById('standby-info')) {
      stopAutoRefresh();
      observer.disconnect();
    }
  });
  observer.observe(container.parentNode || document.body, { childList: true, subtree: true });

  // ── Load Data ──
  await loadStandbyInfo();
  await loadIslcStatus();
  await loadCoreStatus();

  // ── Threshold Slider ──
  const slider = document.getElementById('islc-threshold');
  const sliderVal = document.getElementById('islc-threshold-val');
  slider?.addEventListener('input', () => {
    sliderVal.textContent = `${slider.value} MB`;
  });

  // ── ISLC Buttons ──
  document.getElementById('btn-clear-standby')?.addEventListener('click', async () => {
    logInfo('Clearing Standby List...');
    try {
      const result = await api.clearStandbyNow();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadStandbyInfo();
      if (result.success) { playEnable(); showToast('Clear Standby RAM'); }
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-refresh-standby')?.addEventListener('click', async () => {
    await loadStandbyInfo();
    await loadIslcStatus();
  });

  document.getElementById('btn-start-islc')?.addEventListener('click', async () => {
    const threshold = parseInt(document.getElementById('islc-threshold')?.value || '1024');
    logInfo(`Starting ISLC Monitor (threshold: ${threshold} MB)...`);
    try {
      const result = await api.startIslcMonitor(threshold);
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadIslcStatus();
      startAutoRefresh();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-stop-islc')?.addEventListener('click', async () => {
    logInfo('Stopping ISLC Monitor...');
    try {
      const result = await api.stopIslcMonitor();
      result.success ? logSuccess(result.message) : logError(result.message);
      stopAutoRefresh();
      setTimeout(() => loadIslcStatus(), 1500);
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  // ── Timer Resolution ──
  await loadTimerStatus();

  document.getElementById('btn-boost-timer')?.addEventListener('click', async () => {
    logInfo('Boosting timer resolution to 0.5ms...');
    try {
      const result = await api.boostTimerResolution();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadTimerStatus();
      if (result.success) { playEnable(); showToast('Boost Timer Resolution'); }
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-reset-timer')?.addEventListener('click', async () => {
    logInfo('Resetting timer resolution...');
    try {
      const result = await api.resetTimerResolution();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadTimerStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-refresh-timer')?.addEventListener('click', loadTimerStatus);

  // ── HPET ──
  await loadHpetStatus();

  document.getElementById('btn-disable-hpet')?.addEventListener('click', async () => {
    logInfo('Disabling HPET...');
    try {
      const result = await api.disableHpet();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadHpetStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-enable-hpet')?.addEventListener('click', async () => {
    logInfo('Restoring HPET...');
    try {
      const result = await api.enableHpet();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadHpetStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  // ── Core Unpark Buttons ──
  document.getElementById('btn-unpark')?.addEventListener('click', async () => {
    logInfo('Unparking all CPU cores...');
    try {
      const results = await api.unparkAllCores();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadCoreStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-repark')?.addEventListener('click', async () => {
    logInfo('Restoring default core parking...');
    try {
      const results = await api.reparkCores();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadCoreStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });
}

// ── Data Loaders ──

async function loadStandbyInfo() {
  try {
    const info = await api.getStandbyInfo();
    const el = document.getElementById('standby-info');
    if (!el) return;

    const usageColor = info.usage_percent > 80 ? '#ff4d6a' : info.usage_percent > 60 ? '#ffb347' : '#4dff91';

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.total_ram')}</span>
        <span class="tweak-status" style="color:var(--text-main)">${info.total_ram_mb} MB</span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.used_ram')}</span>
        <span class="tweak-status" style="color:${usageColor}">${info.used_ram_mb} MB (${info.usage_percent.toFixed(1)}%)</span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">⚠ ${t('deep.standby')}</span>
        <span class="tweak-status" style="color:#ffb347; font-weight:600">${info.standby_mb} MB</span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.free_ram')}</span>
        <span class="tweak-status" style="color:#4dff91">${info.free_ram_mb} MB</span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load standby info: ${e}`);
  }
}

async function loadIslcStatus() {
  try {
    const status = await api.getIslcStatus();
    const el = document.getElementById('islc-monitor-status');
    if (!el) return;

    const running = status.monitor_running;

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.monitor_status')}</span>
        <span class="tweak-status ${running ? 'applied' : 'not-applied'}">
          ${running ? t('deep.running') : t('deep.stopped')}
        </span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.threshold')}</span>
        <span class="tweak-status" style="color:var(--text-main)">${status.threshold_mb} MB</span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.clears')}</span>
        <span class="tweak-status" style="color:var(--accent)">${status.total_clears}</span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load ISLC status: ${e}`);
  }
}

async function loadCoreStatus() {
  try {
    const status = await api.getCoreParkingStatus();
    const el = document.getElementById('core-status');
    if (!el) return;

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.total_cores')}</span>
        <span class="tweak-status" style="color:var(--text-main)">${status.total_cores}</span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">${t('deep.min_active')}</span>
        <span class="tweak-status ${!status.cores_parked ? 'applied' : 'not-applied'}">
          ${status.min_cores_percent}%
        </span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">Status</span>
        <span class="tweak-status ${!status.cores_parked ? 'applied' : 'not-applied'}">
          ${status.cores_parked ? '❌ ' + t('deep.parked') : '✅ ' + t('deep.unparked')}
        </span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load core status: ${e}`);
  }
}

// ── Timer & HPET Status ──

async function loadTimerStatus() {
  try {
    const status = await api.getTimerStatus();
    const el = document.getElementById('timer-status');
    if (!el) return;

    const isBoosted = status.timer_boosted;
    const color = isBoosted ? '#4dff91' : '#ffb347';

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">Current Resolution</span>
        <span class="tweak-status" style="color:${color}; font-weight:600">${status.current_resolution_ms.toFixed(3)} ms</span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">Status</span>
        <span class="tweak-status ${isBoosted ? 'applied' : 'not-applied'}">
          ${isBoosted ? '✅ Boosted (0.5ms)' : '⚠ Default (15.6ms)'}
        </span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load timer status: ${e}`);
  }
}

async function loadHpetStatus() {
  try {
    const status = await api.getTimerStatus();
    const el = document.getElementById('hpet-status');
    if (!el) return;

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">HPET</span>
        <span class="tweak-status ${status.hpet_enabled ? 'not-applied' : 'applied'}">
          ${status.hpet_enabled ? '⚠ Enabled (slower)' : '✅ Disabled (TSC active)'}
        </span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load HPET status: ${e}`);
  }
}

// ── Auto refresh while ISLC monitor runs ──
function startAutoRefresh() {
  stopAutoRefresh();
  islcRefreshInterval = setInterval(async () => {
    await loadStandbyInfo();
    await loadIslcStatus();
  }, 15000);
}

function stopAutoRefresh() {
  if (islcRefreshInterval) {
    clearInterval(islcRefreshInterval);
    islcRefreshInterval = null;
  }
}
