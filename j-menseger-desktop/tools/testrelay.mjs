// Relay Nostr mínimo para testes locais. Imita as regras dos relays públicos (strfry):
// evento até 64 KB, efêmeros (20000–29999) só repassados ao vivo e recusados se tiverem
// mais de 60 s, expiração (NIP-40) respeitada, assinatura verificada.
// Com --auth exige NIP-42 para ler e gravar.
//
// Uso: node tools/testrelay.mjs 7781 [--auth]
import { WebSocketServer } from 'ws';
import { verifyEvent } from 'nostr-tools/pure';
import { randomBytes } from 'crypto';

const args = process.argv.slice(2);
const port = parseInt(args.find((a) => /^\d+$/.test(a)) || '7777', 10);
const requireAuth = args.includes('--auth');
const MAX = 65536;
const store = [];
const now = () => Math.floor(Date.now() / 1000);

function matches(f, ev) {
  if (f.ids && !f.ids.includes(ev.id)) return false;
  if (f.kinds && !f.kinds.includes(ev.kind)) return false;
  if (f.authors && !f.authors.includes(ev.pubkey)) return false;
  if (f.since && ev.created_at < f.since) return false;
  if (f.until && ev.created_at > f.until) return false;
  for (const [k, vals] of Object.entries(f)) {
    if (!k.startsWith('#')) continue;
    const tag = k.slice(1);
    const have = ev.tags.filter((t) => t[0] === tag).map((t) => t[1]);
    if (!have.some((v) => vals.includes(v))) return false;
  }
  return true;
}

const expired = (ev) => {
  const t = ev.tags.find((x) => x[0] === 'expiration');
  return t && parseInt(t[1], 10) <= now();
};

const wss = new WebSocketServer({ port, maxPayload: MAX });
console.log(`relay de teste em ws://127.0.0.1:${port}${requireAuth ? ' (exige AUTH)' : ''}`);

wss.on('connection', (ws) => {
  const subs = new Map();
  const challenge = randomBytes(16).toString('hex');
  let authed = !requireAuth;
  const send = (m) => ws.send(JSON.stringify(m));
  if (requireAuth) send(['AUTH', challenge]);

  ws.on('message', (raw) => {
    if (raw.length > MAX) return send(['NOTICE', 'evento grande demais']);
    let m;
    try {
      m = JSON.parse(raw);
    } catch {
      return send(['NOTICE', 'json inválido']);
    }
    const [tipo] = m;

    if (tipo === 'AUTH') {
      const ev = m[1];
      const ok =
        ev &&
        ev.kind === 22242 &&
        verifyEvent(ev) &&
        Math.abs(now() - ev.created_at) < 600 &&
        ev.tags.some((t) => t[0] === 'challenge' && t[1] === challenge);
      authed = !!ok;
      return send(['OK', ev?.id ?? '', ok, ok ? '' : 'auth: desafio não confere']);
    }

    if (tipo === 'EVENT') {
      const ev = m[1];
      if (!authed) return send(['OK', ev?.id ?? '', false, 'auth-required: autentique primeiro']);
      if (!ev || !verifyEvent(ev)) return send(['OK', ev?.id ?? '', false, 'invalid: assinatura']);
      if (JSON.stringify(ev).length > MAX) return send(['OK', ev.id, false, 'invalid: grande demais']);

      const efemero = ev.kind >= 20000 && ev.kind < 30000;
      if (efemero && now() - ev.created_at > 60) {
        return send(['OK', ev.id, false, 'invalid: efêmero antigo demais']);
      }
      if (expired(ev)) return send(['OK', ev.id, false, 'invalid: já expirado']);

      if (!efemero) {
        if (store.some((e) => e.id === ev.id)) {
          return send(['OK', ev.id, true, 'duplicate: já tinha']);
        }
        store.push(ev);
      }
      send(['OK', ev.id, true, '']);

      // Repassa a quem está ouvindo, aqui e nas outras conexões.
      for (const c of wss.clients) {
        if (c.readyState !== 1) continue;
        const cs = c === ws ? subs : c.__subs;
        if (!cs) continue;
        for (const [id, filtros] of cs) {
          if (filtros.some((f) => matches(f, ev))) {
            c.send(JSON.stringify(['EVENT', id, ev]));
          }
        }
      }
      return;
    }

    if (tipo === 'REQ') {
      const [, id, ...filtros] = m;
      if (!authed) return send(['CLOSED', id, 'auth-required: autentique primeiro']);
      subs.set(id, filtros);
      ws.__subs = subs;
      for (const ev of store) {
        if (expired(ev)) continue;
        if (filtros.some((f) => matches(f, ev))) send(['EVENT', id, ev]);
      }
      return send(['EOSE', id]);
    }

    if (tipo === 'CLOSE') {
      subs.delete(m[1]);
    }
  });

  ws.__subs = subs;
});
