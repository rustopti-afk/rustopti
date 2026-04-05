import { initRouter, navigateTo } from './router.js';
import { initTerminal, log, logInfo, logError } from './terminal.js';
import * as api from './api.js';
import { initI18n, setLocale, getCurrentLocale } from './i18n.js';
import { initKeybinds } from './keybinds.js';
import { showWarning } from './toast.js';
import { checkForUpdates } from './updater.js';

// Anti-tamper: freeze the Tauri internals reference so it can't be
// overwritten from DevTools to force web-mock mode
(function lockTauriEnv() {
  if (window.__TAURI_INTERNALS__) {
    try {
      Object.defineProperty(window, '__TAURI_INTERNALS__', {
        value: window.__TAURI_INTERNALS__,
        writable: false,
        configurable: false,
      });
    } catch (_) {}
  }
})();

let statsInterval = null;
let lastOverheatWarning = 0;
const OVERHEAT_THRESHOLD = 85; // °C
const OVERHEAT_COOLDOWN  = 60_000; // show warning max once per minute

async function updateStatusBar() {
  try {
    const stats = await api.getRealtimeStats();
    document.getElementById('status-cpu').textContent = `CPU: ${stats.cpu_usage.toFixed(1)}%`;
    document.getElementById('status-ram').textContent = `RAM: ${(stats.ram_used_mb / 1024).toFixed(1)}GB`;

    // Show temperature in status bar if available
    const tempEl = document.getElementById('status-temp');
    if (tempEl) {
      if (stats.cpu_temp_c != null) {
        const t = stats.cpu_temp_c;
        const hot = t >= OVERHEAT_THRESHOLD;
        tempEl.textContent = `CPU: ${t.toFixed(0)}°C`;
        tempEl.style.color = hot ? 'var(--error)' : '';

        // Overheat warning toast
        if (hot && Date.now() - lastOverheatWarning > OVERHEAT_COOLDOWN) {
          lastOverheatWarning = Date.now();
          showWarning(`⚠ Процесор перегрівається! ${t.toFixed(0)}°C — перевір охолодження`);
        }
      } else {
        tempEl.textContent = '';
      }
    }
  } catch (e) {
    // Ignore errors for silent background updating
  }
}

async function initStatusBar() {
  try {
    const info = await api.getSystemInfo();
    const gpuInfo = document.getElementById('status-gpu');
    if (gpuInfo) gpuInfo.textContent = info.gpu_info || document.querySelector('[data-i18n="status.ready"]')?.textContent || 'GPU Ready';
    await updateStatusBar();
  } catch (e) {
    logError('Failed to initialize status bar metrics');
  }
  statsInterval = setInterval(updateStatusBar, 5000);
}

// ═══════════════════════════════════════════════════════════════
// License revalidation — checks server if cache is stale
// ═══════════════════════════════════════════════════════════════
async function checkLicense(forceRevalidate = false) {
  if (!window.__TAURI_INTERNALS__) return;

  try {
    const cacheStatus = await api.getLicenseCacheStatus();

    // Always revalidate on startup (forceRevalidate=true) or when cache is stale
    if (forceRevalidate || cacheStatus === 'needs_recheck') {
      const result = await api.revalidateLicense();

      if (result.status === 'expired') {
        logError('Підписка закінчилась. Відкочуємо всі зміни...');
        await api.subscriptionExpiredCleanup().catch(() => {});
        logError('Всі оптимізації вимкнено. Оновіть підписку.');
        navigateTo('activation');
      } else if (result.status === 'valid') {
        log('License verified.', 'success');
      }
      // If status === 'offline' — server unreachable, keep cached state
    } else if (cacheStatus === 'expired') {
      logError('Підписка закінчилась. Відкочуємо всі зміни...');
      await api.subscriptionExpiredCleanup().catch(() => {});
      logError('Всі оптимізації вимкнено. Оновіть підписку.');
      navigateTo('activation');
    }
  } catch {
    // If check fails, don't block
  }
}

// Start periodic license check every 5 minutes
function startLicenseMonitor() {
  if (!window.__TAURI_INTERNALS__) return;
  setInterval(() => checkLicense(true), 5 * 60 * 1000);
}

document.addEventListener('DOMContentLoaded', async () => {
  // Initialize UI components
  initI18n();
  initTerminal();
  initRouter();

  // Check for updates silently on startup (no popup if up to date)
  setTimeout(() => checkForUpdates(true), 3000);

  const langSelect = document.getElementById('lang-select');
  if (langSelect) {
    langSelect.value = getCurrentLocale();
    langSelect.addEventListener('change', (e) => {
      setLocale(e.target.value);
    });
  }

  // Web Mode – Hide download section if already in the app
  const downloadSection = document.getElementById('web-download-section');
  if (downloadSection && !!window.__TAURI_INTERNALS__) {
      downloadSection.style.display = 'none';
  }

  // Advanced section toggle
  const advToggle = document.getElementById('nav-advanced-toggle');
  const advItems = document.getElementById('nav-advanced-items');
  if (advToggle && advItems) {
    advToggle.addEventListener('click', () => {
      advToggle.classList.toggle('open');
      advItems.classList.toggle('open');
    });
  }

  initKeybinds();
  log('RustOpti Engine started.', 'success');
  logInfo('Initializing system telemetry...');

  await initStatusBar();
  log('System monitoring active.', 'success');

  // Force revalidate against server on startup — catches revoked licenses immediately
  checkLicense(true);
  startLicenseMonitor();
});
