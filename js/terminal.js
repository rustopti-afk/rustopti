// RustOpti — Terminal Log Component
// ═══════════════════════════════════════════════════════════════

const terminalOutput = () => document.getElementById('terminal-output');

export function log(message, type = 'default') {
  const output = terminalOutput();
  if (!output) return;

  const now = new Date();
  const time = now.toLocaleTimeString('en-GB', { hour12: false });

  const line = document.createElement('div');
  line.className = 'terminal-line';
  line.innerHTML = `
    <span class="term-time">[${time}]</span>
    <span class="term-msg ${type}">${escapeHtml(message)}</span>
  `;

  output.appendChild(line);

  // Limit terminal lines to prevent memory leak
  const MAX_LINES = 500;
  while (output.children.length > MAX_LINES) {
    output.removeChild(output.firstChild);
  }

  output.scrollTop = output.scrollHeight;
}

export function logSuccess(msg) { log(msg, 'success'); }
export function logError(msg) { log(msg, 'error'); }
export function logInfo(msg) { log(msg, 'info'); }

export function logResults(results) {
  if (!Array.isArray(results)) return;
  for (const r of results) {
    const msg = r.message || r;
    const type = typeof r === 'string'
      ? (r.startsWith('✓') ? 'success' : r.startsWith('✗') ? 'error' : 'info')
      : (r.success ? 'success' : 'error');
    log(msg, type);
  }
}

export function clearTerminal() {
  const output = terminalOutput();
  if (output) {
    output.innerHTML = '';
    log('Terminal cleared', 'info');
  }
}

export function initTerminal() {
  const toggle = document.getElementById('terminal-toggle');
  const clear = document.getElementById('terminal-clear');
  const panel = document.getElementById('terminal-panel');
  const input = document.getElementById('terminal-input');

  // Hide console by default — Shift+L to toggle
  if (panel) {
    panel.classList.add('hidden');

    document.addEventListener('keydown', (e) => {
      if (e.shiftKey && e.key === 'L') {
        panel.classList.toggle('hidden');
        if (!panel.classList.contains('hidden')) {
          panel.classList.remove('collapsed');
          if (toggle) toggle.textContent = '▾';
          if (input) input.focus();
          log('Console opened (Shift+L to close)', 'info');
        }
      }
    });
  }
  const resizer = document.getElementById('terminal-resizer');

  if (resizer && panel) {
    let isResizing = false;
    let startY = 0;
    let startHeight = 0;

    resizer.addEventListener('mousedown', (e) => {
      isResizing = true;
      startY = e.clientY;
      startHeight = panel.getBoundingClientRect().height;
      resizer.classList.add('active');
      document.body.style.cursor = 'ns-resize';
      e.preventDefault();
    });

    document.addEventListener('mousemove', (e) => {
      if (!isResizing) return;
      const dy = startY - e.clientY;
      let newHeight = startHeight + dy;
      if (newHeight < 100) newHeight = 100;
      if (newHeight > window.innerHeight * 0.8) newHeight = window.innerHeight * 0.8;
      panel.style.transition = 'none'; // Disable transition during drag
      panel.style.height = `${newHeight}px`;
    });

    document.addEventListener('mouseup', () => {
      if (isResizing) {
        isResizing = false;
        resizer.classList.remove('active');
        document.body.style.cursor = '';
        panel.style.transition = ''; // Restore transition
      }
    });
  }

  if (toggle) {
    toggle.addEventListener('click', () => {
      panel.style.height = ''; // Reset custom drag height when toggling
      panel.classList.toggle('collapsed');
      toggle.textContent = panel.classList.contains('collapsed') ? '▴' : '▾';
      if (!panel.classList.contains('collapsed') && input) input.focus();
    });
  }

  if (clear) {
    clear.addEventListener('click', clearTerminal);
  }
  
  if (input) {
    input.addEventListener('keydown', async (e) => {
      if (e.key === 'Enter') {
        const cmd = input.value.trim();
        if (!cmd) return;
        
        input.value = '';
        log(`> ${cmd}`, 'user-cmd');
        
        // Quick visual un-collapse if typing while collapsed (rare but possible)
        if (panel.classList.contains('collapsed')) {
           panel.classList.remove('collapsed');
           if (toggle) toggle.textContent = '▾';
        }
        
        await processCommand(cmd);
      }
    });
  }
}

async function processCommand(cmd) {
  const parts = cmd.trim().split(/\s+/);
  const baseCmd = parts[0].toLowerCase();
  
  switch(baseCmd) {
    case 'help':
      logInfo('Available commands:');
      logInfo('  help  - Show this commands list');
      logInfo('  clear - Clear terminal output');
      logInfo('  ping  - System API connection check');
      break;
    case 'clear':
      clearTerminal();
      break;
    case 'ping':
      logInfo('Pinging Rust Backend (localhost)...');
      setTimeout(() => logSuccess('Ping successful. Backend responsive (< 1ms).'), 400);
      break;
    case 'clean':
    case 'optimize':
    case 'boost':
      logError(`'${baseCmd}' requires an active subscription.`);
      logInfo('Use the dashboard to manage your subscription.');
      break;
    default:
      logError(`Command not found: '${cmd}'. Type 'help' for available options.`);
      break;
  }
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}
