# J-Menseger para computador — decisões técnicas

Este documento registra as escolhas de arquitetura do porte do aplicativo mobile
para uma aplicação desktop nativa, e o porquê de cada uma.

O produto é o mesmo: o celular e o computador são dois clientes do **mesmo**
aplicativo, com a mesma conta, o mesmo protocolo e os mesmos servidores.

---

## 1. Interface: Iced 0.14

| Candidato | Avaliação |
|---|---|
| **Iced 0.14** | **Escolhido.** Rust puro, renderização por `wgpu` (Vulkan/Metal/DX12) com retorno automático para software quando não há GPU. Publicado no crates.io com versões estáveis. Widgets customizáveis em Rust, sem linguagem de marcação intermediária. |
| GPUI (Zed) | Descartado. Não é publicado no crates.io como versão estável; só por dependência de git, com API volátil e suporte imaturo a Linux e Windows. Arriscado como base de um produto. |
| Slint | Viável, bom suporte a Windows. Descartado porque a interface é descrita em uma linguagem de marcação própria (`.slint`), o que divide a lógica entre Rust e um arquivo externo. Em uma tela de conversa com listas dinâmicas e widgets sob medida, manter tudo em Rust é mais direto. |
| egui | Descartado. Modo imediato é ótimo para ferramentas internas, mas entrega um acabamento aquém do exigido para um aplicativo de comunicação. |

**Validação feita, não presumida:** um protótipo com o layout de quatro colunas
(barra de navegação, barra lateral, área de mensagens, painel de membros) foi
compilado e executado de verdade neste contêiner, sob `Xvfb` com renderização por
software, e a janela foi capturada. Não há HTML, CSS, WebView, Electron ou
Chromium em nenhuma camada.

## 2. Protocolo: biblioteca `nostr` 0.45 (Rust)

O aplicativo mobile fala o protocolo aberto Nostr, usando as NIPs 17, 44 e 59
através da biblioteca JavaScript `nostr-tools`.

A versão desktop usa a biblioteca Rust `nostr` (projeto rust-nostr), que
implementa as **mesmas NIPs**. Isso não é uma dependência de conveniência: é o
que garante compatibilidade de fio entre os dois clientes. Reimplementar
NIP-44 v2 à mão seria reintroduzir risco criptográfico sem ganho nenhum.

- `nostr` — eventos, assinaturas Schnorr (BIP-340), NIP-06 (12 palavras),
  NIP-44 v2 (ChaCha20 + HMAC-SHA256), NIP-59 (gift wrap), NIP-19 (npub/nsec).
- `nostr-sdk` — conjunto de relays com reconexão, filtros e NIP-42 (AUTH).

A camada específica do aplicativo (os tipos de mensagem `t:` dentro do rumor de
kind 1400, o formato das chaves de conversa, o fatiamento de mídia) é portada
diretamente do motor em JavaScript e descrita em `PROTOCOLO.md`.

## 3. Armazenamento: SQLite

O celular usa IndexedDB. No computador, SQLite (`rusqlite`, compilado junto) com
o mesmo modelo de dados: `contacts`, `groups`, `chats`, `msgs`, `media`,
`chunks`, `calls`, `outbox`, `sounds`, `kv`.

Vantagens sobre um arquivo solto: consultas indexadas por conversa e data,
transações, e histórico grande sem carregar tudo na memória.

A chave privada **não** fica no banco: vai no cofre de credenciais do sistema
operacional (Credential Manager no Windows, Secret Service no Linux, Keychain no
macOS), espelhando o uso do Android Keystore no celular.

## 4. Separação em módulos

```
j-menseger-desktop/
  crates/
    jm-core/    protocolo, relays, identidade, armazenamento, estado
    jm-app/     interface nativa (Iced), navegação, telas
    jm-media/   áudio, vídeo e captura de tela (a seguir)
```

`jm-core` não conhece a interface e não depende do Iced: roda sem janela, o que
permite testá-lo com clientes reais em linha de comando, exatamente como o motor
em JavaScript é testado hoje contra um relay local.

### Dentro da interface (`jm-app`)

```
app.rs        telas de entrada, mensagens da interface (Msg), escuta da janela
state.rs      estado da tela principal e consultas (nomes, fotos, contagens)
update.rs     o que cada ação faz (chama o motor, abre janelas, PIN, mídia)
v_*.rs        desenho: barra, conversa, grupos, chamadas, ajustes, janelas
ui.rs         peças reaproveitadas (avatar, linhas de ajuste, forma de onda)
canvas.rs     desenhos interativos: corte de som e recorte de foto
calls.rs      chamadas 1:1 e salas de voz no protocolo do celular
sounds.rs     biblioteca de sons (fábrica, próprios, recebidos na chamada)
platform.rs   o que depende do sistema: arquivos, links, notificações,
              esconder a janela das capturas (Windows)
theme.rs      paleta dinâmica (cor de destaque e fundo escolhidos)
icons.rs      ícones em traço desenhados à mão, no estilo dos do celular
```

Três detalhes que não são óbvios:

- **Posição do menu de contexto.** O Iced não informa onde o clique direito
  aconteceu. A posição do ponteiro é guardada a partir da escuta de eventos da
  janela, sem passar pela fila de mensagens (mexer o mouse não redesenha a
  janela). Como essa escuta pode chegar um instante depois do clique, o menu
  espera um quadro antes de ler a posição.
- **Lista de mensagens ancorada embaixo.** Mensagens novas não empurram a
  leitura. Com a âncora embaixo, o deslocamento conta a partir do fim — "ir
  para a mais nova" é deslocamento zero.
- **Fotos e avatares** viram imagem uma vez só e ficam guardados; desenhar
  decodificando a cada quadro travaria a janela.

## 5. Concorrência

- Uma thread de interface, que nunca faz trabalho pesado.
- Um runtime Tokio para rede, relays e banco.
- Threads dedicadas para captura de áudio, captura de tela e codificação.
  Capturar e comprimir a tela é trabalho bloqueante e constante; no executor
  assíncrono ele disputaria espaço com a rede. Além disso, no Windows o
  capturador guarda um ponteiro do sistema e não pode atravessar threads — o
  compilador cruzado para Windows foi quem apontou o erro, que no Linux passava
  despercebido.
- A comunicação entre o núcleo e a interface é por canal de eventos, não por
  bloqueio: a interface recebe atualizações e redesenha.

Uma transmissão de tela a 60 FPS não pode travar a digitação de uma mensagem.

## 6. Voz, vídeo e transmissão de tela

| Camada | Escolha | Observação |
|---|---|---|
| Transporte | `webrtc` (webrtc-rs) | Rust puro, DTLS-SRTP, canais de dados. É o mesmo transporte que o celular usa, o que mantém a chamada compatível entre PC e celular. |
| Áudio de entrada/saída | `cpal` | Enumeração e captura de dispositivos em Windows (WASAPI), Linux (ALSA) e macOS (CoreAudio). |
| Codec de áudio | `opus` | Mesmo codec do celular. |
| Captura de tela | `xcap` + APIs nativas | Monitor e janela; Windows Graphics Capture no Windows, PipeWire/X11 no Linux. |
| Codec de vídeo | VP8 por software (libvpx) | **Só software.** NVENC, AMF e Quick Sync não estão implementados. VP8 foi escolhido por ser obrigatório em WebRTC, o que garante que o celular entenda a transmissão sem negociação extra. |

**Limitação deste ambiente, declarada desde já:** este contêiner não tem GPU,
placa de som, câmera nem tela física. Posso escrever e compilar essas camadas e
testar a lógica, mas **não posso validar aqui** qualidade de imagem, FPS reais,
latência de áudio ou codificação por hardware. Isso exige rodar em máquinas de
verdade, e será declarado como não testado até que isso aconteça.

## 7. Compilar para Windows a partir do Linux

O aplicativo é Rust puro com duas exceções em C: **libvpx** (vídeo) e
**libopus** (voz). Tudo o mais — janela, desenho, rede, áudio do sistema,
captura de tela — usa as bibliotecas do próprio Windows, sem nenhuma DLL extra.

Por isso o executável do Windows é gerado aqui, no Linux, em dois passos:

1. `packaging/windows/bibliotecas-c.sh` compila libvpx e libopus para Windows
   com o mingw-w64 e escreve os arquivos `.pc` correspondentes.
2. O `cargo build --target x86_64-pc-windows-gnu` encontra essas bibliotecas
   pelo pkg-config, exatamente como encontra as de Linux.

As fontes em C vêm do repositório da própria distribuição, na mesma versão que
o Linux daqui usa. Isso importa: ligações geradas contra uma versão e
executadas contra outra fazem o libvpx recusar a inicialização, erro que já
apareceu neste projeto.

O resultado é um `.exe` que depende apenas de DLLs que vêm no Windows
(`d3d11`, `mmdevapi`, `ws2_32`, `combase`…), sem runtime do mingw junto.

**O que isso não prova:** o executável foi compilado e teve seus recursos
conferidos aqui, mas **não foi executado**, porque não há Windows neste
ambiente. Compilar prova que o código é válido para o alvo; não prova que a
janela abre, que o som sai nem que a captura de tela funciona lá.

## 8. O que não é aceitável neste projeto

- Botão que não faz o que diz.
- Dados de exemplo fingindo ser conversa real.
- Recurso declarado pronto sem ter sido executado.
- Um segundo sistema de contas, separado do celular.
