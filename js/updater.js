import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export async function checkForUpdates(silent = false) {
  try {
    const update = await check();
    if (!update) {
      if (!silent) showToast('Оновлень немає — у тебе остання версія', 'info');
      return;
    }

    showUpdateBanner(update);
  } catch (e) {
    if (!silent) showToast('Не вдалось перевірити оновлення', 'error');
  }
}

function showUpdateBanner(update) {
  // Remove existing banner if any
  document.getElementById('update-banner')?.remove();

  const banner = document.createElement('div');
  banner.id = 'update-banner';
  banner.innerHTML = `
    <div class="update-banner-content">
      <span class="update-icon">⬆</span>
      <div>
        <div class="update-title">Доступне оновлення v${update.version}</div>
        <div class="update-notes">${update.body || ''}</div>
      </div>
      <button class="update-btn" id="do-update-btn">Встановити</button>
      <button class="update-dismiss" id="dismiss-update-btn">✕</button>
    </div>
    <div class="update-progress" id="update-progress" style="display:none">
      <div class="update-progress-bar" id="update-progress-bar"></div>
      <span id="update-progress-label">Завантаження...</span>
    </div>
  `;
  document.body.appendChild(banner);

  document.getElementById('dismiss-update-btn').onclick = () => banner.remove();
  document.getElementById('do-update-btn').onclick = () => installUpdate(update, banner);
}

async function installUpdate(update, banner) {
  const btn = document.getElementById('do-update-btn');
  const progress = document.getElementById('update-progress');
  const bar = document.getElementById('update-progress-bar');
  const label = document.getElementById('update-progress-label');

  btn.disabled = true;
  progress.style.display = 'block';

  let downloaded = 0;
  let total = 0;

  await update.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      total = event.data.contentLength || 0;
      label.textContent = 'Завантаження...';
    } else if (event.event === 'Progress') {
      downloaded += event.data.chunkLength;
      if (total > 0) {
        const pct = Math.round((downloaded / total) * 100);
        bar.style.width = pct + '%';
        label.textContent = `${pct}%`;
      }
    } else if (event.event === 'Finished') {
      label.textContent = 'Встановлення...';
    }
  });

  label.textContent = 'Готово! Перезапуск...';
  setTimeout(() => relaunch(), 1500);
}

function showToast(msg, type = 'info') {
  const t = document.createElement('div');
  t.className = `toast toast-${type}`;
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 3000);
}
