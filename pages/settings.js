import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';
import { setLocale, getCurrentLocale } from '../js/i18n.js';
import { getKeybinds, setKeybind, resetKeybinds, exportConfig, importConfig } from '../js/keybinds.js';
import { areToastsEnabled, setToastsEnabled } from '../js/toast.js';

export async function renderSettings(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('set.title')}</h2>
      <p class="page-subtitle">${t('set.subtitle')}</p>
    </div>

    <div class="section">
      <h3 class="section-title">${t('set.restore')}</h3>
      
      <div style="display:flex;gap:8px;margin-bottom:16px">
        <input type="text" id="backup-desc" placeholder="${t('set.backup_desc')}"
          style="flex:1;padding:10px 12px;background:rgba(0,0,0,0.2);border:1px solid var(--border);border-radius:var(--radius-sm);color:var(--text-main);font-family:var(--font-mono);font-size:13px;outline:none;">
        <button class="btn btn-primary btn-ripple" id="btn-create-backup">${t('set.btn.create_backup')}</button>
      </div>

      <div id="backup-list" class="tweak-list">
        <div class="loading-spinner"></div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">Мова інтерфейсу</h3>
      <div class="tweak-list">
        <div class="keybind-row">
          <span class="keybind-label">Мова / Language</span>
          <select id="settings-lang-select" style="padding:6px 12px;background:rgba(0,0,0,0.3);border:1px solid var(--border);border-radius:var(--radius-sm);color:var(--text-main);font-size:13px;outline:none;cursor:pointer;">
            <option value="en">English</option>
            <option value="uk">Українська</option>
            <option value="ru">Русский</option>
          </select>
        </div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">Повідомлення</h3>
      <div class="tweak-list">
        <div class="keybind-row">
          <div>
            <span class="keybind-label">Toast-повідомлення при активації функцій</span>
            <div class="toggle-desc" style="margin-top:2px">Сповіщення знизу екрану коли функція активована</div>
          </div>
          <label class="toggle-switch">
            <input type="checkbox" id="toggle-toasts">
            <span class="toggle-slider"></span>
          </label>
        </div>
        <div class="keybind-row">
          <div>
            <span class="keybind-label">Попередження про перегрів</span>
            <div class="toggle-desc" style="margin-top:2px">Завжди активне — ігнорує вимкнення повідомлень</div>
          </div>
          <span class="tweak-status applied">Активне</span>
        </div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">Keybinds</h3>
      <p style="color:var(--text-muted);font-size:13px;margin-bottom:16px;">Натисни на клавішу щоб змінити. Працює коли фокус не в полі вводу.</p>
      <div id="keybinds-list"></div>
      <div class="btn-group" style="margin-top:12px">
        <button class="btn btn-ripple" id="btn-reset-binds" style="background:rgba(255,255,255,0.04)">Скинути до дефолту</button>
        <button class="btn btn-ripple" id="btn-export-cfg" style="background:rgba(157,94,245,0.08);color:var(--accent-bright)">Export .cfg</button>
        <label class="btn btn-ripple" style="background:rgba(157,94,245,0.08);color:var(--accent-bright);cursor:pointer">
          Import .cfg<input type="file" id="cfg-file-input" accept=".cfg,.json" style="display:none">
        </label>
      </div>
    </div>

      <div class="section">
        <h3 class="section-title">Developer & Testing</h3>
        <p class="toggle-desc">Use this to test the full activation flow ("Option 1").</p>
        <button class="btn btn-ripple" id="btn-reset-license" style="margin-top:10px; background:rgba(255,77,106,0.1); color:#ff4d6a; border:1px solid rgba(255,77,106,0.3);">
          Reset License & Logout
        </button>
      </div>

      <div class="section">
        <h3 class="section-title">${t('set.about')}</h3>
      <div class="tweak-list">
        <div class="tweak-item">
          <div>
            <span class="tweak-name">${t('set.zero_injection')}</span>
            <div class="toggle-desc">${t('set.zero_desc')}</div>
          </div>
          <span class="tweak-status applied">${t('sys.active')}</span>
        </div>
        <div class="tweak-item">
          <div>
            <span class="tweak-name">${t('set.auto_backup')}</span>
            <div class="toggle-desc">${t('set.auto_desc')}</div>
          </div>
          <span class="tweak-status applied">${t('sys.active')}</span>
        </div>
      </div>
    </div>

    <div class="section" style="text-align:center;padding:40px 20px">
      <div style="font-size:40px;margin-bottom:16px;color:var(--accent-primary);filter:drop-shadow(0 0 10px var(--accent-primary))">[RUSTOPTI]</div>
      <h2 style="margin-bottom:8px">RustOpti v2.2</h2>
      <p style="color:var(--text-muted);font-size:14px;max-width:400px;margin:0 auto;">
        ${t('set.app_desc')}
      </p>
      <div style="margin-top:24px;font-family:var(--font-mono);font-size:12px;color:var(--text-disabled)">
        MIT License • Safe for EAC/BattlEye
      </div>
    </div>
  `;

  await loadBackups();
  renderKeybindsList();

  // Language selector
  const langSel = document.getElementById('settings-lang-select');
  if (langSel) {
    langSel.value = getCurrentLocale();
    langSel.addEventListener('change', (e) => {
      setLocale(e.target.value);
      // Also sync sidebar selector
      const sidebar = document.getElementById('lang-select');
      if (sidebar) sidebar.value = e.target.value;
    });
  }

  // Notifications toggle
  const toastToggle = document.getElementById('toggle-toasts');
  if (toastToggle) {
    toastToggle.checked = areToastsEnabled();
    toastToggle.addEventListener('change', (e) => {
      setToastsEnabled(e.target.checked);
      logSuccess(e.target.checked ? 'Повідомлення увімкнено.' : 'Повідомлення вимкнено.');
    });
  }

  // Keybinds events
  document.getElementById('btn-reset-binds')?.addEventListener('click', () => {
    resetKeybinds();
    renderKeybindsList();
    logSuccess('Keybinds reset to defaults.');
  });

  document.getElementById('btn-export-cfg')?.addEventListener('click', () => {
    const cfg = exportConfig();
    const blob = new Blob([cfg], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'rustopti.cfg';
    a.click();
    logSuccess('Config exported as rustopti.cfg');
  });

  document.getElementById('cfg-file-input')?.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const text = await file.text();
    const ok = importConfig(text);
    if (ok) { renderKeybindsList(); logSuccess('Config imported successfully!'); }
    else logError('Invalid config file.');
    e.target.value = '';
  });

  document.getElementById('btn-reset-license')?.addEventListener('click', async () => {
    if (confirm('Reset license and return to activation screen?')) {
      try {
        await api.revokeLicense();
      } catch (e) {
        // Fallback: even if backend fails, reload to re-check
      }
      window.location.reload();
    }
  });

  document.getElementById('btn-create-backup')?.addEventListener('click', async () => {
    const desc = document.getElementById('backup-desc').value.trim() || 'Manual Backup';
    logInfo(`Creating system restore point: ${desc}...`);
    try {
      const res = await api.createRestorePoint(desc);
      logSuccess(res);
      await loadBackups();
    } catch (e) {
      logError(`Failed: ${e}`);
    }
  });

  container.addEventListener('click', async (e) => {
    if (e.target.classList.contains('btn-restore')) {
      const filename = e.target.dataset.file;
      logInfo(`Opening registry editor to restore ${filename}...`);
      try {
        const res = await api.restoreRegistryBackup(filename);
        logSuccess(res);
      } catch (err) {
        logError(`Restore failed: ${err}`);
      }
    }
  });
}

// ── Keybinds UI ──────────────────────────────────────────────────
let listeningEl = null;
let listeningId = null;

function renderKeybindsList() {
  const el = document.getElementById('keybinds-list');
  if (!el) return;
  const binds = getKeybinds();

  el.innerHTML = Object.entries(binds).map(([id, bind]) => `
    <div class="keybind-row">
      <span class="keybind-label">${bind.label}</span>
      <div class="keybind-key-wrap">
        <span class="keybind-badge" data-bind-id="${id}">${bind.key}</span>
      </div>
    </div>
  `).join('');

  // Click a badge → start listening
  el.querySelectorAll('.keybind-badge').forEach(badge => {
    badge.addEventListener('click', () => {
      // Cancel previous
      if (listeningEl) {
        listeningEl.classList.remove('listening');
        listeningEl.textContent = getKeybinds()[listeningId]?.key || '?';
      }
      listeningEl = badge;
      listeningId = badge.dataset.bindId;
      badge.classList.add('listening');
      badge.textContent = '...';
    });
  });
}

// Global key capture for keybind recording
document.addEventListener('keydown', (e) => {
  if (!listeningEl || !listeningId) return;
  e.preventDefault();
  const key = e.key === ' ' ? 'Space' : e.key;
  setKeybind(listeningId, key);
  listeningEl.classList.remove('listening');
  listeningEl.textContent = key;
  listeningEl = null;
  listeningId = null;
});

async function loadBackups() {
  try {
    const backups = await api.listBackups();
    const el = document.getElementById('backup-list');
    
    if (backups.length === 0) {
      el.innerHTML = `<div style="color:var(--text-muted);font-size:13px;padding:12px">${t('set.no_backups')}</div>`;
      return;
    }

    el.innerHTML = backups.map(b => `
      <div class="tweak-item">
        <div style="display:flex;flex-direction:column;gap:4px">
          <span class="tweak-name" style="font-family:var(--font-mono);font-size:12px">${b}</span>
        </div>
        <button class="btn btn-ripple btn-restore" data-file="${b}" style="padding:4px 12px;font-size:11px;background:rgba(255,255,255,0.05)">${t('set.btn.restore')}</button>
      </div>
    `).join('');
  } catch (e) {
    logError(`Failed to load backups: ${e}`);
  }
}
