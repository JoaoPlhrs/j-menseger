# J-Menseger para computador

Porte do aplicativo J-Menseger de celular para uma aplicação **nativa de
computador**, escrita em Rust. O celular e o PC são dois clientes do mesmo
aplicativo: mesma conta, mesmo protocolo, mesmos servidores.

Não há HTML, CSS, WebView, Electron, Tauri nem Chromium em nenhuma camada. A
interface é desenhada pela GPU (ou por software, quando não há GPU).

- **Decisões técnicas e porquês:** [`ARQUITETURA.md`](ARQUITETURA.md)
- **Protocolo e compatibilidade com o celular:** testes em
  `crates/jm-core/tests/compatibilidade_com_o_mobile.rs`

## Situação de cada parte

O computador tem os mesmos recursos do celular. A comparação item por item,
com o que foi executado para conferir cada um, está em
[`PARIDADE.md`](PARIDADE.md).

| Área | Situação | Como foi verificado |
|---|---|---|
| Conta, 12 palavras, PIN e tela de bloqueio | **Pronto** | Vetores das bibliotecas do celular; fluxo de criação e bloqueio percorrido na interface |
| Mensagens (respostas, reações, apagar, temporárias, recibos) | **Pronto** | Dois aplicativos conversando por servidor local, conferidos tela a tela |
| Fotos, mensagens de voz e sons nas conversas | **Pronto** | Enviados e recebidos entre os aplicativos; formatos do celular nos testes |
| Contatos, pedidos, bloqueados, número de segurança | **Pronto** | Pedido de uma terceira conta aceito pela interface |
| Grupos (canais, admins, convites, foto) | **Pronto** | Grupo criado, convite aceito, conversa no canal |
| Salas de voz e chamadas 1:1 | **Pronto** (sem hardware de áudio aqui) | Conexões feitas entre dois aplicativos; o som não pôde ser ouvido neste ambiente |
| Soundboard (de fábrica, gravar, importar, cortar, na chamada) | **Pronto** | Som criado no editor, mandado e tocado na sala; gravação depende de microfone |
| Transmissão de tela | **Pronto** | A tela de um aplicativo apareceu ao vivo dentro da chamada do outro |
| Ajustes (perfil, privacidade, aparência, rede, armazenamento) | **Pronto** | Todas as páginas abertas; cor e fundo mantidos ao reabrir |
| Código de convite curto, banner e jogos favoritos no perfil | **Pronto** (o celular ainda não entende) | Testes entre duas contas e conferido na interface |
| Ajustes de voz e áudio (microfone, saída, volumes, teste) | **Pronto** | Escolher o microfone pelo nome funcionou onde o padrão falhava; sem som real aqui |
| Chamada em tela cheia no estilo Discord | **Pronto** | Chamada 1:1, transmissão e sala de voz; tela cheia real depende de um gerenciador de janelas |
| GIFs (GIPHY com a chave do app; Tenor depois; do computador, animados) | **Pronto** (o celular ainda não mostra) | GIF enviado e animado nos dois lados. A busca no GIPHY liga com a chave do app, mas **não foi feita daqui**: a rede deste ambiente bloqueia o GIPHY |
| Música do Spotify no perfil ("Ouvindo Spotify") | **Pronto** (o celular ainda não mostra) | Spotify de teste no D-Bus: a música apareceu no perfil, no cabeçalho e sumiu ao parar. No Windows lê o título da janela do Spotify — **não testado num Windows** |
| Animações (janelas, menus, barra da esquerda, "digitando", chamada chegando) | **Pronto** | Conferidas quadro a quadro em vídeo gravado da tela; dá para desligar em Aparência |
| Instalador e desinstalador animados | **Pronto no Linux; no Windows só compilado** | Instalar, atualizar e desinstalar testados no Linux (tela e arquivos). Registro, atalhos .lnk e Área de Trabalho do Windows não puderam ser executados aqui |
| Executável e instalador do Windows | **Gerados, não executados** | Compilação cruzada daqui; instalar e rodar só se testa num Windows |
| Mandar a câmera do computador | **Não implementado** | O computador recebe a câmera do celular, mas não manda a própria |
| Ler QR pela câmera | **Não implementado** | O computador mostra o próprio QR; para adicionar, cola-se o código |
| Chave da conta num cofre do sistema | **Não implementado** | Fica no banco local sem cifra extra (fraqueza conhecida, explicada no app) |

Nada nesta tabela é marcado como pronto sem ter sido executado.

## Como está organizado

```
crates/
  jm-core/     protocolo cifrado, servidores, identidade, banco, motor
  jm-media/    áudio (Opus), vídeo (VP8), conexão WebRTC, captura de tela
  jm-app/      interface nativa em Iced, chamadas e soundboard
tools/
  testrelay.mjs   servidor Nostr local para testes
```

`jm-core` e `jm-media` não conhecem a interface e rodam sem janela, o que
permite testá-los de verdade em linha de comando.

## Compilar e rodar no Linux

Dependências do sistema (Debian/Ubuntu):

```bash
sudo apt install build-essential pkg-config \
  libasound2-dev libopus-dev libvpx-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libvulkan-dev \
  libgl1-mesa-dev libgbm-dev libegl1-mesa-dev libxcb1-dev \
  libpipewire-0.3-dev libdbus-1-dev libfontconfig1-dev libclang-dev
```

Depois:

```bash
cargo run --release -p jm-app
```

Os dados ficam em `~/.local/share/J-Menseger/jmenseger.db`.

### Testar dois clientes na mesma máquina

```bash
cd tools && npm install && node testrelay.mjs 7781 &

J_MENSEGER_DATA=/tmp/ana/jm.db  J_MENSEGER_RELAYS=ws://127.0.0.1:7781 cargo run -p jm-app &
J_MENSEGER_DATA=/tmp/beto/jm.db J_MENSEGER_RELAYS=ws://127.0.0.1:7781 cargo run -p jm-app &
```

| Variável | Para quê |
|---|---|
| `J_MENSEGER_DATA` | Caminho do banco local |
| `J_MENSEGER_RELAYS` | Servidores, separados por vírgula |
| `RUST_LOG` | Nível dos registros, ex.: `RUST_LOG=jm_core=debug` |

## Gerar o programa para Windows

**Pronto para baixar:**
[`packaging/windows/dist/J-Menseger-0.6.0-instalador.exe`](packaging/windows/dist/J-Menseger-0.6.0-instalador.exe).

É um arquivo só, e é o próprio programa. Com "instalador" no nome, ele abre
o **instalador animado** (no estilo do Discord): janela sem bordas com luzes
passeando ao fundo, a logo flutuando, barra de progresso com brilho e um
"certo" que se desenha no final, e então abre o J-Menseger. Ele:

- instala **só para você, sem pedir administrador**, em
  `%LOCALAPPDATA%\Programs\J-Menseger\J-Menseger.exe`;
- cria o atalho no Menu Iniciar e, se marcado, na Área de Trabalho (a de
  verdade, mesmo quando ela está no OneDrive);
- aparece em "Aplicativos instalados", com desinstalador também animado
  (que pergunta se é para apagar a conta e as conversas, desmarcado por
  padrão);
- instalado de novo por cima, **atualiza** sem perder nada (dá para fazer
  com o J-Menseger aberto: o antigo continua até ser fechado);
- tem o botão "Só abrir, sem instalar" (o modo portátil de antes).

Renomeado sem "instalador" no nome (ou com `--portatil`), abre direto o
aplicativo. Quem instalou a versão 0.3 pelo instalador antigo (em Arquivos
de Programas) pode desinstalá-la pelo Windows: as conversas não são
apagadas e aparecem na versão nova.

O aplicativo é Rust com duas dependências em C: **libvpx** (vídeo) e **libopus**
(voz). Tudo o mais usa bibliotecas que já vêm no Windows, então o executável
final **não acompanha nenhuma DLL**.

### A partir do Linux (é como os arquivos entregues foram gerados)

```bash
sudo apt install gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 nasm
rustup target add x86_64-pc-windows-gnu

# Compila libvpx e libopus na versão para Windows (uma vez só).
packaging/windows/bibliotecas-c.sh

export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_LIBDIR="$PWD/target/win-c/lib/pkgconfig"
cargo build --release -p jm-app --target x86_64-pc-windows-gnu

# O instalador é o mesmo programa, com "instalador" no nome.
cp target/x86_64-pc-windows-gnu/release/j-menseger.exe J-Menseger-0.6.0-instalador.exe
```

A busca de GIFs usa chaves do próprio aplicativo, embutidas ao compilar:
`JM_TENOR_KEY=… JM_GIPHY_KEY=… cargo build …` (ou escritas em
`crates/jm-app/src/gifs.rs`). Sem chave, a busca fica desligada.

**O que foi conferido aqui:** o `.exe` é um PE64 de interface gráfica (não abre
console), tem o ícone e os dados do programa embutidos, e só depende de DLLs do
próprio Windows (`d3d11`, `mmdevapi`, `combase`, `ws2_32`…). **O que não foi:**
rodar. Não há Windows neste ambiente, então a janela, o som e a captura de tela
ainda não foram vistos funcionando lá.

### No próprio Windows

```powershell
# Rust + Visual Studio Build Tools ("Desenvolvimento para desktop com C++")
vcpkg install opus:x64-windows-static-md libvpx:x64-windows-static-md
$env:VCPKG_ROOT = "C:\vcpkg"
cargo build --release -p jm-app
```

No Windows a captura de tela usa Windows Graphics Capture e o áudio usa WASAPI,
pelas mesmas bibliotecas usadas no Linux — não há código diferente na aplicação.

### Onde ficam os dados

`%APPDATA%\J-Menseger\jmenseger.db`. O desinstalador só apaga essa pasta se
você marcar a opção: sem as 12 palavras, uma conta apagada não volta. Na versão final do Windows não
há console, então os registros vão para `jmenseger.log`, ao lado do banco.

## Linux e Mac (pacotes para distribuir)

Os dois são compilados pelo GitHub Actions
([`.github/workflows/pc.yml`](../.github/workflows/pc.yml)) a cada envio que
mexe nesta pasta. Os arquivos prontos ficam em **Actions → PC (Linux e
macOS) → execução → Artifacts**.

- **Linux:** compila no Ubuntu 22.04 (glibc 2.35), então roda no Ubuntu
  22.04+, Debian 12+, Fedora 36+, Mint 21+. O libvpx vai dentro do programa
  (`JM_VPX_STATIC=1`). [`packaging/linux/empacotar.sh`](packaging/linux/empacotar.sh)
  gera o `.deb` e o `.tar.gz` (com `instalar.sh`, sem `sudo`).
- **Mac:** um `.dmg` para Apple Silicon (macos-15) e um para Intel
  (macos-15-intel), macOS 11 ou mais novo.
  [`packaging/macos/empacotar.sh`](packaging/macos/empacotar.sh) monta o `.app`
  com as permissões de câmera, microfone e tela. Ele também confere que nada
  depende do Homebrew e assina "ad hoc". Sem a conta paga da Apple não há
  notarização: na primeira vez, **Ajustes do Sistema → Privacidade e
  Segurança → Abrir Mesmo Assim**.

Só no Windows por enquanto:
- ícone na bandeja;
- abrir ao ligar;
- áudio do PC na transmissão;
- atualização automática.

No Linux e no Mac, fechar a janela encerra o app.

### Instalar no Linux a partir do código

```bash
cargo build --release -p jm-app
packaging/linux/instalar.sh      # instala em ~/.local
```

## Testes

```bash
cargo test --workspace
```

São 172 testes automáticos. Os que dependem de rede sobem o servidor local de
`tools/` automaticamente (rode `npm install` lá antes). Os de tela e de
chamada são pulados com aviso quando não há tela ou rede utilizável, em vez de
falharem por algo que não é defeito.

O que os testes cobrem, resumido:

- **Compatibilidade com o celular:** as mesmas 12 palavras geram a mesma conta;
  um envelope cifrado montado pelo celular abre no computador.
- **Protocolo:** envelope de três camadas, horários embaralhados, recusa de
  tentativa de falsificar autoria.
- **Mensagens:** conversa completa entre dois clientes com servidor real,
  contato bloqueado, conta restaurada trazendo o histórico de volta.
- **Grupos:** convite, entrada, conversa no canal, saída, e que quem não tem a
  chave não lê nada.
- **Áudio:** compressão e descompressão, silêncio custando pouco, pacote
  perdido reconstruído, detecção de fala.
- **Chamada:** dois pares conectam e o áudio atravessa a conexão cifrada.
- **Tela:** 60 capturas seguidas sem esgotar o sistema, compressão real, e os
  quadros recebidos virando imagem do tamanho certo.

### Testando a interface

A interface foi conferida dirigindo dois aplicativos de verdade (com
`xdotool` sobre um Xvfb) contra o servidor local, e com uma terceira conta que
manda foto, voz e som:

```bash
cargo run --example terceira_pessoa -p jm-app -- /tmp/caio ws://127.0.0.1:7781 npub1…
```

Nos testes automáticos não há janela de escolher arquivo do sistema. Por isso,
**só na versão de desenvolvimento**, a tecla F9 abre o arquivo indicado em
`J_MENSEGER_TESTE_ARQUIVO` como se tivesse sido escolhido
(`J_MENSEGER_TESTE_ALVO` = `foto`, `perfil`, `banner` ou `som`; com
`gifgrade`, enche a grade de GIFs com o GIF do arquivo, para conferir o
desenho sem chave de API). Isso não existe na versão final.

### Além dos testes automáticos

Há coisas que teste nenhum pega, porque dependem de ver a tela. A transmissão
foi conferida com dois aplicativos de verdade rodando lado a lado, um
transmitindo e o outro recebendo. Medindo a área da imagem recebida a 83
amostras por segundo, ela mudava **cerca de 14 vezes por segundo** — e isto
com os dois aplicativos, o codificador e o decodificador dividindo a mesma
máquina, sem GPU. Num computador comum, com placa de vídeo, a conta é outra;
o número honesto aqui é o do pior caso.

## Privacidade

- As conversas são cifradas de ponta a ponta (NIP-17, NIP-44 v2, NIP-59). Os
  servidores só veem um envelope fechado endereçado a uma chave pública: não
  sabem quem enviou, o que diz, nem o horário real.
- O áudio, o vídeo e a tela transmitida vão **direto entre os computadores**,
  cifrados com DTLS-SRTP. Os servidores não participam disso.
- A chave da conta é criada no aparelho. Não há telefone, e-mail nem cadastro.
- "Digitando…", sinalização de chamada e presença usam envelopes efêmeros, que
  os servidores repassam e não guardam.

## Licença

MIT.
