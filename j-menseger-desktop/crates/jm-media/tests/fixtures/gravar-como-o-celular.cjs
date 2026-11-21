// Grava 2,5 s com o MediaRecorder do Chromium, do mesmo jeito que o celular
// (media.js: audio/webm;codecs=opus a 24 kbps, pedaços de 250 ms).
const { chromium } = require('playwright');
(async () => {
  const b = await chromium.launch({ args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream'] });
  const p = await b.newPage();
  await p.route('http://localhost/**', r => r.fulfill({ contentType: 'text/html', body: '<html></html>' }));
  await p.goto('http://localhost/');
  const out = await p.evaluate(async () => {
    const s = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true } });
    const mime = ['audio/webm;codecs=opus', 'audio/ogg;codecs=opus', 'audio/webm'].find(t => MediaRecorder.isTypeSupported(t));
    const r = new MediaRecorder(s, { mimeType: mime, audioBitsPerSecond: 24000 });
    const parts = [];
    r.ondataavailable = e => e.data.size && parts.push(e.data);
    r.start(250);
    await new Promise(res => setTimeout(res, 2500));
    await new Promise(res => { r.onstop = res; r.stop(); });
    const buf = new Uint8Array(await new Blob(parts, { type: mime }).arrayBuffer());
    let bin = ''; for (const x of buf) bin += String.fromCharCode(x);
    return { mime: r.mimeType, b64: btoa(bin) };
  });
  require('fs').writeFileSync('celular.webm', Buffer.from(out.b64, 'base64'));
  console.log(out.mime, Buffer.from(out.b64, 'base64').length, 'bytes');
  await b.close();
})().catch(e => { console.error(e); process.exit(1); });
