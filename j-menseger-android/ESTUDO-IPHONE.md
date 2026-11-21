# J-Menseger no iPhone — estudo de compatibilidade

**Resumo:** dá para fazer, reaproveitando quase toda a parte do celular que já
existe. A tela e o motor do app Android são uma página (HTML + JavaScript)
dentro de uma "casca" nativa em Java. No iPhone a mesma página roda dentro de
uma casca em Swift (WKWebView). O que muda é a casca e três limitações do iOS:
**notificações com o app fechado**, **chamadas com o app fechado** e **como
instalar**. As duas primeiras precisam de um pequeno servidor de avisos
(push), e a terceira precisa da conta de desenvolvedor da Apple (paga).

## O que funciona igual (a página do Android, sem mudar)

| Recurso | No iPhone | Observação |
|---|---|---|
| Mensagens, grupos, cargos, eventos, enquetes | Sim | É JavaScript puro (Nostr, NIP-44/59). |
| Fotos, GIFs, arquivos, visualização única, prévia de links | Sim | Escolher arquivo/foto: seletor nativo do iOS. |
| Chamadas de voz e vídeo, salas de voz | Sim, com o app aberto | WebRTC e câmera/microfone no WKWebView desde o iOS 14.3. |
| Ver a tela de quem transmite | Sim | É só vídeo recebido. |
| Soundboard, áudios gravados | Sim | Gravar/tocar Opus em WebM: iOS 17 ou mais novo. Para iOS 15–16, um decodificador pequeno em JavaScript. |
| PIN, backup cifrado, mesma conta no PC e no celular | Sim | |
| Status personalizado, aparelhos conectados, volume por pessoa | Sim | |

## O que precisa de trabalho específico do iOS

1. **Notificações com o app fechado.** O iOS não deixa um app ficar
   conectado em segundo plano (não há o "serviço em primeiro plano" que o
   Android usa). A única forma é o **push da Apple (APNs)**. Isso pede um
   servidorzinho que observa os relays e avisa o iPhone quando chega algo
   para ele. O aviso não leva o conteúdo: o servidor só vê o envelope
   cifrado (como os relays). No iPhone, uma extensão de notificação abre o
   envelope com a chave do aparelho e mostra "Ana: chego às 8". A
   privacidade continua a mesma; o servidor só fica sabendo "esta chave
   pública tem um iPhone".
2. **Chamadas com o app fechado.** Precisa de **CallKit + PushKit** (a tela
   de chamada nativa do iPhone) e do mesmo servidor de push. Com o app
   aberto, ou depois de atender, funciona como no Android.
3. **Transmitir a tela do iPhone.** Precisa de uma extensão **ReplayKit**
   (um processo separado com limite de 50 MB de memória). Dá para fazer,
   mas fica para uma segunda etapa.
4. **"Ouvindo no Spotify".** O iOS não deixa um app ver o que outro está
   tocando. Só daria com login na conta do Spotify (API web). Fica de fora
   no começo.
5. **Bloquear capturas de tela.** O iOS não deixa bloquear. Dá para
   esconder o conteúdo em gravações de tela e avisar quando alguém tira
   print, mas não impedir.

## Como instalar (a parte que depende de você)

- **No iPhone não existe APK.** Para alguém instalar, o app precisa passar
  pela Apple:
  - **Conta Apple Developer: US$ 99 por ano.** Sem ela o app só roda no seu
    próprio iPhone, ligado num Mac, e expira a cada 7 dias.
  - **TestFlight**: com a conta paga, até 10 000 testadores por convite ou
    link; cada versão vale 90 dias. É o equivalente ao APK de teste de hoje.
  - **App Store**: revisão da Apple. Pontos que eles conferem num app como
    este:
    - **Conteúdo de usuários**: precisa de "denunciar" e "bloquear" pessoa.
    - **Apagar a conta** dentro do app (já existe).
    - **Declaração de criptografia** (algoritmos padrão; é um formulário).
- **Compilar precisa de um Mac com Xcode.** Também dá pelo GitHub Actions
  (Macs do GitHub, como já é feito para a versão de Mac). Nesse caso os
  certificados da conta ficam guardados como segredos do repositório.

## Caminho recomendado

| Etapa | O que entra | Precisa de |
|---|---|---|
| 1. Casca iOS | WKWebView + ponte nativa igual à do Android (arquivos, câmera, notificações locais, vibrar, salvar). Mensagens, fotos, áudios e chamadas **com o app aberto**. | Conta Apple Developer (para o TestFlight) |
| 2. Push | Servidor de avisos (pequeno, sem ler nada), extensão de notificação que decifra, CallKit/PushKit para chamadas. | Um servidor sempre ligado (ex.: um container barato), certificado APNs |
| 3. Extras | Transmitir a tela (ReplayKit), denunciar e bloquear, publicar na App Store. | Revisão da Apple |

A etapa 1 sozinha já dá um app usável no iPhone para os testadores. Sem a
etapa 2 ele funciona como o WhatsApp Web: só recebe enquanto está aberto.
Isso é aceitável para testar, mas não para o dia a dia.

**Por que não reescrever em Swift/Rust nativo:** o núcleo em Rust do PC
(`jm-core`) até compila para iOS. Mas a interface do PC (iced) não serve
para celular, e refazer a tela em SwiftUI seria um terceiro app inteiro
para manter igual aos outros dois. Reusar a página do Android mantém o
celular igual em Android e iPhone com um código só.

**Aparelhos:** iOS 15 ou mais novo (iPhone 6s em diante). Isso cobre
praticamente todos os iPhones em uso. Os áudios em Opus tocam nativamente a
partir do iOS 17. No 15 e no 16 tocam pelo decodificador em JavaScript.

## O que eu preciso para começar

1. Decidir se vale assinar a **Apple Developer (US$ 99/ano)**. Sem ela não
   há como entregar o app a outras pessoas.
2. Para a etapa 2: um lugar para o servidor de push ficar ligado.

O código da etapa 1 eu consigo escrever e compilar pelo GitHub Actions
(inclusive rodar no simulador de iPhone de lá). Para mandar ao TestFlight,
precisa da conta.
