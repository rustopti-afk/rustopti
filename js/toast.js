// Toast notification system
const TOAST_SETTING_KEY = 'rustopti_toasts';

let toastContainer = null;

function getContainer() {
  if (!toastContainer) {
    toastContainer = document.createElement('div');
    toastContainer.id = 'toast-container';
    document.body.appendChild(toastContainer);
  }
  return toastContainer;
}

// Check if toasts are enabled (stored in localStorage)
export function areToastsEnabled() {
  return localStorage.getItem(TOAST_SETTING_KEY) !== 'off';
}

export function setToastsEnabled(val) {
  localStorage.setItem(TOAST_SETTING_KEY, val ? 'on' : 'off');
}

// Show action toast: "Ви активували: Name"
export function showToast(actionName, type = 'success') {
  if (!areToastsEnabled()) return;
  _show(
    `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
     <span>Ви активували: <b>${actionName}</b></span>`,
    type,
    3000
  );
}

// Show warning toast (overheating, etc.) — always visible regardless of toggle
export function showWarning(message) {
  _show(
    `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
     <span>${message}</span>`,
    'warning',
    6000
  );
}

function _show(innerHtml, type, duration) {
  const el = document.createElement('div');
  el.className = `toast toast-${type}`;
  el.innerHTML = innerHtml;
  getContainer().appendChild(el);

  requestAnimationFrame(() => el.classList.add('toast-show'));

  setTimeout(() => {
    el.classList.remove('toast-show');
    el.classList.add('toast-hide');
    el.addEventListener('transitionend', () => el.remove(), { once: true });
  }, duration);
}
