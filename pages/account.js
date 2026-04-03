import * as api from '../js/api.js';

export async function renderAccount(container) {
  container.innerHTML = `
    <div class="page-header">
      <h2 class="page-title neon-pulse">Кабінет</h2>
      <p class="page-subtitle">Інформація про вашу підписку RustOpti</p>
    </div>
    <div id="account-content" style="display:flex;align-items:center;justify-content:center;padding:40px">
      <div class="loading-spinner"></div>
    </div>
  `;

  try {
    const info = await api.getLicenseInfo();
    renderAccountInfo(container, info);
  } catch (e) {
    renderAccountInfo(container, { active: false });
  }
}

function renderAccountInfo(container, info) {
  const content = document.getElementById('account-content');
  if (!content) return;

  if (!info.active) {
    content.innerHTML = `
      <div class="section" style="max-width:480px;width:100%">
        <div style="text-align:center;padding:32px">
          <div style="font-size:48px;margin-bottom:16px">🔒</div>
          <h3 style="color:var(--text-main);margin:0 0 8px">Ліцензія не активована</h3>
          <p style="color:var(--text-muted);font-size:13px">Введіть ключ на сторінці активації</p>
        </div>
      </div>
    `;
    return;
  }

  const plan = info.plan;
  const isLifetime = plan === 'lifetime';
  const expiresAt = info.expires_at;

  // Calculate days remaining
  let daysLeft = null;
  let expiryDisplay = '∞ Назавжди';
  let expiryColor = '#a855f7';
  let urgentBar = '';

  if (!isLifetime && expiresAt) {
    const expDate = new Date(expiresAt.replace(' ', 'T'));
    const now = new Date();
    const msLeft = expDate - now;
    daysLeft = Math.ceil(msLeft / (1000 * 60 * 60 * 24));

    const day = String(expDate.getDate()).padStart(2, '0');
    const month = String(expDate.getMonth() + 1).padStart(2, '0');
    const year = expDate.getFullYear();
    expiryDisplay = `${day}.${month}.${year}`;

    if (daysLeft <= 0) {
      expiryColor = '#ef4444';
      expiryDisplay = 'Прострочено';
    } else if (daysLeft <= 3) {
      expiryColor = '#ef4444';
      urgentBar = `
        <div style="background:rgba(239,68,68,0.1);border:1px solid rgba(239,68,68,0.3);border-radius:8px;padding:12px 16px;margin-bottom:16px;display:flex;align-items:center;gap:10px">
          <span style="font-size:20px">⚠️</span>
          <div>
            <div style="color:#ef4444;font-weight:600;font-size:13px">Підписка закінчується скоро!</div>
            <div style="color:#9ca3af;font-size:12px;margin-top:2px">Залишилось ${daysLeft} ${getDayWord(daysLeft)} — продовжіть на сайті</div>
          </div>
        </div>
      `;
    } else if (daysLeft <= 7) {
      expiryColor = '#f59e0b';
    } else {
      expiryColor = '#22c55e';
    }
  }

  const planLabel = isLifetime
    ? '<span style="color:#a855f7">👑 Lifetime</span>'
    : '<span style="color:#60a5fa">⚡ Місячна</span>';

  const daysBlock = (!isLifetime && daysLeft !== null) ? `
    <div class="stat-card" style="grid-column:span 2">
      <div style="display:flex;align-items:center;justify-content:space-between">
        <span style="color:var(--text-muted);font-size:13px">Залишилось днів</span>
        <span style="font-size:32px;font-weight:700;color:${expiryColor}">${daysLeft > 0 ? daysLeft : '0'}</span>
      </div>
      <div style="margin-top:10px;height:4px;background:rgba(255,255,255,0.05);border-radius:2px;overflow:hidden">
        <div style="height:100%;width:${Math.min(100, Math.max(0, (daysLeft / 30) * 100))}%;background:${expiryColor};border-radius:2px;transition:width 0.5s ease"></div>
      </div>
    </div>
  ` : '';

  content.innerHTML = `
    <div style="max-width:520px;width:100%">

      ${urgentBar}

      <div class="section">
        <h3 class="section-title" style="margin-bottom:16px">📋 Підписка</h3>

        <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:16px">

          <div class="stat-card">
            <div style="color:var(--text-muted);font-size:12px;margin-bottom:6px">Тип плану</div>
            <div style="font-size:16px;font-weight:600">${planLabel}</div>
          </div>

          <div class="stat-card">
            <div style="color:var(--text-muted);font-size:12px;margin-bottom:6px">Діє до</div>
            <div style="font-size:16px;font-weight:600;color:${expiryColor}">${expiryDisplay}</div>
          </div>

          ${daysBlock}

        </div>

        <div style="background:rgba(168,85,247,0.05);border:1px solid rgba(168,85,247,0.2);border-radius:8px;padding:12px 16px">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
            <span style="color:#a855f7;font-size:14px">✅</span>
            <span style="color:var(--text-main);font-size:13px;font-weight:500">Ліцензія активна</span>
          </div>
          <p style="color:var(--text-muted);font-size:12px;margin:0">
            Всі функції розблоковані. HWID прив'язка активна — ключ захищено від копіювання.
          </p>
        </div>
      </div>

      ${!isLifetime ? `
      <div class="section" style="margin-top:12px">
        <h3 class="section-title" style="margin-bottom:12px">⬆️ Апгрейд</h3>
        <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 16px;background:rgba(168,85,247,0.05);border:1px solid rgba(168,85,247,0.2);border-radius:8px">
          <div>
            <div style="color:var(--text-main);font-weight:600;font-size:13px">Lifetime ліцензія</div>
            <div style="color:var(--text-muted);font-size:12px;margin-top:2px">Без терміну дії · Пріоритетна підтримка</div>
          </div>
          <a href="https://rustopti.fun/pricing" target="_blank"
            style="background:linear-gradient(135deg,#7c3aed,#a855f7);color:#fff;font-size:12px;font-weight:600;padding:8px 16px;border-radius:6px;text-decoration:none;white-space:nowrap">
            Придбати →
          </a>
        </div>
      </div>
      ` : ''}

    </div>
  `;
}

function getDayWord(n) {
  const abs = Math.abs(n);
  if (abs % 10 === 1 && abs % 100 !== 11) return 'день';
  if ([2,3,4].includes(abs % 10) && ![12,13,14].includes(abs % 100)) return 'дні';
  return 'днів';
}
