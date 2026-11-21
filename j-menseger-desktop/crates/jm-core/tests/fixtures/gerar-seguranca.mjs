// Gera vetores do número de segurança com o código EXATO do celular
// (web/src/core/util.js, versão 0.4), para o teste do PC comparar.
//
// Uso: node gerar-seguranca.mjs > seguranca-do-mobile.json
// (precisa de @noble/hashes instalado, como no aplicativo de celular)
import { bytesToHex, hexToBytes } from '@noble/hashes/utils.js';
import { sha256 } from '@noble/hashes/sha2.js';

// --- copiado de util.js do celular, sem mudança ---
export function safetyNumber(pkA, pkB) {
  const [x, y] = [pkA, pkB].sort();
  const h = sha256(hexToBytes(x + y));
  const h2 = sha256(h);
  const all = new Uint8Array([...h, ...h2]);
  const groups = [];
  for (let i = 0; i < 12; i++) {
    const v = ((all[i * 5] << 24) >>> 0) + (all[i * 5 + 1] << 16) + (all[i * 5 + 2] << 8) + all[i * 5 + 3];
    groups.push(String(v % 100000).padStart(5, '0'));
  }
  return groups;
}
// --- fim da cópia ---

// Pares fixos (e alguns aleatórios) para cobrir bytes altos no começo,
// que é onde um erro de sinal no deslocamento apareceria.
const pares = [
  ['f'.repeat(64), '0'.repeat(64)],
  ['80'.repeat(32), '7f'.repeat(32)],
  ['ff00'.repeat(16), '00ff'.repeat(16)],
];
for (let i = 0; i < 20; i++) {
  const a = new Uint8Array(32), b = new Uint8Array(32);
  crypto.getRandomValues(a); crypto.getRandomValues(b);
  pares.push([bytesToHex(a), bytesToHex(b)]);
}
console.log(JSON.stringify(pares.map(([a, b]) => ({ a, b, n: safetyNumber(a, b) })), null, 1));
