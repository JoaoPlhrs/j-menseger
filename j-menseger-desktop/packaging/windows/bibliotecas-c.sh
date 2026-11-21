#!/bin/bash
# Compila, a partir do Linux, as duas bibliotecas em C de que o aplicativo
# depende (libvpx para vídeo e libopus para voz) na versão para Windows, e
# monta um diretório que o pkg-config consegue ler.
#
# É o que permite gerar o executável do Windows sem ter um Windows à mão.
# Nenhuma linha de Rust muda por causa disto: o build.rs continua perguntando
# ao pkg-config onde estão as bibliotecas, e aqui ele passa a responder com as
# versões de Windows em vez das de Linux.
#
# Uso:
#   packaging/windows/bibliotecas-c.sh            # compila em target/win-c
#   DESTINO=/outro/lugar packaging/windows/bibliotecas-c.sh
#
# Depois:
#   export PKG_CONFIG_ALLOW_CROSS=1
#   export PKG_CONFIG_LIBDIR="$PWD/target/win-c/lib/pkgconfig"
#   cargo build --release -p jm-app --target x86_64-pc-windows-gnu
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
DESTINO="${DESTINO:-$RAIZ/target/win-c}"
TRABALHO="$DESTINO/fontes"
CROSS=x86_64-w64-mingw32-

for f in "${CROSS}gcc" nasm; do
  command -v "$f" >/dev/null || {
    echo "Falta '$f'. No Debian/Ubuntu:"
    echo "  apt install gcc-mingw-w64-x86-64 nasm"
    exit 1
  }
done

mkdir -p "$TRABALHO"

# As fontes vêm do próprio repositório da distribuição: é a mesma versão da
# biblioteca que o Linux daqui usa, então os dois sistemas falam o mesmo
# formato de vídeo, e dispensa baixar de sites que podem sair do ar.
baixar() {
  local pacote="$1" pasta="$2"
  [ -d "$TRABALHO/$pasta" ] && return 0
  ( cd "$TRABALHO" && apt-get source "$pacote" ) || {
    echo "Não foi possível baixar as fontes de '$pacote'."
    echo "Habilite os repositórios de código-fonte (deb-src) e rode apt-get update."
    exit 1
  }
}

if [ ! -f "$DESTINO/lib/libvpx.a" ]; then
  echo "### libvpx para Windows"
  baixar libvpx 'libvpx-*'
  cd "$TRABALHO"/libvpx-*/
  make distclean >/dev/null 2>&1 || true
  CROSS=$CROSS ./configure --target=x86_64-win64-gcc --prefix="$DESTINO" \
    --as=nasm --enable-vp8 --enable-vp9 --enable-pic --enable-static \
    --disable-shared --disable-examples --disable-tools --disable-docs \
    --disable-unit-tests --enable-runtime-cpu-detect
  make -j"$(nproc)"
  make install
fi

if [ ! -f "$DESTINO/lib/libopus.a" ]; then
  echo "### libopus para Windows"
  baixar opus 'opus-*'
  cd "$TRABALHO"/opus-*/
  make distclean >/dev/null 2>&1 || true
  ./configure --host=x86_64-w64-mingw32 --prefix="$DESTINO" \
    --enable-static --disable-shared --disable-doc --disable-extra-programs
  make -j"$(nproc)"
  make install
fi

echo
echo "Pronto em $DESTINO"
ls -la "$DESTINO/lib"/*.a
echo
echo "Agora:"
echo "  export PKG_CONFIG_ALLOW_CROSS=1"
echo "  export PKG_CONFIG_LIBDIR=\"$DESTINO/lib/pkgconfig\""
echo "  cargo build --release -p jm-app --target x86_64-pc-windows-gnu"
