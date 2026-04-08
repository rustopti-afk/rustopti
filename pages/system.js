import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

export async function renderSystem(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('sys.title')}</h2>
      <p class="page-subtitle">${t('sys.subtitle')}</p>
    </div>

    <div class="section">
      <h3 class="section-title">${t('sys.status')}</h3>
      <div id="reg-status" class="tweak-list"><div class="loading-spinner"></div></div>
    </div>

    <div class="btn-group">
      <button class="btn btn-primary btn-ripple" id="btn-apply-reg">${t('sys.btn.apply_reg')}</button>
      <button class="btn btn-ripple" id="btn-refresh-reg">${t('sys.btn.refresh_reg')}</button>
    </div>

    <div class="section" style="margin-top:24px">
      <h3 class="section-title">${t('sys.power_plan')}</h3>
      <div id="power-plans" class="tweak-list"><div class="loading-spinner"></div></div>
      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-apply-power">${t('sys.btn.apply_power')}</button>
      </div>
    </div>

    <!-- MSI Mode -->
    <div class="section" style="margin-top:24px">
      <h3 class="section-title">MSI Mode (GPU Interrupts)</h3>
      <p style="color:var(--text-muted);font-size:12px;margin-bottom:12px">Switches GPU to Message Signaled Interrupts — reduces input lag by 2-5ms. Requires reboot.</p>
      <div id="msi-status" class="tweak-list"><div class="loading-spinner"></div></div>
      <div class="btn-group" style="margin-top:12px">
        <button class="btn btn-primary btn-ripple" id="btn-enable-msi">Enable MSI Mode</button>
      </div>
    </div>

    <!-- SysMain -->
    <div class="section" style="margin-top:24px">
      <h3 class="section-title">SysMain (Superfetch)</h3>
      <p style="color:var(--text-muted);font-size:12px;margin-bottom:12px">Windows caches apps in background causing disk I/O stutters. Disable for smoother gaming.</p>
      <div id="sysmain-status" class="tweak-list"><div class="loading-spinner"></div></div>
      <div class="btn-group" style="margin-top:12px">
        <button class="btn btn-primary btn-ripple" id="btn-disable-sysmain">Disable SysMain</button>
        <button class="btn btn-ripple" id="btn-enable-sysmain">Restore</button>
      </div>
    </div>

    <!-- Visual Effects -->
    <div class="section" style="margin-top:24px">
      <h3 class="section-title">Visual Effects</h3>
      <p style="color:var(--text-muted);font-size:12px;margin-bottom:12px">Disable Windows animations, transparency, and shadows. Frees GPU/CPU resources for gaming.</p>
      <div id="visual-status" class="tweak-list"><div class="loading-spinner"></div></div>
      <div class="btn-group" style="margin-top:12px">
        <button class="btn btn-primary btn-ripple" id="btn-disable-visual">Best Performance</button>
        <button class="btn btn-ripple" id="btn-restore-visual">Restore Default</button>
      </div>
    </div>
  `;

  await Promise.all([
    loadRegistryStatus(),
    loadPowerPlans(),
    loadMsiStatus(),
    loadSysmainStatus(),
    loadVisualStatus(),
  ]);

  // Registry tweaks
  document.getElementById('btn-apply-reg')?.addEventListener('click', async () => {
    logInfo('Applying registry tweaks...');
    try {
      const results = await api.applyRegistryTweaks();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadRegistryStatus();
      playEnable(); showToast('Apply Registry Tweaks');
    } catch (e) { logError(`Failed: ${e}`); }
  });

  document.getElementById('btn-refresh-reg')?.addEventListener('click', loadRegistryStatus);

  // Power tweaks
  document.getElementById('btn-apply-power')?.addEventListener('click', async () => {
    logInfo('Applying power tweaks...');
    try {
      const results = await api.applyPowerTweaks();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadPowerPlans();
    } catch (e) { logError(`Failed: ${e}`); }
  });

  // MSI Mode
  document.getElementById('btn-enable-msi')?.addEventListener('click', async () => {
    logInfo('Enabling MSI Mode for GPU...');
    try {
      const results = await api.enableMsiMode();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadMsiStatus();
    } catch (e) { logError(`Failed: ${e}`); }
  });

  // SysMain
  document.getElementById('btn-disable-sysmain')?.addEventListener('click', async () => {
    logInfo('Disabling SysMain...');
    try {
      const result = await api.disableSysmain();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadSysmainStatus();
    } catch (e) { logError(`Failed: ${e}`); }
  });

  document.getElementById('btn-enable-sysmain')?.addEventListener('click', async () => {
    logInfo('Restoring SysMain...');
    try {
      const result = await api.enableSysmain();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadSysmainStatus();
    } catch (e) { logError(`Failed: ${e}`); }
  });

  // Visual Effects
  document.getElementById('btn-disable-visual')?.addEventListener('click', async () => {
    logInfo('Disabling visual effects...');
    try {
      const results = await api.disableVisualEffects();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadVisualStatus();
    } catch (e) { logError(`Failed: ${e}`); }
  });

  document.getElementById('btn-restore-visual')?.addEventListener('click', async () => {
    logInfo('Restoring visual effects...');
    try {
      const result = await api.restoreVisualEffects();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadVisualStatus();
    } catch (e) { logError(`Failed: ${e}`); }
  });
}

async function loadRegistryStatus() {
  try {
    const statuses = await api.getRegistryStatus();
    document.getElementById('reg-status').innerHTML = statuses.map(s => `
      <div class="tweak-item">
        <span class="tweak-name">${s.name}</span>
        <span class="tweak-status ${s.applied ? 'applied' : 'not-applied'}">
          ${s.applied ? t('sys.applied') : t('sys.not_applied')}
        </span>
      </div>
    `).join('');
  } catch (e) { logError(`Failed to load status: ${e}`); }
}

async function loadPowerPlans() {
  try {
    const plans = await api.getPowerPlans();
    document.getElementById('power-plans').innerHTML = plans.map(p => `
      <div class="tweak-item">
        <span class="tweak-name">${p.name}</span>
        <span class="tweak-status ${p.active ? 'applied' : 'not-applied'}">
          ${p.active ? t('sys.active') : t('sys.inactive')}
        </span>
      </div>
    `).join('');
  } catch (e) { logError(`Failed to load power plans: ${e}`); }
}

async function loadMsiStatus() {
  try {
    const items = await api.getMsiModeStatus();
    document.getElementById('msi-status').innerHTML = items.map(s => `
      <div class="tweak-item">
        <span class="tweak-name">${s.name}</span>
        <span class="tweak-status ${s.optimized ? 'applied' : 'not-applied'}">
          ${s.current_value}
        </span>
      </div>
    `).join('');
  } catch (e) { logError(`Failed to load MSI status: ${e}`); }
}

async function loadSysmainStatus() {
  try {
    const status = await api.getSysmainStatus();
    document.getElementById('sysmain-status').innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">${status.name}</span>
        <span class="tweak-status ${status.optimized ? 'applied' : 'not-applied'}">
          ${status.optimized ? 'Disabled' : status.current_value}
        </span>
      </div>
    `;
  } catch (e) { logError(`Failed to load SysMain status: ${e}`); }
}

async function loadVisualStatus() {
  try {
    const status = await api.getVisualEffectsStatus();
    document.getElementById('visual-status').innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">${status.name}</span>
        <span class="tweak-status ${status.optimized ? 'applied' : 'not-applied'}">
          ${status.optimized ? 'Best Performance' : status.current_value}
        </span>
      </div>
    `;
  } catch (e) { logError(`Failed to load visual effects status: ${e}`); }
}
