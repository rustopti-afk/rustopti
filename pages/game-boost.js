import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { playEnable } from '../js/sounds.js';
import { showToast } from '../js/toast.js';

export async function renderGameBoost(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">🚀 Game Boost</h2>
      <p class="page-subtitle">Maximum FPS — one-click game mode, Defender exclusion, Large Pages & CPU affinity</p>
    </div>

    <!-- Active Protection -->
    <div class="section" style="background: linear-gradient(135deg, rgba(77,255,145,0.08), rgba(0,0,0,0)); border-color: var(--success); margin-bottom:24px">
      <h3 class="section-title">🟢 Active Protection</h3>
      <p style="color:var(--text-muted);font-size:13px;margin-bottom:12px">
        Фоновий захист кожні <b>30 секунд</b>: прибирає неактивну RAM у фонових процесів та підтримує Rust на High priority.
        Увімкни один раз — комп відчувається super clean весь сеанс.
      </p>
      <div id="protection-status" style="margin-bottom:14px;font-size:13px;color:var(--text-disabled)">Перевіряємо...</div>
      <div class="btn-group">
        <button class="btn btn-success btn-ripple" id="btn-start-protection" style="padding:12px 28px;font-size:14px;font-weight:700">🟢 Увімкнути</button>
        <button class="btn btn-ripple" id="btn-stop-protection">Вимкнути</button>
      </div>
    </div>

    <!-- Game Mode (one-click) -->
    <div class="section" style="background: linear-gradient(135deg, rgba(139,92,246,0.1), rgba(0,0,0,0)); border-color: var(--accent-primary)">
      <h3 class="section-title">🎮 Game Mode — One Click Boost</h3>
      <p style="color:var(--text-muted);font-size:13px;margin-bottom:16px">
        Start Rust first, then activate. Kills bloatware, sets High priority, pins to fast CPU cores, clears RAM.
      </p>
      <div id="gamemode-status" class="tweak-list" style="margin-bottom:16px">
        <div class="loading-spinner"></div>
      </div>
      <div class="btn-group">
        <button class="btn btn-primary btn-ripple" id="btn-activate-gm" style="padding:12px 24px;font-size:14px">⚡ Activate Game Mode</button>
        <button class="btn btn-ripple" id="btn-deactivate-gm">Deactivate</button>
      </div>
    </div>

    <!-- Defender Exclusion -->
    <div class="section" style="margin-top:24px">
      <h3 class="section-title">🛡 Windows Defender Exclusion</h3>
      <p style="color:var(--text-muted);font-size:13px;margin-bottom:4px">
        Defender scans every file Rust loads. Excluding the game folder gives <span style="color:var(--success);font-weight:600">+10-20 FPS</span> — the single biggest optimization.
      </p>
      <p style="color:var(--warning);font-size:11px;margin-bottom:16px">
        ⚠ Only excludes the Rust game folder. Defender stays active for everything else.
      </p>
      <div id="defender-status" class="tweak-list" style="margin-bottom:12px">
        <div class="loading-spinner"></div>
      </div>
      <div class="btn-group">
        <button class="btn btn-primary btn-ripple" id="btn-add-defender">Add Exclusion</button>
        <button class="btn btn-ripple" id="btn-remove-defender">Remove</button>
      </div>
    </div>

    <!-- Large Pages -->
    <div class="section" style="margin-top:24px">
      <h3 class="section-title">📄 Large Pages</h3>
      <p style="color:var(--text-muted);font-size:13px;margin-bottom:16px">
        Enables "Lock Pages in Memory" for your user account. Reduces CPU TLB misses, gives <span style="color:var(--success);font-weight:600">+5-15% FPS</span> in Unity games like Rust. Requires reboot.
      </p>
      <div id="largepages-status" class="tweak-list" style="margin-bottom:12px">
        <div class="loading-spinner"></div>
      </div>
      <div class="btn-group">
        <button class="btn btn-primary btn-ripple" id="btn-enable-lp">Enable Large Pages</button>
      </div>
    </div>
  `;

  // Load all statuses in parallel
  await Promise.all([
    loadActiveProtectionStatus(),
    loadGameModeStatus(),
    loadDefenderStatus(),
    loadLargePagesStatus(),
  ]);

  // Active Protection
  document.getElementById('btn-start-protection')?.addEventListener('click', async () => {
    const btn = document.getElementById('btn-start-protection');
    btn.disabled = true;
    btn.textContent = '⏳ Запускаємо...';
    logInfo('Starting Active Protection...');
    try {
      const result = await api.startActiveProtection();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadActiveProtectionStatus();
      if (result.success) { playEnable(); showToast('Active Protection ON'); }
    } catch (e) {
      logError(`Failed: ${e}`);
    }
    btn.disabled = false;
    btn.textContent = '🟢 Увімкнути';
  });

  document.getElementById('btn-stop-protection')?.addEventListener('click', async () => {
    logInfo('Stopping Active Protection...');
    try {
      const result = await api.stopActiveProtection();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadActiveProtectionStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  // Game Mode
  document.getElementById('btn-activate-gm')?.addEventListener('click', async () => {
    const btn = document.getElementById('btn-activate-gm');
    btn.textContent = '⏳ Activating...';
    btn.disabled = true;
    logInfo('Activating Game Mode...');
    try {
      await new Promise(r => requestAnimationFrame(() => setTimeout(r, 100)));
      const results = await api.activateGameMode();
      results.forEach(r => r.success ? logSuccess(r.message) : logError(r.message));
      await loadGameModeStatus();
      playEnable(); showToast('Activate Game Mode');
    } catch (e) {
      logError(`Failed: ${e}`);
    }
    btn.textContent = '⚡ Activate Game Mode';
    btn.disabled = false;
  });

  document.getElementById('btn-deactivate-gm')?.addEventListener('click', async () => {
    logInfo('Deactivating Game Mode...');
    try {
      const result = await api.deactivateGameMode();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadGameModeStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  // Defender
  document.getElementById('btn-add-defender')?.addEventListener('click', async () => {
    logInfo('Adding Rust folder to Defender exclusions...');
    try {
      const result = await api.addDefenderExclusion();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadDefenderStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  document.getElementById('btn-remove-defender')?.addEventListener('click', async () => {
    logInfo('Removing Defender exclusion...');
    try {
      const result = await api.removeDefenderExclusion();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadDefenderStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  // Large Pages
  document.getElementById('btn-enable-lp')?.addEventListener('click', async () => {
    logInfo('Enabling Large Pages...');
    try {
      const result = await api.enableLargePages();
      result.success ? logSuccess(result.message) : logError(result.message);
      await loadLargePagesStatus();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });
}

async function loadActiveProtectionStatus() {
  const el = document.getElementById('protection-status');
  if (!el) return;
  try {
    const active = await api.getActiveProtectionStatus();
    el.innerHTML = active
      ? `<span style="color:var(--success);font-weight:600">🟢 Активний — RAM trim & Rust priority кожні 30с</span>`
      : `<span style="color:var(--text-disabled)">⚫ Вимкнено</span>`;
  } catch {
    el.innerHTML = `<span style="color:var(--text-disabled)">⚫ Вимкнено</span>`;
  }
}

async function loadGameModeStatus() {
  try {
    const status = await api.getGameBoostStatus();
    const el = document.getElementById('gamemode-status');
    if (!el) return;

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">Game Mode</span>
        <span class="tweak-status ${status.game_mode_active ? 'applied' : 'not-applied'}">
          ${status.game_mode_active ? '✅ Active' : '⚠ Inactive'}
        </span>
      </div>
      <div class="tweak-item">
        <span class="tweak-name">Rust Process</span>
        <span class="tweak-status" style="color:${status.rust_running ? 'var(--success)' : 'var(--text-disabled)'}">
          ${status.rust_running ? `Running (PID: ${status.rust_pid})` : 'Not running — start Rust first'}
        </span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load status: ${e}`);
  }
}

async function loadDefenderStatus() {
  try {
    const status = await api.getDefenderStatus();
    const el = document.getElementById('defender-status');
    if (!el) return;

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">Rust Folder</span>
        <span class="tweak-status ${status.success ? 'applied' : 'not-applied'}">
          ${status.message}
        </span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load Defender status: ${e}`);
  }
}

async function loadLargePagesStatus() {
  try {
    const status = await api.getLargePagesStatus();
    const el = document.getElementById('largepages-status');
    if (!el) return;

    el.innerHTML = `
      <div class="tweak-item">
        <span class="tweak-name">Lock Pages in Memory</span>
        <span class="tweak-status ${status.success ? 'applied' : 'not-applied'}">
          ${status.message}
        </span>
      </div>
    `;
  } catch (e) {
    logError(`Failed to load Large Pages status: ${e}`);
  }
}
