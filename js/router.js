// NeonRust Pro — Page Router (SPA)
// ═══════════════════════════════════════════════════════════════

import { renderDashboard } from '../pages/dashboard.js';
import { renderSystem } from '../pages/system.js';
import { renderGpu } from '../pages/gpu.js';
import { renderNetwork } from '../pages/network.js';
import { renderProcess } from '../pages/process.js';
import { renderRustTweaks } from '../pages/rust-tweaks.js';
import { renderCleanup } from '../pages/cleanup.js';
import { renderSettings } from '../pages/settings.js';
import { renderDeepTweaks } from '../pages/deep-tweaks.js';
import { renderActivation } from '../pages/activation.js';
import { renderGameBoost } from '../pages/game-boost.js';
import { renderAccount } from '../pages/account.js';
import { renderGameMode } from '../pages/gamemode.js';
import { renderSmartBoost } from '../pages/smartboost.js';
import * as api from './api.js';

const routes = {
  'dashboard': renderDashboard,
  'system': renderSystem,
  'gpu': renderGpu,
  'network': renderNetwork,
  'process': renderProcess,
  'rust-tweaks': renderRustTweaks,
  'cleanup': renderCleanup,
  'settings': renderSettings,
  'deep-tweaks': renderDeepTweaks,
  'game-boost': renderGameBoost,
  'account': renderAccount,
  'gamemode': renderGameMode,
  'smartboost': renderSmartBoost,
  'activation': renderActivation,
};

let currentPage = 'dashboard';
const isTauri = () => !!window.__TAURI_INTERNALS__;

export async function navigateTo(page) {
  const contentArea = document.getElementById('content-area');
  if (!contentArea || !routes[page]) return;

  // License Check — ask the Rust backend, not localStorage
  if (isTauri() && page !== 'activation') {
    try {
      const licensed = await api.checkLicenseStatus();
      if (!licensed) {
        return navigateTo('activation');
      }
    } catch {
      // If backend check fails, redirect to activation
      return navigateTo('activation');
    }
  }

  // Hide sidebar on activation page for "Lock" feel
  const sidebar = document.querySelector('.sidebar');
  const mainWrapper = document.querySelector('.main-wrapper');
  if (sidebar) {
    sidebar.style.display = (page === 'activation') ? 'none' : 'flex';
  }
  if (mainWrapper) {
    mainWrapper.style.marginLeft = (page === 'activation') ? '0' : '';
  }

  currentPage = page;

  // Update active nav
  document.querySelectorAll('.nav-item').forEach(item => {
    item.classList.toggle('active', item.dataset.page === page);
  });

  // Render page
  contentArea.innerHTML = '';
  contentArea.style.opacity = '0';

  requestAnimationFrame(() => {
    routes[page](contentArea);
    contentArea.style.transition = 'opacity 0.3s ease';
    contentArea.style.opacity = '1';
  });
}

export function initRouter() {
  document.querySelectorAll('.nav-item').forEach(item => {
    item.addEventListener('click', (e) => {
      e.preventDefault();
      const page = item.dataset.page;
      if (page) navigateTo(page);
    });
  });

  window.addEventListener('languagechange', () => {
    if (currentPage && routes[currentPage]) {
      const contentArea = document.getElementById('content-area');
      if (contentArea) routes[currentPage](contentArea);
    }
  });

  navigateTo('dashboard');
}

export function getCurrentPage() {
  return currentPage;
}
