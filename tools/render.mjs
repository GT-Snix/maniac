// Turn a baked maniac player (scene.html) into an MP4 using headless Chrome and ffmpeg.
// No npm packages: talks to Chrome over the DevTools protocol with Node's built-in WebSocket.
//
//   node tools/render.mjs scene.html scene.mp4 [cellPx=12] [bloom=0.4]
//
// Needs Node 22+, Google Chrome and ffmpeg on PATH (or CHROME=/path/to/chrome).
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const [input, output, cellArg = '12', bloomArg = '0.4'] = process.argv.slice(2);
if (!input || !output) { console.error('usage: node tools/render.mjs scene.html scene.mp4 [cellPx] [bloom]'); process.exit(1); }
const CHROME = process.env.CHROME || (process.platform === 'darwin'
  ? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' : 'google-chrome');
const port = 9400 + Math.floor(Math.random() * 500);
const profile = mkdtempSync(join(tmpdir(), 'maniac-chrome-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
  '--allow-file-access-from-files', '--no-first-run', 'about:blank'], { stdio: 'ignore' });
const sleep = ms => new Promise(r => setTimeout(r, ms));

try {
  let tgt;
  for (let i = 0; i < 50 && !tgt; i++) {
    try { tgt = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json(); } catch { await sleep(200); }
  }
  const ws = new WebSocket(tgt.webSocketDebuggerUrl);
  await new Promise(r => ws.onopen = r);
  let id = 0; const pending = new Map();
  ws.onmessage = e => { const m = JSON.parse(e.data); pending.get(m.id)?.(m); pending.delete(m.id); };
  const cmd = (method, params = {}) => new Promise(r => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const ev = async expr => {
    const r = await cmd('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
    if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.text);
    return r.result.result.value;
  };
  await cmd('Page.navigate', { url: 'file://' + resolve(input) });
  for (let i = 0; i < 150 && !(await ev(`typeof M !== 'undefined' && frame >= 0`).catch(() => false)); i++) await sleep(200);

  // Fixed cell size (12 px wide → 160 cols = 1920 px), bloom composited into each frame.
  const [w, h, n, fps] = await ev(`(async () => {
    await document.fonts.load('20px "JetBrains Mono"');
    playing = false; cw = ${+cellArg}; chh = cw * 2; cv.width = M.cols * cw; cv.height = M.rows * chh;
    ctx.font = (cw / 0.6) + 'px "JetBrains Mono", monospace'; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    const V = document.createElement('canvas'), S = document.createElement('canvas');
    V.width = cv.width; V.height = cv.height; S.width = cv.width / 4; S.height = cv.height / 4;
    const vx = V.getContext('2d'), sx = S.getContext('2d');
    window.shot = f => {
      if (f < frame) seek(f); else goto(f);
      vx.globalCompositeOperation = 'source-over'; vx.globalAlpha = 1; vx.filter = 'none'; vx.drawImage(cv, 0, 0);
      sx.drawImage(cv, 0, 0, S.width, S.height);
      vx.globalCompositeOperation = 'screen'; vx.globalAlpha = ${+bloomArg}; vx.filter = 'blur(9px) saturate(1.4)';
      vx.drawImage(S, 0, 0, V.width, V.height);
      return V.toDataURL('image/jpeg', 0.95).slice(23);
    };
    seek(0); return [V.width, V.height, M.n, M.fps];
  })()`);

  const ff = spawn('ffmpeg', ['-loglevel', 'error', '-y', '-f', 'image2pipe', '-framerate', String(fps), '-c:v', 'mjpeg', '-i', '-',
    '-c:v', 'libx264', '-preset', 'slow', '-crf', '20', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', output], { stdio: ['pipe', 'inherit', 'inherit'] });
  const t0 = Date.now();
  for (let f = 0; f < n; f++) {
    const buf = Buffer.from(await ev(`shot(${f})`), 'base64');
    if (!ff.stdin.write(buf)) await new Promise(r => ff.stdin.once('drain', r));
    if (f % 150 === 0) process.stderr.write(`\r${f}/${n} frames`);
  }
  ff.stdin.end();
  await new Promise(r => ff.on('close', r));
  console.error(`\rwrote ${output}: ${n} frames, ${w}x${h} @ ${fps} fps in ${((Date.now() - t0) / 1000).toFixed(0)}s`);
  ws.close();
} finally {
  chrome.kill();
  await sleep(300);
  rmSync(profile, { recursive: true, force: true });
}
