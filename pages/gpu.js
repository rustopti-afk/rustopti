import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

export async function renderGpu(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('gpu.title')}</h2>
      <p class="page-subtitle">${t('gpu.subtitle')}</p>
    </div>

    <div class="card-grid">
      <div class="card card-enter hex-bg">
        <div class="card-icon">[GPU]</div>
        <div class="card-title">${t('gpu.vendor')}</div>
        <div class="card-value" id="gpu-vendor">${t('gpu.detecting')}</div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">${t('gpu.tweaks')}</h3>
      <div id="gpu-tweaks" class="tweak-list">
        <div style="color:var(--text-muted);font-size:12px">${t('gpu.desc')}</div>
      </div>
      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-gpu-optimize">${t('gpu.btn.optimize')}</button>
      </div>
    </div>
  `;

  try {
    const vendor = await api.detectGpuVendor();
    document.getElementById('gpu-vendor').textContent = vendor;
    logInfo(`GPU detected: ${vendor}`);
  } catch (e) {
    document.getElementById('gpu-vendor').textContent = t('txt.unknown');
  }

  document.getElementById('btn-gpu-optimize')?.addEventListener('click', async () => {
    logInfo('Applying GPU optimizations...');
    try {
      const results = await api.applyGpuTweaks();
      const el = document.getElementById('gpu-tweaks');
      el.innerHTML = results.map(r => `
        <div class="tweak-item">
          <span class="tweak-name">${r.name}</span>
          <span class="tweak-status ${r.success ? 'applied' : 'not-applied'}">
            ${r.success ? t('gpu.success') : t('gpu.failed')}
          </span>
        </div>
      `).join('');
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      playEnable(); showToast('Apply GPU Tweaks');
    } catch (e) {
      logError(`GPU optimization failed: ${e}`);
    }
  });
}
