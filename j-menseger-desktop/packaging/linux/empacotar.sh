#!/bin/bash
# Gera o .deb (Ubuntu, Debian, Mint, Pop!_OS…) e o .tar.gz (qualquer Linux).
#   uso: packaging/linux/empacotar.sh [binário]
# Antes (num sistema com glibc antiga, ex. Ubuntu 22.04, para rodar em mais lugares):
#   sudo apt install libvpx-dev libasound2-dev libgbm-dev libxcb1-dev
#   JM_VPX_STATIC=1 cargo build --release -p jm-app
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${1:-$RAIZ/target/release/j-menseger}"
VERSAO="$(sed -n 's/^version = "\(.*\)"/\1/p' "$RAIZ/Cargo.toml" | head -1)"
SAIDA="$RAIZ/packaging/linux/dist"
TRAB="$(mktemp -d)"
mkdir -p "$SAIDA"

# glibc mínima que o programa pede (as versões que o carregador exige).
GLIBC="$(readelf -V "$BIN" | sed -n 's/.*Name: GLIBC_\([0-9.]*\) *Flags: none.*/\1/p' | sort -uV | tail -1)"
echo "glibc mínima: $GLIBC"

# --- .deb ---
D="$TRAB/deb"
install -Dm755 "$BIN" "$D/usr/bin/j-menseger"
install -Dm644 "$RAIZ/crates/jm-app/recursos/icone.png" "$D/usr/share/icons/hicolor/128x128/apps/j-menseger.png"
install -Dm644 "$RAIZ/packaging/linux/j-menseger.desktop" "$D/usr/share/applications/j-menseger.desktop"
mkdir -p "$D/DEBIAN"
cat > "$D/DEBIAN/control" <<CTRL
Package: j-menseger
Version: $VERSAO
Section: net
Priority: optional
Architecture: amd64
Maintainer: J-Menseger <noreply@j-menseger.app>
Depends: libc6 (>= $GLIBC), libasound2 | libasound2t64, libgbm1, libxcb1
Recommends: libxkbcommon0, libx11-6, libwayland-client0, libvulkan1 | libgl1, xdg-desktop-portal
Installed-Size: $(du -sk "$D/usr" | cut -f1)
Description: Conversas cifradas de ponta a ponta
 Mensagens, chamadas de voz e vídeo, salas de voz e transmissão de tela,
 cifradas de ponta a ponta sobre o protocolo Nostr.
CTRL
DEB="$SAIDA/J-Menseger-$VERSAO-linux-amd64.deb"
dpkg-deb --root-owner-group --build "$D" "$DEB" >/dev/null

# --- .tar.gz ---
N="J-Menseger-$VERSAO-linux-x86_64"
T="$TRAB/$N"
install -Dm755 "$BIN" "$T/j-menseger"
install -Dm644 "$RAIZ/crates/jm-app/recursos/icone.png" "$T/icone.png"
install -Dm644 "$RAIZ/packaging/linux/j-menseger.desktop" "$T/j-menseger.desktop"
cat > "$T/instalar.sh" <<'INST'
#!/bin/bash
# Instala o J-Menseger só para você (sem sudo), em ~/.local.
set -euo pipefail
AQUI="$(cd "$(dirname "$0")" && pwd)"
DESTINO="${PREFIX:-$HOME/.local}"
install -Dm755 "$AQUI/j-menseger"         "$DESTINO/bin/j-menseger"
install -Dm644 "$AQUI/icone.png"          "$DESTINO/share/icons/hicolor/128x128/apps/j-menseger.png"
mkdir -p "$DESTINO/share/applications"
sed "s|^Exec=.*|Exec=$DESTINO/bin/j-menseger|" "$AQUI/j-menseger.desktop" > "$DESTINO/share/applications/j-menseger.desktop"
update-desktop-database "$DESTINO/share/applications" 2>/dev/null || true
echo "Pronto: o J-Menseger está no menu de aplicativos."
echo "(programa em $DESTINO/bin/j-menseger)"
INST
chmod 755 "$T/instalar.sh"
TGZ="$SAIDA/$N.tar.gz"
tar -C "$TRAB" --owner=0 --group=0 -czf "$TGZ" "$N"
rm -rf "$TRAB"
ls -la "$DEB" "$TGZ"
