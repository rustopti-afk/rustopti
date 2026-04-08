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
      <h3 class="section-title">Visual Profile — 1 клік</h3>
      <div class="card" style="background:rgba(42,125,225,0.06);border:1px solid rgba(42,125,225,0.2);padding:16px;margin-bottom:12px">
        <div style="font-size:13px;color:var(--text-2);margin-bottom:12px">
          Встановлює оптимальні налаштування дисплея та Rust cfg:
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;margin-bottom:14px">
          <div style="text-align:center;padding:8px;background:rgba(0,0,0,0.2);border-radius:8px">
            <div style="font-size:18px;font-weight:700;color:var(--accent-bright)">65%</div>
            <div style="font-size:11px;color:var(--text-3)">Яскравість</div>
          </div>
          <div style="text-align:center;padding:8px;background:rgba(0,0,0,0.2);border-radius:8px">
            <div style="font-size:18px;font-weight:700;color:var(--accent-bright)">70%</div>
            <div style="font-size:11px;color:var(--text-3)">Контраст</div>
          </div>
          <div style="text-align:center;padding:8px;background:rgba(0,0,0,0.2);border-radius:8px">
            <div style="font-size:18px;font-weight:700;color:var(--accent-bright)">1.40</div>
            <div style="font-size:11px;color:var(--text-3)">Гамма</div>
          </div>
        </div>
        <button class="btn btn-primary btn-ripple" id="btn-visual-profile" style="width:100%;justify-content:center">
          Застосувати Visual Profile
        </button>
        <div id="visual-profile-results" style="margin-top:10px"></div>
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

  document.getElementById('btn-visual-profile')?.addEventListener('click', async () => {
    const btn = document.getElementById('btn-visual-profile');
    const resultsEl = document.getElementById('visual-profile-results');
    btn.disabled = true;
    btn.textContent = 'Застосовується...';
    playEnable();
    try {
      const results = await api.applyVisualProfile();
      resultsEl.innerHTML = results.map(r => `
        <div style="display:flex;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid rgba(255,255,255,0.04);font-size:12px">
          <span style="color:${r.success ? 'var(--success)' : 'var(--danger)'}">${r.success ? '+' : '-'}</span>
          <span style="color:var(--text-2)">${r.message}</span>
        </div>
      `).join('');
      showToast('Visual Profile застосовано!', 'success');
      logSuccess('Visual Profile applied');
    } catch (e) {
      resultsEl.innerHTML = `<div style="color:var(--danger);font-size:12px">Помилка: ${e}</div>`;
      logError(`Visual Profile error: ${e}`);
    }
    btn.disabled = false;
    btn.textContent = 'Застосувати Visual Profile';
  });

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
