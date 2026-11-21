// Conversa de exemplo: alterna entre o que vocês leem e o envelope cifrado
// que os servidores recebem. Ao abrir a página, a conversa aparece cifrada e
// se "decifra" uma vez; depois, só muda quando a pessoa escolhe.
(() => {
  const demo = document.querySelector(".demo");
  if (!demo) return;
  const baloes = [...demo.querySelectorAll(".balao")];
  const botoes = [...demo.querySelectorAll(".visao button")];
  const status = document.getElementById("demo-status");
  const nota = document.getElementById("visao-nota");
  const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const semMovimento = matchMedia("(prefers-reduced-motion: reduce)").matches;

  // Texto cifrado fixo para cada balão (parece um envelope NIP-44, em base64).
  const cifra = (txt, semente) => {
    let s = semente * 9301 + 49297, out = "Ag";
    const n = Math.max(24, Math.round(txt.length * 1.5));
    for (let i = 0; i < n; i++) { s = (s * 9301 + 49297) % 233280; out += B64[s % 64]; }
    return out + "==";
  };
  const cifrados = baloes.map((b, i) => cifra(b.dataset.txt, i + 7));
  let atual = "voces", timer = null;

  const pinta = (visao) => {
    demo.classList.toggle("servidor", visao === "servidor");
    status.textContent = visao === "servidor" ? "Envelope cifrado · conteúdo ilegível" : "Conversa cifrada";
    nota.textContent = visao === "servidor"
      ? "É isso que passa pelos servidores: sem a chave de vocês, não dá para ler."
      : "Só quem está na conversa tem a chave para ler.";
    botoes.forEach((b) => b.setAttribute("aria-checked", String(b.dataset.visao === visao)));
  };

  // Troca os caracteres aos poucos, da esquerda para a direita.
  const anima = (de, para, ms = 700) => {
    clearInterval(timer);
    if (semMovimento) {
      baloes.forEach((b, i) => (b.querySelector(".txt").textContent = para === "voces" ? b.dataset.txt : cifrados[i]));
      return;
    }
    const inicio = performance.now();
    timer = setInterval(() => {
      const p = Math.min(1, (performance.now() - inicio) / ms);
      baloes.forEach((b, i) => {
        const alvo = para === "voces" ? b.dataset.txt : cifrados[i];
        const origem = de === "voces" ? b.dataset.txt : cifrados[i];
        const n = Math.round(alvo.length * p);
        let meio = "";
        for (let k = n; k < Math.min(alvo.length, n + 6); k++) meio += B64[(Math.random() * 64) | 0];
        b.querySelector(".txt").textContent = p >= 1 ? alvo : alvo.slice(0, n) + meio + origem.slice(n + meio.length, Math.max(n + meio.length, Math.min(origem.length, alvo.length)));
      });
      if (p >= 1) clearInterval(timer);
    }, 34);
  };

  const escolhe = (visao) => {
    if (visao === atual) return;
    pinta(visao);
    anima(atual, visao);
    atual = visao;
  };

  botoes.forEach((b) => b.addEventListener("click", () => escolhe(b.dataset.visao)));
  // Setas mudam a opção, como em qualquer grupo de botões de opção.
  demo.querySelector(".visao").addEventListener("keydown", (e) => {
    if (!["ArrowLeft", "ArrowRight"].includes(e.key)) return;
    e.preventDefault();
    const prox = atual === "voces" ? "servidor" : "voces";
    escolhe(prox);
    botoes.find((b) => b.dataset.visao === prox).focus();
  });
  botoes.forEach((b) => (b.tabIndex = b.getAttribute("aria-checked") === "true" ? 0 : -1));
  const reTab = () => botoes.forEach((b) => (b.tabIndex = b.getAttribute("aria-checked") === "true" ? 0 : -1));
  botoes.forEach((b) => b.addEventListener("click", reTab));

  // Momento de abertura: começa como o servidor vê e se decifra uma vez.
  if (!semMovimento) {
    baloes.forEach((b, i) => (b.querySelector(".txt").textContent = cifrados[i]));
    pinta("servidor");
    atual = "servidor";
    setTimeout(() => { escolhe("voces"); reTab(); }, 900);
  }
})();

// Animações de rolagem: títulos sobem palavra por palavra, blocos aparecem
// em sequência e o celular da vitrine se move mais devagar que o PC.
(() => {
  const semMovimento = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (semMovimento || !("IntersectionObserver" in window)) return;

  // Quebra os títulos em palavras (mantém <br> e o texto acessível igual).
  document.querySelectorAll("h1, .secao-cabeca h2, .grupo h3, .dl h3").forEach((t) => {
    let i = 0;
    const quebra = (no) => {
      [...no.childNodes].forEach((n) => {
        if (n.nodeType === 3) {
          const frag = document.createDocumentFragment();
          n.textContent.split(/(\s+)/).forEach((p) => {
            if (!p) return;
            if (/^\s+$/.test(p)) { frag.append(p); return; }
            const fora = document.createElement("span"); fora.className = "palavra";
            const dentro = document.createElement("span"); dentro.textContent = p; dentro.style.setProperty("--i", i++);
            fora.append(dentro); frag.append(fora);
          });
          n.replaceWith(frag);
        } else if (n.nodeType === 1 && n.tagName !== "BR" && n.tagName !== "svg" && !n.classList.contains("i")) quebra(n);
      });
    };
    quebra(t);
    t.classList.add("titulo-anima");
  });

  // Blocos que sobem: o que vem depois de cada título, em sequência.
  const grupos = [
    [".hero .lead", ".hero .acoes", ".hero .plataformas", ".demo"],
    [".telas .secao-cabeca p", ".vitrine .pc", ".vitrine .cel"],
    [".grupo"],
    [".privacidade .secao-cabeca p", ".quem-ve tbody tr", ".garantias li"],
    [".baixar .secao-cabeca p", ".dl"],
  ];
  grupos.forEach((sels) => {
    let d = 0;
    sels.forEach((sel) => document.querySelectorAll(sel).forEach((el) => { el.classList.add("sobe"); el.style.setProperty("--d", d++); }));
  });

  const obs = new IntersectionObserver((itens) => {
    itens.forEach((it) => {
      if (!it.isIntersecting) return;
      it.target.classList.add("visivel");
      obs.unobserve(it.target);
    });
  }, { rootMargin: "0px 0px -8% 0px", threshold: 0.12 });
  document.querySelectorAll(".titulo-anima, .sobe").forEach((el) => obs.observe(el));

  // Profundidade: o celular anda mais devagar que a tela do PC.
  const cel = document.querySelector(".vitrine .cel");
  const vitrine = document.querySelector(".vitrine");
  if (cel && vitrine && matchMedia("(min-width: 901px)").matches) {
    let pedido = 0;
    const atualiza = () => {
      pedido = 0;
      const r = vitrine.getBoundingClientRect();
      const p = (window.innerHeight - r.top) / (window.innerHeight + r.height); // 0 → 1 enquanto passa
      const y = (0.5 - Math.min(1, Math.max(0, p))) * 90;
      cel.style.translate = `0 ${y.toFixed(1)}px`;
    };
    addEventListener("scroll", () => { pedido ||= requestAnimationFrame(atualiza); }, { passive: true });
    atualiza();
  }
})();

// Downloads: o cartão do sistema de quem está vendo vem primeiro e marcado.
(() => {
  const ua = navigator.userAgent, plat = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";
  const so = /Android/i.test(ua) ? "android"
    : /iPhone|iPad|iPod/i.test(ua) ? null
    : /Win/i.test(plat) ? "windows"
    : /Mac/i.test(plat) ? "mac"
    : /Linux|X11/i.test(plat + ua) ? "linux" : null;
  const lista = document.querySelector(".downloads");
  const card = so && lista && lista.querySelector(`[data-so="${so}"]`);
  if (!card) return;
  card.classList.add("seu");
  const tag = document.createElement("p");
  tag.className = "seu-tag";
  tag.textContent = "Para o seu aparelho";
  card.prepend(tag);
  lista.prepend(card);
})();
