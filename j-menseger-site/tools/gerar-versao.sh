#!/bin/bash
# Gera o versao.json que os apps leem para se atualizar sozinhos.
#   uso: tools/gerar-versao.sh <versão-pc> <versão-android> <código-android> "novidades"
# Os arquivos precisam estar em downloads/ com os nomes:
#   J-Menseger-<versão-pc>-instalador.exe   e   J-Menseger-<versão-android>.apk
set -euo pipefail
cd "$(dirname "$0")/.."
PC="$1"; AND="$2"; COD="$3"; NOV="${4:-}"
BASE="https://j-menseger-site.vercel.app/downloads"
EXE="downloads/J-Menseger-$PC-instalador.exe"
APK="downloads/J-Menseger-$AND.apk"
for f in "$EXE" "$APK"; do [ -f "$f" ] || { echo "faltando: $f"; exit 1; }; done
python3 - "$PC" "$AND" "$COD" "$NOV" "$BASE" "$EXE" "$APK" <<'PY'
import sys, json, hashlib, os
pc, an, cod, nov, base, exe, apk = sys.argv[1:]
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
d = {
  "windows": {"versao": pc, "url": f"{base}/{os.path.basename(exe)}", "sha256": sha(exe), "tamanho": os.path.getsize(exe), "novidades": nov},
  "android": {"versao": an, "codigo": int(cod), "url": f"{base}/{os.path.basename(apk)}", "sha256": sha(apk), "tamanho": os.path.getsize(apk), "novidades": nov},
}
# Linux e Mac: ainda sem atualização automática, mas o app e o site já
# sabem onde está a versão mais nova.
for chave, nomes in (("linux", [f"J-Menseger-{pc}-linux-amd64.deb", f"J-Menseger-{pc}-linux-x86_64.tar.gz"]),
                     ("mac", [f"J-Menseger-{pc}-macos-arm64.dmg", f"J-Menseger-{pc}-macos-x86_64.dmg"])):
    arqs = [f"downloads/{n}" for n in nomes if os.path.exists(f"downloads/{n}")]
    if arqs:
        d[chave] = {"versao": pc, "arquivos": [{"url": f"{base}/{os.path.basename(a)}", "sha256": sha(a), "tamanho": os.path.getsize(a)} for a in arqs], "novidades": nov}
open("versao.json", "w").write(json.dumps(d, ensure_ascii=False, indent=2) + "\n")
print(open("versao.json").read())
PY
