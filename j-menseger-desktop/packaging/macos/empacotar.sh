#!/bin/bash
# Monta o J-Menseger.app e o .dmg. Roda num Mac (ou no GitHub Actions).
#   uso: packaging/macos/empacotar.sh <binário> <arquitetura: arm64|x86_64>
# Antes:  brew install libvpx pkg-config
#         JM_VPX_STATIC=1 cargo build --release -p jm-app
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$1"; ARQ="$2"
VERSAO="$(sed -n 's/^version = "\(.*\)"/\1/p' "$RAIZ/Cargo.toml" | head -1)"
SAIDA="$RAIZ/packaging/macos/dist"
TRAB="$(mktemp -d)"
APP="$TRAB/J-Menseger.app"

mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$SAIDA"
install -m755 "$BIN" "$APP/Contents/MacOS/j-menseger"
sed "s/VERSAO/$VERSAO/g" "$RAIZ/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"

# Ícone: o PNG vira .icns (todos os tamanhos que o Finder pede).
ICONES="$TRAB/icone.iconset"; mkdir -p "$ICONES"
for t in 16 32 64 128 256 512; do
  sips -z $t $t "$RAIZ/crates/jm-app/recursos/icone.png" --out "$ICONES/icon_${t}x${t}.png" >/dev/null
  d=$((t*2)); sips -z $d $d "$RAIZ/crates/jm-app/recursos/icone.png" --out "$ICONES/icon_${t}x${t}@2x.png" >/dev/null
done
iconutil -c icns "$ICONES" -o "$APP/Contents/Resources/icone.icns"

# Nada de bibliotecas do Homebrew: no Mac de quem baixa elas não existem.
if otool -L "$APP/Contents/MacOS/j-menseger" | grep -E '/opt/homebrew|/usr/local/(lib|opt|Cellar)'; then
  echo "O programa depende de bibliotecas do Homebrew (veja acima)."; exit 1
fi

# Assinatura "ad hoc": sem ela o macOS nem abre programas para Apple Silicon.
codesign --force --deep --sign - "$APP"

# .dmg com o atalho para Aplicativos, para arrastar.
PASTA="$TRAB/dmg"; mkdir -p "$PASTA"
cp -R "$APP" "$PASTA/"
ln -s /Applications "$PASTA/Applications"
DMG="$SAIDA/J-Menseger-$VERSAO-macos-$ARQ.dmg"
rm -f "$DMG"
hdiutil create -volname "J-Menseger" -srcfolder "$PASTA" -ov -format UDZO "$DMG" >/dev/null
echo "$DMG"
