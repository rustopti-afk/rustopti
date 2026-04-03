// Keybinds system — customizable hotkeys for functions
import { playEnable } from './sounds.js';
import { showToast } from './toast.js';
import { navigateTo } from './router.js';

// Default keybinds config
const DEFAULT_BINDS = {
  cleanup_cache:      { key: 'F1',  label: 'Cleanup Cache',         page: 'cleanup',     btnId: 'btn-run-cleanup' },
  apply_registry:     { key: 'F2',  label: 'Apply Registry Tweaks', page: 'system',      btnId: 'btn-apply-reg' },
  apply_network:      { key: 'F3',  label: 'Apply Network Tweaks',  page: 'network',     btnId: 'btn-apply-net' },
  apply_gpu:          { key: 'F4',  label: 'Apply GPU Tweaks',      page: 'gpu',         btnId: 'btn-apply-gpu' },
  clear_standby:      { key: 'F5',  label: 'Clear Standby RAM',     page: 'deep-tweaks', btnId: 'btn-clear-standby' },
  boost_timer:        { key: 'F6',  label: 'Boost Timer',           page: 'deep-tweaks', btnId: 'btn-boost-timer' },
  game_mode:          { key: 'F7',  label: 'Activate Game Mode',    page: 'game-boost',  btnId: 'btn-activate-gm' },
  active_protection:  { key: 'F8',  label: 'Active Protection ON',  page: 'game-boost',  btnId: 'btn-start-protection' },
};

const STORAGE_KEY = 'rustopti_keybinds';

// Load keybinds from localStorage (or use defaults)
export function getKeybinds() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
    return { ...DEFAULT_BINDS, ...saved };
  } catch {
    return { ...DEFAULT_BINDS };
  }
}

// Save single keybind
export function setKeybind(id, key) {
  const binds = getKeybinds();
  if (!binds[id]) return;
  binds[id] = { ...binds[id], key };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(
    Object.fromEntries(Object.entries(binds).map(([k, v]) => [k, { key: v.key }]))
  ));
}

// Reset all to defaults
export function resetKeybinds() {
  localStorage.removeItem(STORAGE_KEY);
}

// Export config as JSON string (for cfg file)
export function exportConfig() {
  const binds = getKeybinds();
  const cfg = {
    version: 1,
    app: 'RustOpti',
    keybinds: Object.fromEntries(
      Object.entries(binds).map(([id, v]) => [id, { key: v.key, label: v.label }])
    )
  };
  return JSON.stringify(cfg, null, 2);
}

// Import config from JSON string
export function importConfig(jsonStr) {
  try {
    const cfg = JSON.parse(jsonStr);
    if (!cfg.keybinds) throw new Error('Invalid config');
    const toSave = {};
    for (const [id, v] of Object.entries(cfg.keybinds)) {
      if (DEFAULT_BINDS[id] && v.key) toSave[id] = { key: v.key };
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(toSave));
    return true;
  } catch {
    return false;
  }
}

// Initialize global keydown listener
export function initKeybinds() {
  document.addEventListener('keydown', async (e) => {
    // Don't fire when typing in inputs
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

    const binds = getKeybinds();
    for (const [, bind] of Object.entries(binds)) {
      if (e.key === bind.key) {
        e.preventDefault();
        await triggerBind(bind);
        break;
      }
    }
  });
}

async function triggerBind(bind) {
  // Navigate to correct page
  const { getCurrentPage } = await import('./router.js');
  if (getCurrentPage() !== bind.page) {
    navigateTo(bind.page);
    // Wait for page render
    await new Promise(r => setTimeout(r, 400));
  }

  // Click the button if present
  const btn = document.getElementById(bind.btnId);
  if (btn && !btn.disabled) btn.click();

  playEnable();
  showToast(bind.label);
}
