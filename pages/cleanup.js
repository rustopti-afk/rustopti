import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

export async function renderCleanup(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('clean.title')}</h2>
      <p class="page-subtitle">${t('clean.subtitle')}</p>
    </div>

    <div class="card-grid" id="cleanup-preview-cards">
      <div class="card card-enter hex-bg">
        <div class="card-icon">[CACHE]</div>
        <div class="card-title">${t('clean.temp')}</div>
        <div class="card-value">${t('clean.scanning')}</div>
      </div>
      <div class="card card-enter hex-bg" style="animation-delay:0.1s">
        <div class="card-icon">[WEB]</div>
        <div class="card-title">${t('clean.cache')}</div>
        <div class="card-value">${t('clean.scanning')}</div>
      </div>
      <div class="card card-enter hex-bg" style="animation-delay:0.2s">
        <div class="card-icon">[LOGS]</div>
        <div class="card-title">${t('clean.logs')}</div>
        <div class="card-value">${t('clean.scanning')}</div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">${t('clean.disk_clean')}</h3>
      <p style="color:var(--text-muted);font-size:13px;margin-bottom:16px;">
        ${t('clean.desc')}
      </p>
      
      <div class="btn-group">
        <button class="btn btn-primary btn-ripple" id="btn-run-cleanup">${t('clean.btn.deep_clean')}</button>
        <button class="btn btn-ripple" id="btn-rescan">${t('clean.btn.rescan')}</button>
      </div>
    </div>
  `;

  await scanCleanup();

  document.getElementById('btn-rescan')?.addEventListener('click', scanCleanup);
  document.getElementById('btn-run-cleanup')?.addEventListener('click', async () => {
    logInfo('Starting deep disk cleanup...');
    try {
      const msgs = await api.runDiskCleanup();
      msgs.forEach(m => logSuccess(m.message));
      logSuccess('Cleanup complete! Triggering rescan...');
      await scanCleanup();
      playEnable();
      showToast('Cleanup Cache');
    } catch (e) {
      logError(`Cleanup failed: ${e}`);
    }
  });
}

async function scanCleanup() {
  try {
    const preview = await api.getCleanupPreview();
    const cards = document.getElementById('cleanup-preview-cards');
    cards.innerHTML = preview.map((p, i) => `
      <div class="card card-enter hex-bg" style="animation-delay:${i * 0.1}s">
        <div class="card-icon">${p.category.includes('Temp') ? '[TEMP]' : '[SYS]'}</div>
        <div class="card-title">${p.category}</div>
        <div class="card-value">${(p.size_mb || 0).toFixed(1)} MB</div>
      </div>
    `).join('');
    
    const total = preview.reduce((sum, p) => sum + (p.size_mb || 0), 0);
    logInfo(`Scan complete: ${total.toFixed(1)} MB of junk found.`);
  } catch (e) {
    logError(`Cleanup scan failed: ${e}`);
  }
}
