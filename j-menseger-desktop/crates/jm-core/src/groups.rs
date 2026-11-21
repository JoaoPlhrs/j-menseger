//! Grupos no estilo servidor: canais de texto, canais de voz e membros.
//!
//! Um grupo tem **a sua própria chave**. Quem recebe um convite recebe essa
//! chave, e é ela que abre as mensagens do grupo — não existe servidor
//! decidindo quem entra. Por isso, tirar alguém do grupo não é apagar um nome
//! de uma lista: é trocar a chave e entregar a nova só a quem fica.

use nostr::prelude::*;
use serde::{Deserialize, Serialize};

/// Tipo de canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Text,
    Voice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ChannelKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// Dados do grupo que todos os membros compartilham.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lid: Option<String>,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub t: u8,
    pub channels: Vec<Channel>,
    pub owner: String,
    #[serde(default)]
    pub admins: Vec<String>,
    #[serde(default)]
    pub members: Vec<String>,
    /// Nome que cada membro escolheu mostrar dentro do grupo.
    #[serde(default)]
    pub names: std::collections::HashMap<String, String>,
    /// Cargos do grupo (nome e cor), na ordem em que aparecem.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,
    /// Cargos de cada membro.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub mroles: std::collections::HashMap<String, Vec<String>>,
}

/// Um cargo do grupo: só nome e cor (os poderes continuam sendo de dono e admins).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub n: String,
    /// Uma das 8 cores da paleta (0 a 7), igual no celular.
    #[serde(default)]
    pub c: u8,
    /// Poderes do cargo: `pin` (fixar), `del` (apagar mensagens dos outros),
    /// `todos` (mencionar @todos). Mudar o grupo continua só com admins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub p: Vec<String>,
}

/// Poderes que um cargo pode dar.
pub const PERMS: [(&str, &str); 3] = [("pin", "Fixar mensagens"), ("del", "Apagar mensagens de outros"), ("todos", "Mencionar @todos")];

pub const MAX_ROLES: usize = 12;
pub const MAX_ROLES_POR_MEMBRO: usize = 4;

/// As 8 cores de cargo, em RGB (as mesmas do celular).
pub const ROLE_CORES: [(u8, u8, u8); 8] = [
    (0xef, 0x53, 0x50),
    (0xff, 0x98, 0x00),
    (0xfd, 0xd8, 0x35),
    (0x4c, 0xaf, 0x50),
    (0x26, 0xa6, 0x9a),
    (0x42, 0xa5, 0xf5),
    (0xab, 0x47, 0xbc),
    (0xec, 0x40, 0x7a),
];

fn valid_role_id(id: &str) -> bool {
    (1..=12).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Limites que evitam que um grupo malformado (ou mal-intencionado) estoure a
/// memória ou a tela de quem recebe.
const MAX_CHANNELS: usize = 20;
const MAX_MEMBERS: usize = 64;
const MAX_ADMINS: usize = 10;

impl GroupMeta {
    pub fn is_admin(&self, pk: &str) -> bool {
        self.owner == pk || self.admins.iter().any(|a| a == pk)
    }

    /// A pessoa tem este poder (admins têm todos).
    pub fn can(&self, pk: &str, perm: &str) -> bool {
        self.is_admin(pk) || self.roles_of(pk).iter().any(|r| r.p.iter().any(|x| x == perm))
    }

    /// Cargos de um membro, na ordem do grupo.
    pub fn roles_of(&self, pk: &str) -> Vec<&Role> {
        let Some(ids) = self.mroles.get(pk) else { return Vec::new() };
        self.roles.iter().filter(|r| ids.contains(&r.id)).collect()
    }

    pub fn text_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter().filter(|c| c.kind == ChannelKind::Text)
    }

    pub fn voice_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter().filter(|c| c.kind == ChannelKind::Voice)
    }

    /// Nome de um membro dentro do grupo.
    pub fn name_of(&self, pk: &str) -> Option<&str> {
        self.names.get(pk).map(|s| s.as_str())
    }

    /// Valida e apara dados vindos de fora.
    ///
    /// Tudo isto chega pela rede, de outro aparelho: nada pode ser aceito do
    /// jeito que veio.
    pub fn sanitize(mut self, owner_fallback: &str) -> Option<GroupMeta> {
        self.name = clamp(&self.name, 40);
        if self.name.is_empty() {
            self.name = "Grupo".into();
        }
        self.desc = clamp(&self.desc, 140);
        self.t %= 6;

        self.channels.truncate(MAX_CHANNELS);
        self.channels.retain(|c| valid_channel_id(&c.id));
        for c in &mut self.channels {
            c.name = clamp(&c.name, 24);
            if c.name.is_empty() {
                c.name = c.id.clone();
            }
            c.topic = c.topic.as_deref().map(|t| clamp(t, 80)).filter(|t| !t.is_empty());
        }
        // Um grupo sem canal de texto não teria onde conversar.
        if !self.channels.iter().any(|c| c.kind == ChannelKind::Text) {
            return None;
        }

        if !is_hex64(&self.owner) {
            self.owner = owner_fallback.to_string();
        }
        if !is_hex64(&self.owner) {
            return None;
        }

        self.members.retain(|m| is_hex64(m));
        self.members.truncate(MAX_MEMBERS);
        self.admins.retain(|a| is_hex64(a));
        self.admins.truncate(MAX_ADMINS);
        self.names.retain(|k, _| is_hex64(k));
        for v in self.names.values_mut() {
            *v = clamp(v, 40);
        }
        self.clean_roles();
        Some(self)
    }
}

/// Um grupo como ele fica guardado neste aparelho.
#[derive(Debug, Clone)]
pub struct Group {
    /// Identificador estável do grupo. Não muda quando a chave é trocada.
    pub lid: String,
    /// Chave pública atual: é o endereço das mensagens do grupo.
    pub pk: String,
    /// Chave privada atual: é o que permite ler.
    pub sk: String,
    /// Chaves antigas, para continuar lendo o histórico depois de uma troca.
    pub old_keys: Vec<(String, String)>,
    pub meta: GroupMeta,
    pub ver: u64,
    pub pic: Option<String>,
    pub pic_v: u64,
    /// Foto que cada membro mostrou dentro do grupo (vem no `gjoin`).
    pub pics: std::collections::HashMap<String, String>,
    /// Canais de texto apagados por um admin: mensagens atrasadas que ainda
    /// chegarem para eles são descartadas, em vez de ressuscitar o canal.
    pub gone: Vec<String>,
    pub muted: bool,
    pub joined_at: i64,
}

impl Group {
    /// Cria um grupo novo, com a chave sorteada neste aparelho.
    pub fn create(name: &str, owner: &PublicKey, owner_name: &str, template: Template) -> Self {
        let keys = Keys::generate();
        let pk = keys.public_key().to_hex();
        let owner_hex = owner.to_hex();

        let mut usados = std::collections::HashSet::new();
        let mut canais = Vec::new();
        for (nome, kind) in template.channels() {
            let mut id = slug(nome);
            while !usados.insert(id.clone()) {
                id.push_str("-2");
            }
            canais.push(Channel {
                id,
                name: nome.to_string(),
                kind,
                topic: None,
            });
        }

        let mut names = std::collections::HashMap::new();
        names.insert(owner_hex.clone(), owner_name.to_string());

        Self {
            lid: pk.clone(),
            pk: pk.clone(),
            sk: keys.secret_key().to_secret_hex(),
            old_keys: Vec::new(),
            meta: GroupMeta {
                roles: Vec::new(),
                mroles: Default::default(),
                lid: Some(pk.clone()),
                name: clamp(name, 40),
                desc: String::new(),
                t: rand::random::<u8>() % 6,
                channels: canais,
                owner: owner_hex.clone(),
                admins: vec![owner_hex.clone()],
                members: vec![owner_hex],
                names,
            },
            ver: 1,
            pic: None,
            pic_v: 0,
            pics: std::collections::HashMap::new(),
            gone: Vec::new(),
            muted: false,
            joined_at: crate::engine::now_ms(),
        }
    }

    pub fn keys(&self) -> Option<Keys> {
        Keys::parse(&self.sk).ok()
    }

    /// Todas as chaves que este aparelho ainda consegue abrir, incluindo as
    /// antigas: sem isso o histórico sumiria a cada troca de chave.
    pub fn all_keys(&self) -> Vec<(String, Keys)> {
        let mut v = Vec::new();
        if let Ok(k) = Keys::parse(&self.sk) {
            v.push((self.pk.clone(), k));
        }
        for (pk, sk) in &self.old_keys {
            if let Ok(k) = Keys::parse(sk) {
                v.push((pk.clone(), k));
            }
        }
        v
    }

    /// Primeiro canal de texto, usado para avisos do próprio grupo.
    pub fn main_channel(&self) -> Option<&Channel> {
        self.meta.text_channels().next()
    }

    /// Troca a chave do grupo, mantendo a antiga para ler o histórico.
    pub fn rekey(&mut self) -> Keys {
        let novas = Keys::generate();
        self.old_keys.push((self.pk.clone(), self.sk.clone()));
        if self.old_keys.len() > 5 {
            self.old_keys.remove(0);
        }
        self.pk = novas.public_key().to_hex();
        self.sk = novas.secret_key().to_secret_hex();
        self.ver += 1;
        novas
    }
}

/// Uma mudança feita por um administrador. Cada uma vale para todos os
/// membros: depois de aplicada, os dados novos são publicados no grupo.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupPatch {
    Name(String),
    Desc(String),
    AddChannel { name: String, kind: ChannelKind },
    EditChannel { id: String, name: Option<String>, topic: Option<String> },
    /// `dir` negativo sobe, positivo desce, sempre entre canais do mesmo tipo.
    MoveChannel { id: String, dir: i8 },
    RemoveChannel(String),
    Admin { pk: String, on: bool },
    AddRole { n: String, c: u8 },
    EditRole { id: String, n: Option<String>, c: Option<u8>, p: Option<Vec<String>> },
    RemoveRole(String),
    MemberRole { pk: String, id: String, on: bool },
}

impl GroupMeta {
    /// Aplica uma mudança, com as mesmas regras do celular. Devolve os canais
    /// que deixaram de existir, para apagar as mensagens deles.
    pub fn apply(&mut self, patch: &GroupPatch) -> Vec<Channel> {
        let mut removidos = Vec::new();
        match patch {
            GroupPatch::Name(n) => {
                let n = clamp(n, 40);
                if !n.is_empty() {
                    self.name = n;
                }
            }
            GroupPatch::Desc(d) => self.desc = clamp(d, 140),
            GroupPatch::AddChannel { name, kind } => {
                let nome = clamp(name, 24);
                if !nome.is_empty() && self.channels.len() < MAX_CHANNELS {
                    let mut id = slug(&nome);
                    while self.channels.iter().any(|c| c.id == id) {
                        id.push_str("-2");
                    }
                    self.channels.push(Channel { id, name: nome, kind: *kind, topic: None });
                }
            }
            GroupPatch::EditChannel { id, name, topic } => {
                if let Some(c) = self.channels.iter_mut().find(|c| &c.id == id) {
                    if let Some(n) = name {
                        let n = clamp(n, 24);
                        if !n.is_empty() {
                            c.name = n;
                        }
                    }
                    if let Some(t) = topic {
                        let t = clamp(t, 80);
                        c.topic = (!t.is_empty()).then_some(t);
                    }
                }
            }
            GroupPatch::MoveChannel { id, dir } => {
                if let Some(i) = self.channels.iter().position(|c| &c.id == id) {
                    let passo: isize = if *dir < 0 { -1 } else { 1 };
                    let tipo = self.channels[i].kind;
                    let mut j = i as isize + passo;
                    while j >= 0 && (j as usize) < self.channels.len() && self.channels[j as usize].kind != tipo {
                        j += passo;
                    }
                    if j >= 0 && (j as usize) < self.channels.len() {
                        self.channels.swap(i, j as usize);
                    }
                }
            }
            GroupPatch::RemoveChannel(id) => {
                if let Some(c) = self.channels.iter().find(|c| &c.id == id).cloned() {
                    let ultimo_texto = c.kind == ChannelKind::Text && self.text_channels().count() == 1;
                    if !ultimo_texto {
                        self.channels.retain(|x| x.id != c.id);
                        removidos.push(c);
                    }
                }
            }
            GroupPatch::AddRole { n, c } => {
                if self.roles.len() < MAX_ROLES {
                    let mut b = [0u8; 4];
                    getrandom_bytes(&mut b);
                    let id: String = b.iter().map(|x| format!("{x:02x}")).collect();
                    let n = clamp(n, 24);
                    self.roles.push(Role { id, n: if n.is_empty() { "Cargo".into() } else { n }, c: c % 8, p: Vec::new() });
                }
            }
            GroupPatch::EditRole { id, n, c, p } => {
                if let Some(r) = self.roles.iter_mut().find(|r| &r.id == id) {
                    if let Some(n) = n {
                        let n = clamp(n, 24);
                        if !n.is_empty() {
                            r.n = n;
                        }
                    }
                    if let Some(c) = c {
                        r.c = c % 8;
                    }
                    if let Some(p) = p {
                        r.p = p.clone();
                    }
                }
            }
            GroupPatch::RemoveRole(id) => {
                self.roles.retain(|r| &r.id != id);
                self.clean_roles();
            }
            GroupPatch::MemberRole { pk, id, on } => {
                if self.roles.iter().any(|r| &r.id == id) {
                    let lista = self.mroles.entry(pk.clone()).or_default();
                    lista.retain(|x| x != id);
                    if *on && lista.len() < MAX_ROLES_POR_MEMBRO {
                        lista.push(id.clone());
                    }
                }
                self.clean_roles();
            }
            GroupPatch::Admin { pk, on } => {
                if *on {
                    if is_hex64(pk) && !self.admins.contains(pk) && self.admins.len() < MAX_ADMINS {
                        self.admins.push(pk.clone());
                    }
                } else if pk != &self.owner {
                    self.admins.retain(|a| a != pk);
                }
            }
        }
        removidos
    }
}

/// Conjuntos de canais prontos, para não começar com o grupo vazio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Amigos,
    Estudos,
    Jogos,
    Branco,
}

impl Template {
    pub fn channels(&self) -> Vec<(&'static str, ChannelKind)> {
        match self {
            Template::Amigos => vec![
                ("geral", ChannelKind::Text),
                ("fotos", ChannelKind::Text),
                ("Sala de estar", ChannelKind::Voice),
            ],
            Template::Estudos => vec![
                ("avisos", ChannelKind::Text),
                ("dúvidas", ChannelKind::Text),
                ("materiais", ChannelKind::Text),
                ("Estudo em grupo", ChannelKind::Voice),
            ],
            Template::Jogos => vec![
                ("geral", ChannelKind::Text),
                ("clipes", ChannelKind::Text),
                ("Jogatina", ChannelKind::Voice),
                ("Sala 2", ChannelKind::Voice),
            ],
            Template::Branco => vec![("geral", ChannelKind::Text), ("Voz", ChannelKind::Voice)],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Template::Amigos => "Amigos",
            Template::Estudos => "Estudos",
            Template::Jogos => "Jogos",
            Template::Branco => "Em branco",
        }
    }

    pub fn all() -> [Template; 4] {
        [Template::Amigos, Template::Estudos, Template::Jogos, Template::Branco]
    }
}

/* ---------------- apoio ---------------- */

fn clamp(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn valid_channel_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 24
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Transforma o nome digitado em um identificador curto e previsível.
pub fn slug(s: &str) -> String {
    let sem_acento: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect();
    let mut out = String::new();
    let mut hifen = false;
    for c in sem_acento.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            hifen = false;
        } else if !out.is_empty() && !hifen {
            out.push('-');
            hifen = true;
        }
        if out.len() >= 20 {
            break;
        }
    }
    let limpo = out.trim_matches('-').to_string();
    if limpo.is_empty() {
        "canal".into()
    } else {
        limpo
    }
}

impl GroupMeta {
    /// Apara cargos vindos de fora: ids válidos, nomes curtos, cores 0..8, e
    /// só membros de verdade com cargos que existem.
    fn clean_roles(&mut self) {
        self.roles.retain(|r| valid_role_id(&r.id));
        self.roles.truncate(MAX_ROLES);
        for r in &mut self.roles {
            r.n = clamp(&r.n, 24);
            if r.n.is_empty() {
                r.n = "Cargo".into();
            }
            r.c %= 8;
            r.p.retain(|x| PERMS.iter().any(|(k, _)| k == x));
            r.p.dedup();
        }
        let ids: Vec<String> = self.roles.iter().map(|r| r.id.clone()).collect();
        let membros = self.members.clone();
        self.mroles.retain(|pk, v| {
            v.retain(|x| ids.contains(x));
            v.truncate(MAX_ROLES_POR_MEMBRO);
            membros.contains(pk) && !v.is_empty()
        });
    }
}

fn getrandom_bytes(b: &mut [u8]) {
    rand::Rng::fill(&mut rand::rng(), b);
}

#[cfg(test)]
mod patch_tests {
    use super::*;

    fn meta() -> GroupMeta {
        let dono = Keys::generate();
        Group::create("Turma", &dono.public_key(), "Ana", Template::Jogos).meta
    }

    #[test]
    fn grupo_novo_leva_o_id_estavel_nos_dados() {
        let dono = Keys::generate();
        let g = Group::create("Turma", &dono.public_key(), "Ana", Template::Amigos);
        assert_eq!(g.meta.lid.as_deref(), Some(g.lid.as_str()));
    }

    #[test]
    fn criar_canal_com_nome_repetido_ganha_sufixo() {
        let mut m = meta();
        m.apply(&GroupPatch::AddChannel { name: "geral".into(), kind: ChannelKind::Text });
        assert!(m.channels.iter().any(|c| c.id == "geral-2"));
    }

    #[test]
    fn mover_canal_pula_os_de_outro_tipo() {
        let mut m = meta(); // geral, clipes (texto); Jogatina, Sala 2 (voz)
        m.apply(&GroupPatch::MoveChannel { id: "clipes".into(), dir: 1 });
        // Não há outro canal de texto abaixo de "clipes": nada muda.
        assert_eq!(m.channels[1].id, "clipes");
        m.apply(&GroupPatch::MoveChannel { id: "clipes".into(), dir: -1 });
        assert_eq!(m.channels[0].id, "clipes");
        assert_eq!(m.channels[1].id, "geral");
    }

    #[test]
    fn ultimo_canal_de_texto_nao_pode_ser_apagado() {
        let mut m = meta();
        assert_eq!(m.apply(&GroupPatch::RemoveChannel("clipes".into())).len(), 1);
        assert!(m.apply(&GroupPatch::RemoveChannel("geral".into())).is_empty());
        assert!(m.channels.iter().any(|c| c.id == "geral"));
    }

    #[test]
    fn dono_nunca_perde_o_cargo_de_admin() {
        let mut m = meta();
        let dono = m.owner.clone();
        m.apply(&GroupPatch::Admin { pk: dono.clone(), on: false });
        assert!(m.is_admin(&dono));
        let outro = "b".repeat(64);
        m.apply(&GroupPatch::Admin { pk: outro.clone(), on: true });
        assert!(m.is_admin(&outro));
        m.apply(&GroupPatch::Admin { pk: outro.clone(), on: false });
        assert!(!m.is_admin(&outro));
    }

    #[test]
    fn descricao_do_canal_pode_ser_tirada() {
        let mut m = meta();
        m.apply(&GroupPatch::EditChannel { id: "geral".into(), name: Some("conversa".into()), topic: Some("tudo".into()) });
        let c = m.channels.iter().find(|c| c.id == "geral").unwrap();
        assert_eq!(c.name, "conversa");
        assert_eq!(c.topic.as_deref(), Some("tudo"));
        m.apply(&GroupPatch::EditChannel { id: "geral".into(), name: None, topic: Some(String::new()) });
        assert!(m.channels.iter().find(|c| c.id == "geral").unwrap().topic.is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargos_sao_aparados_e_seguem_os_membros() {
        let mut m = Group::create("Turma", &Keys::generate().public_key(), "Ana", Template::Jogos).meta;
        let membro = m.members[0].clone();
        m.apply(&GroupPatch::AddRole { n: "Moderador".into(), c: 13 });
        let id = m.roles[0].id.clone();
        assert_eq!(m.roles[0].c, 5, "cor fica entre 0 e 7");
        m.apply(&GroupPatch::MemberRole { pk: membro.clone(), id: id.clone(), on: true });
        assert_eq!(m.roles_of(&membro).len(), 1);
        // Quem não é membro não guarda cargo.
        m.apply(&GroupPatch::MemberRole { pk: "f".repeat(64), id: id.clone(), on: true });
        assert!(!m.mroles.contains_key(&"f".repeat(64)));
        // Apagar o cargo tira de todo mundo.
        m.apply(&GroupPatch::RemoveRole(id));
        assert!(m.roles_of(&membro).is_empty() && m.mroles.is_empty());
        // Cargo vindo de fora com id esquisito some na limpeza.
        let mut v = serde_json::to_value(&m).unwrap();
        v["roles"] = serde_json::json!([{"id":"../x","n":"Mal","c":1},{"id":"ab12","n":"Bom","c":99}]);
        v["mroles"] = serde_json::json!({ membro.clone(): ["ab12","../x"] });
        let limpo: GroupMeta = serde_json::from_value::<GroupMeta>(v).unwrap().sanitize(&m.owner).unwrap();
        assert_eq!(limpo.roles.len(), 1);
        assert_eq!(limpo.roles[0].c, 3);
        assert_eq!(limpo.mroles[&membro], vec!["ab12".to_string()]);
    }

    fn dono() -> Keys {
        Keys::generate()
    }

    #[test]
    fn grupo_novo_nasce_com_canais_e_dono() {
        let k = dono();
        let g = Group::create("Squad Beta", &k.public_key(), "Ana", Template::Amigos);

        assert_eq!(g.meta.name, "Squad Beta");
        assert_eq!(g.meta.owner, k.public_key().to_hex());
        assert!(g.meta.is_admin(&k.public_key().to_hex()));
        assert_eq!(g.meta.members, vec![k.public_key().to_hex()]);
        assert_eq!(g.meta.text_channels().count(), 2);
        assert_eq!(g.meta.voice_channels().count(), 1);
        assert_eq!(g.main_channel().unwrap().id, "geral");
        assert_eq!(g.meta.name_of(&k.public_key().to_hex()), Some("Ana"));
        // A chave do grupo é própria, não a de quem criou.
        assert_ne!(g.pk, k.public_key().to_hex());
        assert!(g.keys().is_some());
    }

    #[test]
    fn identificadores_de_canal_nao_se_repetem() {
        let k = dono();
        let g = Group::create("G", &k.public_key(), "Ana", Template::Jogos);
        let ids: Vec<&str> = g.meta.channels.iter().map(|c| c.id.as_str()).collect();
        let unicos: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unicos.len(), "ids repetidos: {ids:?}");
    }

    #[test]
    fn trocar_a_chave_preserva_o_historico() {
        let k = dono();
        let mut g = Group::create("G", &k.public_key(), "Ana", Template::Branco);
        let pk_antiga = g.pk.clone();
        let ver_antes = g.ver;

        g.rekey();

        assert_ne!(g.pk, pk_antiga, "a chave tem que mudar");
        assert_eq!(g.ver, ver_antes + 1);
        assert_eq!(g.old_keys.len(), 1);
        // Continua conseguindo abrir o que foi escrito com a chave antiga.
        let abriveis: Vec<String> = g.all_keys().into_iter().map(|(pk, _)| pk).collect();
        assert!(abriveis.contains(&pk_antiga));
        assert!(abriveis.contains(&g.pk));
    }

    #[test]
    fn guarda_no_maximo_cinco_chaves_antigas() {
        let k = dono();
        let mut g = Group::create("G", &k.public_key(), "Ana", Template::Branco);
        for _ in 0..8 {
            g.rekey();
        }
        assert_eq!(g.old_keys.len(), 5, "não pode crescer para sempre");
    }

    #[test]
    fn dados_vindos_de_fora_sao_aparados() {
        let dono_hex = "a".repeat(64);
        let bruto = GroupMeta {
            roles: Vec::new(),
            mroles: Default::default(),
            lid: None,
            name: " ".to_string() + &"n".repeat(80),
            desc: "d".repeat(400),
            t: 99,
            channels: vec![
                Channel { id: "geral".into(), name: "x".repeat(60), kind: ChannelKind::Text, topic: Some("t".repeat(200)) },
                Channel { id: "INVÁLIDO!".into(), name: "mau".into(), kind: ChannelKind::Text, topic: None },
            ],
            owner: "não-é-chave".into(),
            admins: vec!["b".repeat(64), "curta".into()],
            members: vec!["c".repeat(64), "ruim".into()],
            names: std::collections::HashMap::from([
                ("d".repeat(64), "z".repeat(90)),
                ("xx".into(), "ignorado".into()),
            ]),
        };

        let m = bruto.sanitize(&dono_hex).expect("deveria sobrar um canal de texto");
        assert_eq!(m.name.chars().count(), 40, "nome aparado");
        assert_eq!(m.desc.chars().count(), 140);
        assert!(m.t < 6);
        assert_eq!(m.channels.len(), 1, "canal com id inválido é descartado");
        assert_eq!(m.channels[0].name.chars().count(), 24);
        assert_eq!(m.channels[0].topic.as_ref().unwrap().chars().count(), 80);
        assert_eq!(m.owner, dono_hex, "dono inválido cai para o esperado");
        assert_eq!(m.admins.len(), 1, "admin com chave inválida sai");
        assert_eq!(m.members.len(), 1);
        assert_eq!(m.names.len(), 1, "nome com chave inválida sai");
        assert_eq!(m.names.values().next().unwrap().chars().count(), 40);
    }

    #[test]
    fn grupo_sem_canal_de_texto_e_recusado() {
        let m = GroupMeta {
            roles: Vec::new(),
            mroles: Default::default(),
            lid: None,
            name: "G".into(),
            desc: String::new(),
            t: 0,
            channels: vec![Channel { id: "voz".into(), name: "Voz".into(), kind: ChannelKind::Voice, topic: None }],
            owner: "a".repeat(64),
            admins: vec![],
            members: vec![],
            names: Default::default(),
        };
        assert!(m.sanitize(&"a".repeat(64)).is_none());
    }

    #[test]
    fn nome_de_canal_vira_identificador_limpo() {
        assert_eq!(slug("Sala de estar"), "sala-de-estar");
        assert_eq!(slug("dúvidas"), "duvidas");
        assert_eq!(slug("  Avisos!!  "), "avisos");
        assert_eq!(slug("ção"), "cao");
        assert_eq!(slug("!!!"), "canal");
        assert_eq!(slug(""), "canal");
        assert!(slug(&"palavra ".repeat(20)).len() <= 20);
    }

    #[test]
    fn modelos_sempre_tem_canal_de_texto() {
        for t in Template::all() {
            let tem = t.channels().iter().any(|(_, k)| *k == ChannelKind::Text);
            assert!(tem, "modelo {} sem canal de texto", t.label());
        }
    }
}
