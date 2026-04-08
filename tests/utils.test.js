import { describe, it, expect } from 'vitest';

// ── Pure helpers extracted from pages (no Tauri dependency) ──────────────────

// From smartboost.js
function scoreColor(score) {
  if (score >= 75) return '#4ade80';
  if (score >= 50) return '#fbbf24';
  return '#f87171';
}

function scoreLabel(score) {
  if (score >= 85) return 'ПК добре оптимізований';
  if (score >= 65) return 'Є простір для покращення';
  if (score >= 40) return 'Потребує оптимізації';
  return 'Критично — оптимізуй зараз';
}

function ramPressureLabel(p) {
  return p === 'high' ? 'Критично' : p === 'medium' ? 'Помірно' : 'Норма';
}

// From gamemode.js
function formatDuration(secs) {
  if (!secs) return '—';
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return m > 0 ? `${m}хв ${s}с` : `${s}с`;
}

// From gamemode.js
function escHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;')
    .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// From account.js
function getDayWord(n) {
  const abs = Math.abs(n);
  if (abs % 10 === 1 && abs % 100 !== 11) return 'день';
  if ([2,3,4].includes(abs % 10) && ![12,13,14].includes(abs % 100)) return 'дні';
  return 'днів';
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('scoreColor', () => {
  it('green for 75+', () => {
    expect(scoreColor(75)).toBe('#4ade80');
    expect(scoreColor(100)).toBe('#4ade80');
    expect(scoreColor(80)).toBe('#4ade80');
  });
  it('yellow for 50-74', () => {
    expect(scoreColor(50)).toBe('#fbbf24');
    expect(scoreColor(60)).toBe('#fbbf24');
    expect(scoreColor(74)).toBe('#fbbf24');
  });
  it('red for below 50', () => {
    expect(scoreColor(49)).toBe('#f87171');
    expect(scoreColor(0)).toBe('#f87171');
  });
});

describe('scoreLabel', () => {
  it('correct label for each range', () => {
    expect(scoreLabel(90)).toBe('ПК добре оптимізований');
    expect(scoreLabel(85)).toBe('ПК добре оптимізований');
    expect(scoreLabel(70)).toBe('Є простір для покращення');
    expect(scoreLabel(65)).toBe('Є простір для покращення');
    expect(scoreLabel(50)).toBe('Потребує оптимізації');
    expect(scoreLabel(40)).toBe('Потребує оптимізації');
    expect(scoreLabel(39)).toBe('Критично — оптимізуй зараз');
    expect(scoreLabel(0)).toBe('Критично — оптимізуй зараз');
  });
});

describe('ramPressureLabel', () => {
  it('maps pressure levels correctly', () => {
    expect(ramPressureLabel('high')).toBe('Критично');
    expect(ramPressureLabel('medium')).toBe('Помірно');
    expect(ramPressureLabel('low')).toBe('Норма');
    expect(ramPressureLabel('unknown')).toBe('Норма');
  });
});

describe('formatDuration', () => {
  it('returns dash for zero/falsy', () => {
    expect(formatDuration(0)).toBe('—');
    expect(formatDuration(null)).toBe('—');
    expect(formatDuration(undefined)).toBe('—');
  });
  it('shows seconds only for < 60s', () => {
    expect(formatDuration(30)).toBe('30с');
    expect(formatDuration(1)).toBe('1с');
    expect(formatDuration(59)).toBe('59с');
  });
  it('shows minutes and seconds for 60s+', () => {
    expect(formatDuration(60)).toBe('1хв 0с');
    expect(formatDuration(90)).toBe('1хв 30с');
    expect(formatDuration(3661)).toBe('61хв 1с');
  });
});

describe('escHtml', () => {
  it('escapes dangerous characters', () => {
    expect(escHtml('<script>')).toBe('&lt;script&gt;');
    expect(escHtml('"hello"')).toBe('&quot;hello&quot;');
    expect(escHtml('a & b')).toBe('a &amp; b');
  });
  it('converts non-string to string', () => {
    expect(escHtml(42)).toBe('42');
    expect(escHtml(null)).toBe('null');
  });
  it('safe strings pass through unchanged', () => {
    expect(escHtml('RustClient.exe')).toBe('RustClient.exe');
  });
});

describe('getDayWord', () => {
  it('1 день', () => {
    expect(getDayWord(1)).toBe('день');
    expect(getDayWord(21)).toBe('день');
    expect(getDayWord(31)).toBe('день');
  });
  it('2-4 дні', () => {
    expect(getDayWord(2)).toBe('дні');
    expect(getDayWord(3)).toBe('дні');
    expect(getDayWord(4)).toBe('дні');
    expect(getDayWord(22)).toBe('дні');
  });
  it('5+ днів', () => {
    expect(getDayWord(5)).toBe('днів');
    expect(getDayWord(11)).toBe('днів');
    expect(getDayWord(12)).toBe('днів');
    expect(getDayWord(20)).toBe('днів');
    expect(getDayWord(30)).toBe('днів');
  });
});
