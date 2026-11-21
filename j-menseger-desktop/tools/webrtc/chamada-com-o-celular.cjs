// Chamada real entre o PC (Rust) e o WebRTC do celular (Chromium, que é o
// motor da WebView do Android), com a classe Peer copiada de calls.js.
//
// Uso: NODE_PATH=$(npm root -g) node chamada-com-o-celular.cjs <binário ponte_webrtc> <celular-oferta|pc-oferta>
const { chromium } = require('playwright');
const { spawn } = require('child_process');
const readline = require('readline');

const [bin, cenario] = process.argv.slice(2);

// --- código da página: a classe Peer do celular (calls.js), sem mudanças de lógica ---
const PAGINA = `
<script>
const AUDIO_CONSTRAINTS = { echoCancellation: true, noiseSuppression: true, autoGainControl: true };
class Peer {
  constructor(h) {
    this.h = h;
    this.pc = new RTCPeerConnection({ iceServers: [], iceTransportPolicy: 'all', bundlePolicy: 'max-bundle' });
    this.dc = this.pc.createDataChannel('jm', { negotiated: true, id: 0 });
    this.dc.onmessage = ev => { try { h.onData(JSON.parse(ev.data), typeof ev.data); } catch (e) { h.onBad(typeof ev.data); } };
    this.dc.onopen = () => h.onOpen && h.onOpen();
    this.remote = new MediaStream();
    this.pc.ontrack = ev => { if (!this.remote.getTracks().includes(ev.track)) this.remote.addTrack(ev.track); };
    this.pc.onconnectionstatechange = () => h.onState(this.pc.connectionState);
  }
  async gather() {
    if (this.pc.iceGatheringState === 'complete') return;
    await new Promise(res => {
      const t = setTimeout(res, 2600);
      this.pc.addEventListener('icegatheringstatechange', () => { if (this.pc.iceGatheringState === 'complete') { clearTimeout(t); res(); } });
    });
  }
  async makeOffer(stream) {
    const at = stream.getAudioTracks()[0], vt = stream.getVideoTracks()[0];
    this.aT = this.pc.addTransceiver(at || 'audio', { direction: 'sendrecv', streams: [stream] });
    this.vT = this.pc.addTransceiver(vt || 'video', { direction: 'sendrecv', streams: [stream] });
    await this.pc.setLocalDescription(await this.pc.createOffer());
    await this.gather();
    return this.pc.localDescription.sdp;
  }
  async acceptOffer(sdp, stream) {
    await this.pc.setRemoteDescription({ type: 'offer', sdp });
    const tr = this.pc.getTransceivers();
    this.aT = tr.find(t => t.receiver.track && t.receiver.track.kind === 'audio');
    this.vT = tr.find(t => t.receiver.track && t.receiver.track.kind === 'video');
    const at = stream.getAudioTracks()[0];
    if (this.aT) { await this.aT.sender.replaceTrack(at || null); this.aT.direction = 'sendrecv'; }
    if (this.vT) { this.vT.direction = 'sendrecv'; }
    await this.pc.setLocalDescription(await this.pc.createAnswer());
    await this.gather();
    return this.pc.localDescription.sdp;
  }
  async acceptAnswer(sdp) { await this.pc.setRemoteDescription({ type: 'answer', sdp }); }
  send(o) { try { if (this.dc.readyState === 'open') this.dc.send(JSON.stringify(o)); } catch (e) {} }
}
window.T = { msgs: [], tipos: [], ruins: [], estado: '' };
window.criar = async (video) => {
  // getMedia() de calls.js: câmera 640x480 a 24 qps numa chamada de vídeo.
  window.stream = await navigator.mediaDevices.getUserMedia({ audio: AUDIO_CONSTRAINTS, video: video ? { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 }, frameRate: { ideal: 24, max: 30 } } : false });
  window.peer = new Peer({
    onData: (d, tipo) => { T.msgs.push(d); T.tipos.push(tipo); },
    onBad: tipo => T.ruins.push(tipo),
    onOpen: () => { peer.send({ t: 'cam', on: false }); peer.send({ t: 'mic', on: true }); },
    onState: s => { T.estado = s; }
  });
};
window.estatisticas = async () => {
  const r = { audio: 0, video: 0, quadros: 0, audioEnviado: 0 };
  (await peer.pc.getStats()).forEach(s => {
    if (s.type === 'inbound-rtp' && s.kind === 'audio') r.audio = s.packetsReceived;
    if (s.type === 'inbound-rtp' && s.kind === 'video') { r.video = s.packetsReceived; r.quadros = s.framesDecoded || 0; r.codec = s.codecId; }
    if (s.type === 'outbound-rtp' && s.kind === 'audio') r.audioEnviado = s.packetsSent;
    if (s.type === 'outbound-rtp' && s.kind === 'video') { r.videoEnviado = s.packetsSent; r.videoCodificado = s.framesEncoded; r.limite = s.qualityLimitationReason; }
  });
  return r;
};
</script>`;

(async () => {
  const pc = spawn(bin, [cenario === 'pc-oferta' ? 'oferta' : 'resposta'], { stdio: ['pipe', 'pipe', 'inherit'] });
  const linhas = readline.createInterface({ input: pc.stdout });
  const eventos = [];
  let esperando = null;
  linhas.on('line', l => { const v = JSON.parse(l); if (esperando && (v.answer || v.offer)) { const f = esperando; esperando = null; f(v); } else eventos.push(v); });
  // A oferta/resposta do PC pode chegar antes de começarmos a esperar.
  const proxima = () => new Promise(res => {
    const i = eventos.findIndex(v => v.answer || v.offer);
    if (i >= 0) return res(eventos.splice(i, 1)[0]);
    esperando = res;
  });

  const b = await chromium.launch({ args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream'] });
  const p = await b.newPage();
  await p.route('http://localhost/**', r => r.fulfill({ contentType: 'text/html', body: PAGINA }));
  await p.goto('http://localhost/');
  await p.evaluate(v => criar(v), cenario === 'celular-video');

  if (cenario === 'pc-oferta') {
    const { offer } = await proxima();
    const sdp = await p.evaluate(o => peer.acceptOffer(o, stream), offer);
    pc.stdin.write(JSON.stringify({ sdp }) + '\n');
  } else {
    const sdp = await p.evaluate(() => peer.makeOffer(stream));
    const espera = proxima();
    pc.stdin.write(JSON.stringify({ sdp }) + '\n');
    const { answer } = await espera; require('fs').writeFileSync('/tmp/claude-0/-home-user-Portifolio/00edfc2b-a394-5ac2-aa54-d0982fa76258/scratchpad/sdp-pc.txt', answer);
    await p.evaluate(a => peer.acceptAnswer(a), answer);
  }

  // O celular toca um som e desliga o microfone, pelo canal de dados.
  await p.waitForFunction(() => T.estado === 'connected', null, { timeout: 15000 }).catch(() => {});
  await p.waitForTimeout(1500);
  await p.evaluate(() => { peer.send({ t: 'snd', id: 'aplausos', n: 'Aplausos', e: '👏', d: 3.7 }); peer.send({ t: 'mic', on: false }); });
  await p.waitForTimeout(6000);
  const st = await p.evaluate(() => estatisticas());
  const T = await p.evaluate(() => window.T);
  await new Promise(res => pc.on('exit', res));
  const fim = eventos.find(e => e.ev === 'fim') || {};
  const dados = eventos.filter(e => e.ev === 'data').map(e => JSON.parse(e.text));
  console.log(JSON.stringify({
    cenario,
    navegador_conectou: T.estado,
    pc_conectou: eventos.some(e => e.ev === 'connected'),
    pc_recebeu_audio_pacotes: fim.audio_pacotes, pc_video_pacotes: fim.video_pacotes, pc_video_decodificado: fim.video_decodificados, pc_video_tamanho: fim.video_tamanho, pc_nivel_audio: Number((fim.audio_nivel || 0).toFixed(3)),
    navegador_recebeu_audio: st.audio, navegador_video_pacotes: st.video, navegador_quadros_video: st.quadros, navegador_codec_video: st.codec, sdp_pc: (eventos.find(e => e.answer) || {}).answer, navegador_enviou_audio: st.audioEnviado, navegador_enviou_video: st.videoEnviado, navegador_codificou: st.videoCodificado,
    navegador_recebeu_do_pc: T.msgs, tipos_das_mensagens: [...new Set(T.tipos)], mensagens_descartadas: T.ruins.length,
    pc_recebeu_do_navegador: dados
  }, null, 1));
  await b.close();
})().catch(e => { console.error(e); process.exit(1); });
