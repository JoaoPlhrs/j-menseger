# Paridade com o aplicativo de celular

Cada recurso do J-Menseger de celular e como ele está no computador. A coluna
"Como foi conferido" diz o que de fato rodou. Quando algo não pôde ser
executado aqui (falta de microfone, de câmera ou de um Windows de verdade),
isso está escrito — nada foi marcado como pronto sem ter funcionado.

Legenda: **✓** igual ao celular · **≈** existe com adaptação para o
computador · **✗** não existe no computador · **—** não se aplica.

O ambiente de verificação foi um Linux sem placa de som, sem câmera e sem
Windows: dois aplicativos de verdade (Ana e Beto) conversando por um servidor
local, conferidos tela a tela, mais uma terceira conta de teste
(`crates/jm-app/examples/terceira_pessoa.rs`) mandando foto, voz e som.

## Conta

| Recurso | PC | Como foi conferido |
|---|---|---|
| Criar conta: nome → cor → 12 palavras → avisos | ✓ | Fluxo percorrido nas duas contas |
| "Anotar depois", com lembrete até anotar | ✓ | Ponto amarelo em Ajustes e aviso ao sair |
| Entrar com as 12 palavras ou com `nsec` | ✓ | Testes com vetores das bibliotecas do celular |
| Chave de recuperação (pede o PIN, palavras escondidas, "Já anotei") | ✓ | Tela aberta com e sem PIN |
| Sair e apagar deste aparelho | ✓ | Pede o PIN quando há PIN |

## Contatos

| Recurso | PC | Como foi conferido |
|---|---|---|
| Meu código com QR (`jmenseger://add?k=…&n=…`) | ✓ | QR e código copiável |
| Adicionar por código, link ou convite colado | ✓ | Ana e Beto se adicionaram pelo código |
| Ler o QR de outra pessoa | ✗ | O computador não lê QR (sem câmera). A tela explica e oferece colar o código |
| Apelido, recado e foto do contato | ✓ | Apelido "Aninha" no Beto; foto nova da Ana chegou nele |
| Número de segurança e "verificado" | ✓ | Mesmo cálculo do celular, conferido com 23 vetores dele |
| Pedidos de conversa (aceitar, bloquear) e lista de bloqueados | ✓ | Pedido da terceira conta aceito pela tela de pedidos |
| Convidar amigos | ≈ | Copia o mesmo texto de convite (no celular abre o compartilhar) |

## Conversas

| Recurso | PC | Como foi conferido |
|---|---|---|
| Texto com várias linhas e links clicáveis | ✓ | Enter envia, Shift+Enter quebra a linha |
| Responder citando | ✓ | Citação apareceu no outro lado |
| Reações (6 rápidas e todas as outras) | ✓ | Reação chegou e some ao repetir |
| Copiar, apagar para mim e para todos | ✓ | "Mensagem apagada" apareceu no outro lado |
| Entrega e leitura (tiques), "digitando…" | ✓ | Tiques azuis ao abrir a conversa |
| Mensagens temporárias (por conversa e padrão) | ✓ | Aviso no cabeçalho e mensagem do sistema nos dois lados |
| Fixar, silenciar, marcar como lida, apagar conversa | ✓ | Menu do botão direito e do "…" |
| Mensagem só de emoji em tamanho grande | ✓ | — |
| Fotos: reduzidas, sem metadados, miniatura, tela cheia, salvar | ✓ | Escolhendo arquivo ou arrastando para a janela |
| Tirar foto com a câmera | ✗ | Sem câmera no computador |
| Mensagem de voz: tocar, forma de onda, avançar | ✓ | Áudio da terceira conta exibido; o formato do celular (WebM e Ogg Opus) é decodificado nos testes |
| Mensagem de voz: gravar | ≈ | Implementado (até 10 min). **Não gravado aqui**: o ambiente de teste não tem microfone |
| Sons do soundboard na conversa (de fábrica e próprios) | ✓ | Som de fábrica e som criado chegaram e tocam no outro lado |
| Convite de grupo com botão de entrar | ✓ | Beto entrou pelo cartão do convite |

## Soundboard

| Recurso | PC | Como foi conferido |
|---|---|---|
| 14 sons de fábrica, tirar da lista, restaurar | ✓ | — |
| Volume dos sons | ✓ | — |
| Importar arquivo (MP3, M4A, OGG, WAV, FLAC…) | ✓ | Também soltando o arquivo na janela |
| Editor de corte (até 8 s, forma de onda, ouvir o trecho) | ✓ | Bordas arrastadas, nome e emoji trocados, som salvo |
| Gravar um som (até 10 s) | ≈ | Implementado; **não gravado aqui** (sem microfone) |
| Editar nome/emoji, excluir | ✓ | — |
| Na chamada: toca para todos, aviso de quem tocou, "salvar no meu soundboard" | ✓ | Aviso "Aninha tocou Tcharam!" apareceu no Beto pela conexão direta |

## Grupos

| Recurso | PC | Como foi conferido |
|---|---|---|
| Criar com modelos (Amigos, Estudos, Jogos, Em branco) | ✓ | Grupo "Turma do Rust" criado |
| Convidar contatos | ✓ | Convite enviado e aceito |
| Canais de texto e salas de voz (até 6) | ✓ | — |
| Nome, descrição e foto do grupo | ✓ | — |
| Criar, renomear, dar assunto, mover e apagar canais | ✓ | Janela "Dados do grupo" e botão direito no canal |
| Admins e remover membro (troca a chave) | ✓ | Regras iguais às do celular, cobertas pelos testes do núcleo |
| Silenciar e sair | ✓ | — |
| Salas de voz: presença, quem está falando, microfone desligado | ✓ | Os dois entraram e se conectaram; o áudio real não foi ouvido aqui (sem placa de som) |

## Chamadas

| Recurso | PC | Como foi conferido |
|---|---|---|
| Voz 1:1: chamar, tocar, atender, recusar, ocupado, perdida | ✓ | Chamada atendida; "Ocupado" registrado quando o outro estava numa sala |
| Ver a câmera de quem está no celular | ✓ | Teste com o código do celular rodando no Chromium (640×480) |
| Mandar a câmera do computador | ✗ | Não implementado. A tela da chamada avisa |
| Trocar câmera, viva-voz | — | Não fazem sentido no computador |
| Desligar o microfone (avisando o outro lado) | ✓ | — |
| Transmitir a tela | ≈ | Só o computador transmite; o celular recebe. Conferido entre dois PCs (imagem ao vivo) e com o celular nos testes |
| Histórico de chamadas | ✓ | "Feita · 0:39" e "Ocupado" registrados |
| Aviso de chamada chegando | ≈ | Janela de atender + notificação do sistema + destaque na barra de tarefas. No celular é tela cheia |

## Privacidade e segurança

| Recurso | PC | Como foi conferido |
|---|---|---|
| PIN: criar, trocar, quando pedir, tirar, "esqueci o PIN" | ✓ | Tela de bloqueio ao abrir; PIN errado e certo; mesmo PBKDF2 do celular |
| Bloquear capturas de tela | ≈ | Windows 10 2004+ (`SetWindowDisplayAffinity`). **Não executado num Windows aqui**. No Linux o interruptor aparece desligado com o motivo |
| Confirmação de leitura, "digitando", prévia das notificações, quem pode ligar | ✓ | — |
| Ocultar o IP nas chamadas (TURN) | ✓ | — |
| Chave da conta num cofre do sistema | ✗ | No celular fica no Android Keystore. No PC fica no banco de dados local, sem cifra extra — fraqueza conhecida, explicada em "Sobre" |

## Aparência, rede, armazenamento e avisos

| Recurso | PC | Como foi conferido |
|---|---|---|
| Cor de destaque (9 prontas e qualquer outra) e fundo | ✓ | Trocada ao vivo e mantida ao reabrir |
| Servidores: adicionar, remover, voltar aos de fábrica; TURN | ✓ | — |
| Espaço usado e apagar fotos e áudios | ✓ | Também limpa o histórico de chamadas |
| Notificações e sons no app | ≈ | Notificação do sistema quando a janela não está em foco. **Não vista aqui** (ambiente sem serviço de notificação). No Windows ela usa a identidade padrão de programas não registrados, então aparece com a origem "Windows PowerShell" |
| Manter conectado em segundo plano | — | No computador o app precisa estar aberto (pode ficar minimizado) |
| Vibrar | — | — |

## Novidades que só o computador tem por enquanto

Feitas primeiro no computador; o celular ainda precisa ser atualizado para
entender cada uma. Enquanto isso, o que o celular faz com elas está na
última coluna.

| Recurso | Como foi conferido | No celular de hoje |
|---|---|---|
| Código de convite curto (`XXXX-XXXX`, vale 7 dias) | Beto adicionou a Ana só pelo código curto (teste `codigo_curto_encontra_a_pessoa`) | Não aceita o código curto; o código longo e o QR continuam valendo |
| Banner do perfil (recortado em 3:1) | Banner da Ana apareceu no perfil dela visto pelo Beto | Ignora o banner (`pban`) |
| Jogos favoritos no perfil (até 6) | Jogos da Ana apareceram no perfil dela visto pelo Beto | Ignora a lista (`g`) |
| GIFs: busca no Tenor ou no GIPHY com a chave do próprio app, GIF do computador, GIF animado na conversa | GIF mandado do arquivo chegou ao Beto e anima nos dois lados; grade da busca conferida com GIFs locais; chave falsa recusada pelo Tenor com aviso claro. **A busca está desligada até o app ter chave** | Recusa a mídia `image/gif` (não aparece) |
| Voz e áudio: escolher microfone e saída, volume do microfone e das chamadas, testar o microfone | Tela conferida; escolher o dispositivo pelo nome fez o microfone funcionar numa chamada onde o padrão do sistema falhava; o teste mostra o erro do sistema quando falha | — (no celular quem escolhe é o sistema) |
| Chamada em tela cheia no estilo Discord (F11, Esc para sair) | Chamada 1:1 com e sem transmissão de tela, e sala de voz com duas pessoas | — |
| "Ouvindo Spotify" no perfil, no cabeçalho e na lista de membros | Com um Spotify de teste no D-Bus (Linux): chegou ao contato, com barra de tempo e link, e sumiu ao parar. Aviso `lis` só para contatos aceitos, nunca guardado nos servidores. No Windows lê o título da janela do Spotify — não executado aqui | Ignora o aviso `lis` |
| Animações da interface | Gravadas em vídeo e conferidas quadro a quadro | — |
| Instalador e desinstalador animados | Testados no Linux; no Windows só compilados | — |

## O que ainda não foi executado de verdade

- O programa e o instalador no **Windows** (são gerados aqui por compilação
  cruzada).
- **Microfone e alto-falante reais**: gravação de voz e de sons, e ouvir uma
  chamada. A conexão das chamadas e salas foi conferida; o som, não.
- Notificações do sistema e o bloqueio de capturas, que dependem do Windows.
- A **tela cheia de verdade** da chamada: no ambiente de teste não há
  gerenciador de janelas, então o palco foi conferido dentro da janela.
- A **busca de GIFs no GIPHY** (a chave do app já está no programa; a rede do ambiente de testes bloqueia o GIPHY). O Tenor ainda não tem chave.
- O **instalador no Windows** (registro, atalhos, Área de Trabalho) e a
  leitura do Spotify pelo título da janela: compilados, não executados.
