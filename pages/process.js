import * as api from '../js/api.js';
import { logInfo, logSuccess, logError } from '../js/terminal.js';
import { t } from '../js/i18n.js';

export async function renderProcess(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">${t('proc.title')}</h2>
      <p class="page-subtitle">${t('proc.subtitle')}</p>
    </div>

    <div class="card-grid">
      <div class="card card-enter hex-bg">
        <div class="card-icon">[CPU]</div>
        <div class="card-title">${t('proc.bloatware')}</div>
        <div class="card-value" id="btn-kill-bloat" style="cursor:pointer;color:var(--accent-primary)">${t('proc.kill_now')}</div>
        <div class="card-subtitle">${t('proc.term_apps')}</div>
      </div>
      <div class="card card-enter hex-bg" style="animation-delay:0.1s">
        <div class="card-icon">[RUST]</div>
        <div class="card-title">${t('proc.rust_proc')}</div>
        <div class="card-value" id="rust-status">${t('proc.not_running')}</div>
        <div class="card-subtitle">${t('proc.auto_detect')}</div>
      </div>
    </div>

    <div class="section">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;border-bottom:1px solid var(--border);padding-bottom:12px;">
        <h3 class="section-title" style="margin:0;border:none;padding:0">${t('proc.running_procs')}</h3>
        <button class="btn btn-ripple" id="btn-refresh-proc">${t('proc.btn.refresh')}</button>
      </div>

      <div style="max-height:400px;overflow-y:auto;background:rgba(0,0,0,0.15);border:1px solid var(--border);border-radius:var(--radius-sm);">
        <table class="process-table" id="proc-table">
          <thead>
            <tr>
              <th>${t('proc.col.pid')}</th>
              <th>${t('proc.col.name')}</th>
              <th>${t('proc.col.mem')}</th>
              <th>${t('proc.col.cpu')}</th>
              <th>${t('proc.col.actions')}</th>
            </tr>
          </thead>
          <tbody id="proc-list">
            <tr><td colspan="5" style="text-align:center">${t('txt.loading')}</td></tr>
          </tbody>
        </table>
      </div>
    </div>
  `;

  await loadProcesses();

  // Event delegation for kill buttons (attached once, not per refresh)
  document.getElementById('proc-list')?.addEventListener('click', async (e) => {
    const btn = e.target.closest('.btn-kill-proc');
    if (!btn) return;
    const pid = parseInt(btn.dataset.pid);
    const name = btn.dataset.name;
    logInfo(`Killing process ${name} (${pid})...`);
    try {
      const res = await api.killProcess(pid);
      logSuccess(res);
      setTimeout(() => document.getElementById('btn-refresh-proc')?.click(), 500);
    } catch (err) {
      logError(`Failed to kill process: ${err}`);
    }
  });

  document.getElementById('btn-refresh-proc')?.addEventListener('click', loadProcesses);
  document.getElementById('btn-kill-bloat')?.addEventListener('click', async () => {
    logInfo('Killing bloatware...');
    try {
      const results = await api.killBloatware();
      results.forEach(r => logSuccess(r));
      await loadProcesses();
    } catch (e) { logError(`Failed: ${e}`); }
  });
}

const PROTECTED = new Set([
  'system', 'idle', 'registry', 'smss.exe', 'csrss.exe', 'wininit.exe',
  'winlogon.exe', 'lsass.exe', 'services.exe', 'svchost.exe', 'dwm.exe',
  'explorer.exe', 'audiodg.exe', 'spoolsv.exe', 'fontdrvhost.exe',
  'taskhostw.exe', 'sihost.exe', 'ctfmon.exe', 'runtimebroker.exe',
  'securityhealthservice.exe', 'wmiprvse.exe', 'wudfhost.exe',
  'msdtc.exe', 'lsm.exe', 'ntoskrnl.exe', 'hal.dll', 'conhost.exe',
  'dllhost.exe', 'searchindexer.exe', 'sppsvc.exe', 'msiexec.exe',
  'wlanext.exe', 'unsecapp.exe', 'taskmgr.exe', 'rustopti.exe',
]);

async function loadProcesses() {
  try {
    const allProcs = await api.getProcessList();
    const procs = allProcs.filter(p => !PROTECTED.has(p.name.toLowerCase()));

    const rustProc = allProcs.find(p => p.name.toLowerCase().includes('rust') && p.name.toLowerCase() !== 'rustopti.exe');
    const rustStatus = document.getElementById('rust-status');
    if (rustStatus) {
      rustStatus.textContent = rustProc ? `${t('proc.running')} (${rustProc.pid})` : t('proc.not_running');
      rustStatus.style.color = rustProc ? 'var(--success)' : 'var(--text-disabled)';
    }

    const memKey = 'memory_mb' in (procs[0] || {}) ? 'memory_mb' : 'memory_usage';
    procs.sort((a, b) => (b[memKey] || 0) - (a[memKey] || 0));

    document.getElementById('proc-list').innerHTML = procs.map(p => {
      const mem = p.memory_mb ?? p.memory_usage ?? 0;
      return `
      <tr>
        <td style="font-family:var(--font-mono)">${p.pid}</td>
        <td>${p.name}</td>
        <td>${mem.toFixed(1)}</td>
        <td>${p.cpu_usage.toFixed(1)}</td>
        <td>
          <button class="btn btn-danger btn-kill-proc" data-pid="${p.pid}" data-name="${p.name}" style="padding:4px 8px;font-size:11px">${t('proc.btn.kill')}</button>
        </td>
      </tr>`;
    }).join('');
  } catch (e) {
    logError(`Failed to load processes: ${e}`);
  }
}
