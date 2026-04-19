import * as api from '../js/api.js';
import { t } from '../js/i18n.js';

const isTauri = () => !!window.__TAURI_INTERNALS__;

export async function renderActivation(container) {
  container.innerHTML = `
    <div class="activation-page" style="display:flex; flex-direction:column; align-items:center; justify-content:center; height:100%; text-align:center; padding:40px;">
      <div class="license-card card-enter" style="max-width:450px; width:100%; padding:32px; background:rgba(20,20,30,0.8); border:1px solid var(--accent); border-radius:var(--radius-md); box-shadow: 0 0 30px rgba(200, 200, 200, 0.1);">
        <h1 class="neon-pulse" style="font-size:32px; margin-bottom:12px; color:var(--accent)">RustOpti</h1>
        <p style="color:var(--text-dim); margin-bottom:24px;">${t('act.desc')}</p>

        ${!isTauri() ? `
          <div style="padding:20px; background:rgba(255,77,106,0.1); border:1px solid rgba(255,77,106,0.3); border-radius:var(--radius-sm); margin-bottom:20px;">
            <p style="color:#ff4d6a; font-size:14px;">License activation requires the desktop app.</p>
            <a href="https://github.com/rustopti-afk/rustopti/releases/download/v2.2.27/NexOpti_2.2.22_x64-setup.exe" class="btn btn-primary" style="margin-top:12px; display:inline-block; padding:10px 24px;">
              Download NexOpti
            </a>
          </div>
        ` : `
          <div style="margin-bottom:24px; text-align:left;">
            <label style="display:block; font-size:12px; color:var(--accent); text-transform:uppercase; letter-spacing:1px; margin-bottom:8px;">License Key</label>
            <input type="text" id="license-key-input" placeholder="RUST-XXXX-XXXX-XXXX"
              style="width:100%; padding:14px; background:rgba(0,0,0,0.3); border:1px solid var(--border); border-radius:var(--radius-sm); color:white; font-family:var(--font-mono); font-size:16px; outline:none;"
              value="">
          </div>

          <button id="btn-activate" class="btn btn-primary btn-ripple" style="width:100%; padding:14px; font-size:16px;">
            ${t('act.btn')}
          </button>

          <div id="activation-status" style="margin-top:20px; font-size:14px; min-height:20px;"></div>
        `}
      </div>

      ${isTauri() ? `
        <div style="margin-top:24px; color:var(--text-muted); font-size:11px;">
          HWID: <span id="display-hwid" style="font-family:var(--font-mono);">Loading...</span>
        </div>
      ` : ''}
    </div>
  `;

  // Web mode — no activation possible
  if (!isTauri()) return;

  const statusEl = document.getElementById('activation-status');
  const hwidEl = document.getElementById('display-hwid');
  const keyInput = document.getElementById('license-key-input');

  // Show HWID (masked — only last 8 chars visible)
  let currentHwid = "";
  try {
    currentHwid = await api.getHwid();
    const masked = '••••••••' + currentHwid.slice(-8);
    hwidEl.textContent = masked;
  } catch (e) {
    hwidEl.textContent = 'ERROR';
  }

  document.getElementById('btn-activate').addEventListener('click', async () => {
    const key = keyInput.value.trim();
    if (!key) {
      statusEl.innerHTML = `<span style="color:#ff4d6a">Please enter a key</span>`;
      return;
    }

    // Basic format validation before sending to server
    if (!/^RUST-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(key)) {
      statusEl.innerHTML = `<span style="color:#ff4d6a">Invalid key format. Expected: RUST-XXXX-XXXX-XXXX</span>`;
      return;
    }

    statusEl.innerHTML = `<div class="loading-spinner" style="width:20px; height:20px; margin:0 auto;"></div>`;

    try {
      const result = await api.validateLicenseKey(key);

      if (result.success) {
        statusEl.innerHTML = `<span style="color:#4dff91">${result.message}</span>`;
        // License state is now stored in Rust memory — no localStorage needed
        setTimeout(() => window.location.reload(), 1500);
      } else {
        const errMsg = result.error || result.message || "Unknown error";
        statusEl.innerHTML = `<span style="color:#ff4d6a">${errMsg}</span>`;
      }
    } catch (e) {
      statusEl.innerHTML = `<span style="color:#ff4d6a">System Error: ${e}</span>`;
    }
  });
}
