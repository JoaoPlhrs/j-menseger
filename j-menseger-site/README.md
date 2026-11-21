# Site do J-Menseger

Página estática (HTML, CSS e um pouco de JavaScript), sem etapa de build.
Abra `index.html` ou publique a pasta inteira em qualquer hospedagem de
arquivos (GitHub Pages, Netlify, Cloudflare Pages…).

- `downloads/` — os arquivos que os botões baixam: instalador do Windows,
  APK, `.deb`/`.tar.gz` do Linux e `.dmg` do Mac (os dois últimos vêm do
  GitHub Actions, workflow "PC (Linux e macOS)"). Ao lançar versão nova,
  troque os arquivos e os números em `index.html` e rode
  `tools/gerar-versao.sh <pc> <android> <código> "novidades"`.
- `assets/img/` — capturas reais do app.
- `assets/fonts/` — Bricolage Grotesque e Figtree (as mesmas do app, licença OFL).
- O link do código no GitHub ainda não foi colocado.
