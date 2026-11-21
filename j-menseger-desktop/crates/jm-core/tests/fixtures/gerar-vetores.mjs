import { generateMnemonic, validateMnemonic } from '@scure/bip39';
import { wordlist as PT } from '@scure/bip39/wordlists/portuguese.js';
import { privateKeyFromSeedWords } from 'nostr-tools/nip06';
import { getPublicKey, finalizeEvent, getEventHash } from 'nostr-tools/pure';
import { npubEncode, nsecEncode } from 'nostr-tools/nip19';
import * as nip44 from 'nostr-tools/nip44';
const bytesToHex = (u) => Array.from(u).map(b => b.toString(16).padStart(2,'0')).join('');

const out = { mnemonics: [], nip44: [], wrap: null };

// 1. derivação das 12 palavras em português
for (let i = 0; i < 5; i++) {
  const words = generateMnemonic(PT, 128);
  const sk = privateKeyFromSeedWords(words);
  out.mnemonics.push({ words, sk: bytesToHex(sk), pk: getPublicKey(sk), npub: npubEncode(getPublicKey(sk)), nsec: nsecEncode(sk) });
}
// frase fixa, para o teste não depender de sorteio
{
  const words = 'abacate abaixo abalar abater abduzir abelha aberto abismo abotoar abranger abreviar abrigar';
  if (validateMnemonic(words, PT)) {
    const sk = privateKeyFromSeedWords(words);
    out.mnemonics.push({ words, sk: bytesToHex(sk), pk: getPublicKey(sk), npub: npubEncode(getPublicKey(sk)), nsec: nsecEncode(sk), fixa: true });
  } else out.fraseFixaInvalida = true;
}

// 2. NIP-44 v2: o Rust tem que decifrar isto
const a = privateKeyFromSeedWords(out.mnemonics[0].words);
const b = privateKeyFromSeedWords(out.mnemonics[1].words);
const ck = nip44.v2.utils.getConversationKey(a, getPublicKey(b));
for (const texto of ['Oi Beto! 👋', 'acentuação e emoji 🎉', 'x'.repeat(500)]) {
  out.nip44.push({ de_sk: bytesToHex(a), para_pk: getPublicKey(b), texto, cifrado: nip44.v2.encrypt(texto, ck) });
}

// 3. envelope NIP-59 completo: selo + envelope, como o app mobile monta
const rumor = { kind: 14, content: 'mensagem dentro do envelope', tags: [['p', getPublicKey(b)]], created_at: Math.floor(Date.now()/1000), pubkey: getPublicKey(a) };

rumor.id = getEventHash(rumor);
const seal = finalizeEvent({ kind: 13, content: nip44.v2.encrypt(JSON.stringify(rumor), nip44.v2.utils.getConversationKey(a, getPublicKey(b))), created_at: Math.floor(Date.now()/1000) - 3600, tags: [] }, a);
const throwaway = privateKeyFromSeedWords(out.mnemonics[2].words);
const wrap = finalizeEvent({ kind: 1059, content: nip44.v2.encrypt(JSON.stringify(seal), nip44.v2.utils.getConversationKey(throwaway, getPublicKey(b))), created_at: Math.floor(Date.now()/1000) - 7200, tags: [['p', getPublicKey(b)]] }, throwaway);
out.wrap = { destinatario_sk: bytesToHex(b), rumor_esperado: rumor, evento: wrap };

console.log(JSON.stringify(out, null, 1));
