# J-Menseger para Android

Página (HTML, CSS e JavaScript) dentro de uma WebView, com uma camada Android
fina em Java para o que a página não faz sozinha: guardar a chave no Android
Keystore, notificações, chamada em tela cheia, conexão em segundo plano, viva-voz.

## De onde veio este código

O código-fonte original do celular (versão 0.4.0-beta) se perdeu junto com o
ambiente onde foi escrito. Esta pasta foi **reconstruída a partir do APK
0.4.0-beta**:

- `web/index.html`, `web/styles.css`, os sons e as fontes: tirados do APK
  (o CSS foi só reformatado).
- `web/app.js`: o JavaScript do APK, reformatado para leitura. Nomes de
  métodos, textos e HTML estão como eram; os nomes de variáveis internas são
  os curtos que o empacotador gerou. As bibliotecas (nostr-tools, @noble,
  @scure) estão no começo do arquivo; o código do app vem depois da linha
  ~9500.
- `android/`: reescrito do zero com a mesma ponte (`window.JMNative`) que a
  página usa.

## Montar

```bash
# Ferramentas oficiais do Android (sem Gradle): build-tools 34 e plataforma 34
#   https://dl.google.com/android/repository/build-tools_r34-linux.zip  -> /opt/android/android-14
#   https://dl.google.com/android/repository/platform-34-ext7_r02.zip   -> /opt/android/android-34
./build.sh            # gera build/J-Menseger-<versão>.apk
```

A chave de assinatura fica em `keystore/` (fora do git). **Guarde-a**: sem ela
não dá para lançar atualizações que instalem por cima.

## Novidades da 0.7.0-beta (iguais às do computador 0.6.0)

- Responder e marcar como lida pela notificação (RemoteInput).
- Arquivos até 5 MB, encaminhar, visualização única, prévia de links (busca
  nativa em `LinkPreview.java`, sem endereços internos).
- Poderes por cargo (fixar, apagar de outros, @todos), eventos com presença e
  lembrete.
- Volume por pessoa, status personalizado com Não perturbe, aparelhos
  conectados com "Desconectar".
- Estudo do iPhone: [ESTUDO-IPHONE.md](ESTUDO-IPHONE.md).

## Novidades da 0.6.0-beta (iguais às do computador 0.5.0)

- Editar mensagem, fixar mensagens na conversa, buscar na conversa, enquetes,
  @menções (avisam mesmo com o canal silenciado), online/ausente (pode
  esconder em Privacidade) e cargos com cor nos grupos.
- Backup cifrado com a chave da conta (Armazenamento → Fazer backup), no mesmo
  formato do PC: um backup do celular abre no computador e vice-versa.
- Mesma conta no PC e no celular: quem entra com as mesmas 12 palavras recebe
  contatos e grupos do outro aparelho sozinho.
- Transmitir a tela do celular em chamadas e salas (MediaProjection).
- Atualização automática: o app procura versão nova no site, baixa, confere o
  SHA-256 e pede para instalar.
- Notificações com o app fechado: além do serviço em segundo plano, um
  trabalho a cada ~15 min religa a conexão se o Android a derrubar.
- Tela piscando preta ao entrar em chamada de voz: corrigido (sensor de
  proximidade só apaga a tela depois de 0,65 s com o rosto perto).

**Não testado num aparelho de verdade** (não há um aqui): a captura de tela,
a instalação da atualização e o trabalho periódico. A página foi testada no
Chromium em tamanho de celular, junto com o app de computador.

## Novidades da 0.5.0-beta (iguais às do computador)

Mesmo protocolo do app de computador, conferido com ele rodando de verdade no
servidor de teste: código curto (evento kind 30078), banner (`pban`), jogos
favoritos (`g` no perfil), "ouvindo agora" (`lis`, efêmero, só contatos
aceitos), GIF (`image/gif`, busca no GIPHY com a chave do app), ajustes de
microfone, tela do PC inteira nas chamadas e o teclado que não fecha ao
enviar. Ver `GUIA-DO-TESTADOR.md`.

**Testado aqui:** a página no Chromium em tamanho de celular conversando com
o app de computador (código curto nos dois sentidos, banner, jogos, GIF,
música do Spotify, chamada com a tela do PC). **Não testado:** a camada
Android reescrita (Java) num aparelho ou emulador de verdade — não há um
aqui. O APK foi montado e assinado com as ferramentas oficiais e passou na
verificação de assinatura.
