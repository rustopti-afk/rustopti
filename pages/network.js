import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

export async function renderNetwork(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('net.title')}</h2>
      <p class="page-subtitle">${t('net.subtitle')}</p>
    </div>

    <div class="section">
      <h3 class="section-title">${t('net.status')}</h3>
      <div id="net-status" class="tweak-list"><div class="loading-spinner"></div></div>
    </div>

    <div class="btn-group">
      <button class="btn btn-primary btn-ripple" id="btn-apply-net">${t('net.btn.optimize')}</button>
      <button class="btn btn-ripple" id="btn-refresh-net">${t('btn.refresh')}</button>
    </div>
  `;

  await loadNetStatus();

  document.getElementById('btn-apply-net')?.addEventListener('click', async () => {
    logInfo('Applying network optimizations...');
    try {
      const results = await api.applyNetworkTweaks();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadNetStatus();
      playEnable(); showToast('Apply Network Tweaks');
    } catch (e) {
      logError(`Network optimization failed: ${e}`);
    }
  });

  document.getElementById('btn-refresh-net')?.addEventListener('click', loadNetStatus);
}

async function loadNetStatus() {
  try {
    const items = await api.getNetworkStatus();
    document.getElementById('net-status').innerHTML = items.map(i => `
      <div class="tweak-item">
        <div>
          <span class="tweak-name">${i.name}</span>
          <div class="toggle-desc">${i.description}</div>
        </div>
        <span class="tweak-status ${i.optimized ? 'applied' : 'not-applied'}">
          ${i.optimized ? t('net.optimized') : t('net.default')}
        </span>
      </div>
    `).join('');
  } catch (e) {
    logError(`Failed to load network status: ${e}`);
  }
}
