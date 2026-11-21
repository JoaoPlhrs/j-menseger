#!/bin/bash
# Instala o J-Menseger neste computador Linux, para o usuário atual.
# Compile antes com:  cargo build --release -p jm-app
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$RAIZ/target/release/j-menseger"

if [ ! -x "$BIN" ]; then
  echo "O programa ainda não foi compilado."
  echo "Rode:  cargo build --release -p jm-app"
  exit 1
fi

DESTINO="${PREFIX:-$HOME/.local}"
install -Dm755 "$BIN"                                   "$DESTINO/bin/j-menseger"
install -Dm644 "$RAIZ/crates/jm-app/recursos/icone.png" "$DESTINO/share/icons/hicolor/128x128/apps/j-menseger.png"
install -Dm644 "$RAIZ/packaging/linux/j-menseger.desktop" "$DESTINO/share/applications/j-menseger.desktop"
update-desktop-database "$DESTINO/share/applications" 2>/dev/null || true

echo "Instalado em $DESTINO/bin/j-menseger"
case ":$PATH:" in
  *":$DESTINO/bin:"*) ;;
  *) echo "Atenção: $DESTINO/bin não está no seu PATH." ;;
esac
