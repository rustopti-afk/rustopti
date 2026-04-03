// Sound effects module
let audioCtx = null;
let enableBuffer = null;

async function getAudioContext() {
  if (!audioCtx) audioCtx = new (window.AudioContext || window.webkitAudioContext)();
  return audioCtx;
}

async function loadEnableSound() {
  if (enableBuffer) return enableBuffer;
  try {
    const ctx = await getAudioContext();
    const res = await fetch('enable.wav');
    const arr = await res.arrayBuffer();
    enableBuffer = await ctx.decodeAudioData(arr);
    return enableBuffer;
  } catch {
    return null;
  }
}

// Play the enable.wav sound
export async function playEnable() {
  try {
    const ctx = await getAudioContext();
    if (ctx.state === 'suspended') await ctx.resume();
    const buffer = await loadEnableSound();
    if (!buffer) return;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    source.connect(ctx.destination);
    source.start(0);
  } catch {
    // Ignore audio errors silently
  }
}
