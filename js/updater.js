const CURRENT_VERSION = '2.2.16';
const RELEASES_API = 'https://api.github.com/repos/rustopti-afk/rustopti/releases/latest';

export async function checkForUpdates(silent = false) {
  if (!window.__TAURI_INTERNALS__) return;

  try {
    const res = await fetch(RELEASES_API);
    if (!res.ok) return;
    const data = await res.json();

    const latest = data.tag_name?.replace('v', '');
    if (!latest) return;

    if (isNewer(latest, CURRENT_VERSION)) {
      const exeAsset = data.assets?.find(a => a.name.endsWith('-setup.exe'));
      showUpdateBanner(latest, exeAsset?.browser_download_url);
    } else if (!silent) {
      showToast('У тебе остання версія', 'info');
    }
  } catch {
    if (!silent) showToast('Не вдалось перевірити оновлення', 'error');
  }
}

function isNewer(latest, current) {
  const a = latest.split('.').map(Number);
  const b = current.split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    if ((a[i] || 0) > (b[i] || 0)) return true;
    if ((a[i] || 0) < (b[i] || 0)) return false;
  }
  return false;
}

async function openUrl(url) {
  try {
    // Tauri v2 shell plugin
    const { open } = await import('@tauri-apps/plugin-shell');
    await open(url);
  } catch {
    // fallback
    window.open(url, '_blank');
  }
}

function showUpdateBanner(version, downloadUrl) {
  document.getElementById('update-banner')?.remove();

  const banner = document.createElement('div');
  banner.id = 'update-banner';
  banner.innerHTML = `
    <div class="update-banner-content">
      <span class="update-icon">⬆</span>
      <div>
        <div class="update-title">Доступне оновлення v${version}</div>
        <div class="update-notes">Натисни щоб завантажити нову версію</div>
      </div>
      <button class="update-btn" id="do-update-btn">Завантажити</button>
      <button class="update-dismiss" id="dismiss-update-btn">✕</button>
    </div>
  `;
  document.body.appendChild(banner);

  document.getElementById('dismiss-update-btn').onclick = () => banner.remove();
  document.getElementById('do-update-btn').onclick = () => {
    if (downloadUrl) openUrl(downloadUrl);
  };
}

function showToast(msg, type = 'info') {
  const t = document.createElement('div');
  t.className = `toast toast-${type}`;
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 3000);
}
