import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

export async function renderRustTweaks(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('rust.title')}</h2>
      <p class="page-subtitle">${t('rust.subtitle')}</p>
    </div>

    <div class="card-grid">
      <div class="card card-enter hex-bg">
        <div class="card-icon">[GAME]</div>
        <div class="card-title">${t('rust.game_path')}</div>
        <div class="card-value" id="rust-path" style="font-size:14px;word-break:break-all">${t('rust.detecting')}</div>
        <div class="card-subtitle">${t('rust.steam_lib')}</div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">${t('rust.launch_opts')}</h3>
      <div style="background:#000;padding:12px;border:1px solid var(--border);border-radius:var(--radius-sm);margin-bottom:12px;font-family:var(--font-mono);font-size:12px;color:var(--accent-primary);white-space:pre-wrap;word-break:break-all" id="rust-launch-opts">${t('txt.loading')}</div>
      <button class="btn btn-ripple" id="btn-copy-launch">${t('rust.btn.copy_launch')}</button>
      <div class="toggle-desc" style="margin-top:8px">${t('rust.launch_desc')}</div>
    </div>

    <div class="section">
      <h3 class="section-title">${t('rust.console_cmds')}</h3>
      <div style="background:#000;padding:12px;border:1px solid var(--border);border-radius:var(--radius-sm);margin-bottom:12px;font-family:var(--font-mono);font-size:12px;color:var(--info);white-space:pre-wrap;word-break:break-all" id="rust-console-cmds">${t('txt.loading')}</div>
      <button class="btn btn-ripple" id="btn-copy-console">${t('rust.btn.copy_console')}</button>
      <div class="toggle-desc" style="margin-top:8px">${t('rust.console_desc')}</div>
    </div>

    <div class="section">
      <h3 class="section-title">${t('rust.engine_tweaks')}</h3>
      <div id="rust-tweaks-list" class="tweak-list">
        <div class="tweak-item">
          <div>
            <span class="tweak-name">${t('rust.tweak_name')}</span>
            <div class="toggle-desc">${t('rust.tweak_desc')}</div>
          </div>
          <span class="tweak-status not-applied">${t('rust.pending')}</span>
        </div>
      </div>
      <div class="btn-group" style="margin-top:16px">
        <button class="btn btn-primary btn-ripple" id="btn-apply-rust">${t('rust.btn.apply_rust')}</button>
      </div>
    </div>
  `;

  try {
    const path = await api.detectRustInstallation();
    document.getElementById('rust-path').textContent = path;
  } catch (e) {
    document.getElementById('rust-path').textContent = t('rust.not_found');
  }

  try {
    const opts = await api.getRecommendedLaunchOptions();
    document.getElementById('rust-launch-opts').textContent = opts;
    
    document.getElementById('btn-copy-launch')?.addEventListener('click', () => {
      navigator.clipboard.writeText(opts);
      logSuccess('Launch options copied to clipboard!');
    });
  } catch (e) {
    document.getElementById('rust-launch-opts').textContent = 'Error loading options';
  }

  try {
    const cmds = await api.getRecommendedConsoleCommands();
    document.getElementById('rust-console-cmds').textContent = cmds.join('\n');
    
    document.getElementById('btn-copy-console')?.addEventListener('click', () => {
      navigator.clipboard.writeText(cmds.join('; '));
      logSuccess('Console commands copied to clipboard (chained)!');
    });
  } catch (e) {
    document.getElementById('rust-console-cmds').textContent = 'Error loading commands';
  }

  document.getElementById('btn-apply-rust')?.addEventListener('click', async () => {
    logInfo('Applying Rust client.cfg tweaks (EAC SAFE)...');
    try {
      const results = await api.applyRustTweaks();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));

      const el = document.querySelector('#rust-tweaks-list .tweak-status');
      if (el && results.every(r => r.success)) {
        el.className = 'tweak-status applied';
        el.textContent = t('rust.applied');
        playEnable(); showToast('Apply Rust Engine Tweaks');
      }
    } catch (e) {
      logError(`Rust engine tweak failed: ${e}`);
    }
  });
}
