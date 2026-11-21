//! Tipos de mensagem internos do J-Menseger.
//!
//! Dentro do envelope cifrado existem três formatos de rumor:
//!
//! - **kind 14** — texto puro, no campo `content`.
//! - **kind 7** — reação; `content` é o emoji e a marca `e` aponta a mensagem.
//! - **kind 1400** — mensagem interna do aplicativo; `content` é um JSON cujo
//!   campo `t` diz o que é.
//!
//! Os nomes curtos (`t`, `n`, `p`…) não são economia à toa: cada mensagem é
//! cifrada duas vezes e vai em base64, então cada byte poupado aqui vira três
//! bytes a menos no envelope, e o limite dos servidores é de 64 KB por evento.

use serde::{Deserialize, Serialize};

/// Como uma conversa é identificada, tanto na tela quanto no banco.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChatKey {
    /// Conversa de duas pessoas: `dm:<chave pública>`.
    Direct { peer: String },
    /// Canal de um grupo: `g:<id do grupo>:<id do canal>`.
    Channel { lid: String, channel: String },
}

impl ChatKey {
    pub fn parse(s: &str) -> Option<Self> {
        if let Some(pk) = s.strip_prefix("dm:") {
            if pk.len() == 64 && pk.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(ChatKey::Direct { peer: pk.to_string() });
            }
            return None;
        }
        let rest = s.strip_prefix("g:")?;
        let (lid, ch) = rest.split_once(':')?;
        if lid.len() != 64 || ch.is_empty() {
            return None;
        }
        Some(ChatKey::Channel {
            lid: lid.to_string(),
            channel: ch.to_string(),
        })
    }

    pub fn as_string(&self) -> String {
        match self {
            ChatKey::Direct { peer } => format!("dm:{peer}"),
            ChatKey::Channel { lid, channel } => format!("g:{lid}:{channel}"),
        }
    }

    pub fn is_group(&self) -> bool {
        matches!(self, ChatKey::Channel { .. })
    }
}

impl std::fmt::Display for ChatKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_string())
    }
}

/// Estado de entrega de uma mensagem enviada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    /// Ainda não saiu do aparelho.
    Pending,
    /// Algum servidor aceitou.
    Sent,
    /// O aparelho da outra pessoa confirmou que recebeu.
    Delivered,
    /// A outra pessoa abriu a conversa.
    Read,
}

impl Delivery {
    /// O estado só anda para frente: uma confirmação de entrega atrasada não
    /// pode rebaixar uma mensagem que já foi lida.
    pub fn advance(self, other: Delivery) -> Delivery {
        if other > self {
            other
        } else {
            self
        }
    }
}

/// Música tocando agora (por enquanto, só do Spotify).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Listening {
    /// Nome da música (vazio: parou de tocar).
    pub s: String,
    /// Artista.
    #[serde(default)]
    pub a: String,
    /// Álbum.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub al: String,
    /// Identificador da faixa no Spotify (22 letras e números).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Quando começou a tocar (ms desde 1970).
    #[serde(default)]
    pub st: i64,
    /// Duração em segundos, quando se sabe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<u32>,
}

fn corta(s: &str, n: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(n).collect::<String>().trim().to_string()
}

impl Listening {
    /// Limpa o que veio de fora. `None` quando não está tocando nada.
    pub fn clean(&self) -> Option<Listening> {
        let s = corta(&self.s, 120);
        if s.is_empty() {
            return None;
        }
        let id = self.id.as_ref().filter(|i| i.len() == 22 && i.chars().all(|c| c.is_ascii_alphanumeric())).cloned();
        Some(Listening { s, a: corta(&self.a, 120), al: corta(&self.al, 120), id, st: self.st.max(0), d: self.d.filter(|d| *d > 0 && *d <= 3 * 3600) })
    }

    /// Mesma faixa (ignora quando começou).
    pub fn same_track(&self, o: &Listening) -> bool {
        self.s == o.s && self.a == o.a && self.id == o.id
    }

    /// Link para ouvir: a faixa, ou uma busca no Spotify.
    pub fn url(&self) -> String {
        match &self.id {
            Some(id) => format!("https://open.spotify.com/track/{id}"),
            None => {
                let q: String = format!("{} {}", self.s, self.a)
                    .chars()
                    .map(|c| if c.is_alphanumeric() { c.to_string() } else if c == ' ' { "%20".into() } else { c.to_string().bytes().map(|b| format!("%{b:02X}")).collect() })
                    .collect();
                format!("https://open.spotify.com/search/{q}")
            }
        }
    }
}

/// Mensagem interna do aplicativo (rumor de kind 1400).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "t")]
pub enum AppMessage {
    /// Perfil de quem envia: nome, recado, cor, foto e servidores preferidos.
    #[serde(rename = "prof")]
    Profile {
        n: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        s: String,
        #[serde(default)]
        c: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        p: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        r: Vec<String>,
        /// Jogos favoritos (versões antigas não mandam: fica como estava).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        g: Option<Vec<String>>,
    },
    /// Banner do perfil (data URL; vazio tira o banner). Separado do perfil
    /// porque foto e banner juntos passariam do tamanho que os servidores
    /// aceitam por envelope.
    #[serde(rename = "pban")]
    ProfileBanner { b: String },
    /// "Me manda seu perfil": enviado quando ainda não se conhece o nome.
    #[serde(rename = "profreq")]
    ProfileRequest,
    /// Confirmação de entrega (`d`) ou de leitura (`r`).
    #[serde(rename = "rcpt")]
    Receipt { s: ReceiptKind, ids: Vec<String> },
    /// "Digitando…". Vai sempre como efêmero.
    #[serde(rename = "typ")]
    Typing,
    /// Apagar para todos.
    #[serde(rename = "del")]
    Delete { ids: Vec<String> },
    /// Liga ou desliga mensagens temporárias (`v` em segundos, 0 desliga).
    #[serde(rename = "ttl")]
    Ttl { v: u32 },
    /// Cabeçalho de uma mídia, seguido pelos blocos.
    #[serde(rename = "mh")]
    MediaHeader(MediaHeader),
    /// Um bloco de mídia, em base64.
    #[serde(rename = "mc")]
    MediaChunk { mid: String, i: u32, d: String },
    /// Som do soundboard de fábrica.
    #[serde(rename = "snd")]
    Sound { id: String },
    /// Convite para um grupo, com a chave do grupo dentro.
    /// O que a pessoa está ouvindo (`s` vazio: parou). Sempre efêmero.
    #[serde(rename = "lis")]
    Listening(Listening),
    #[serde(rename = "ginv")]
    GroupInvite { g: GroupInvite },
    /// O grupo trocou de chave (alguém foi removido).
    #[serde(rename = "grekey")]
    GroupRekey {
        lid: String,
        old: String,
        sk: String,
        meta: serde_json::Value,
        ver: u64,
    },
    /// Dados do grupo mudaram (nome, canais, membros).
    #[serde(rename = "gmeta")]
    GroupMeta { meta: serde_json::Value, ver: u64 },
    /// Entrou no grupo, ou atualizou o próprio nome e foto nele.
    #[serde(rename = "gjoin")]
    GroupJoin {
        n: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        p: Option<String>,
    },
    /// Saiu do grupo.
    #[serde(rename = "gleave")]
    GroupLeave,
    /// Foto do grupo, com um número de versão para resolver conflitos.
    #[serde(rename = "gpic")]
    GroupPic {
        #[serde(default)]
        p: String,
        v: u64,
    },
    /// Presença em sala de voz. Vai sempre como efêmero.
    #[serde(rename = "room")]
    Room { ch: String, a: Presence },
    /// Sinalização de chamada. Efêmera, menos `missed`, que fica no histórico.
    #[serde(rename = "call")]
    Call(CallSignal),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReceiptKind {
    /// Entregue.
    #[serde(rename = "d")]
    Delivered,
    /// Lida.
    #[serde(rename = "r")]
    Read,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Presence {
    Here,
    Leave,
}

/// Cabeçalho de uma mídia fatiada em blocos.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaHeader {
    pub mid: String,
    #[serde(rename = "type")]
    pub kind: MediaKind,
    pub mime: String,
    pub size: u64,
    /// Quantos blocos esperar.
    pub n: u32,
    /// Hash do arquivo inteiro, para detectar bloco corrompido.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sha: String,
    /// Duração do áudio, em segundos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dur: Option<u32>,
    /// Forma de onda para desenhar o áudio sem precisar baixá-lo (0 a 31).
    ///
    /// Lida como inteiro largo: um valor fora da faixa é aparado depois, em
    /// vez de fazer a mensagem inteira ser descartada.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wf: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    /// Miniatura embutida, para a foto aparecer antes de chegar inteira.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th: Option<String>,
    /// Quando o áudio é um som do soundboard criado pela pessoa.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sb: Option<SoundMeta>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Audio,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundMeta {
    pub n: String,
    pub e: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupInvite {
    /// Chave privada do grupo: quem tem isto lê o grupo.
    pub sk: String,
    pub meta: serde_json::Value,
    pub ver: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pic: String,
    /// O nome no fio é `picV`, como no aplicativo mobile.
    #[serde(default, rename = "picV")]
    pub pic_v: u64,
}

/// Sinalização de chamada, trocada por envelope efêmero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallSignal {
    /// Identificador da chamada, para não confundir duas ao mesmo tempo.
    ///
    /// Opcional no fio: o celular manda o aviso de chamada perdida sem ele
    /// (`{t:'call', a:'missed', v}`), e exigir o campo fazia o PC descartar
    /// todas as chamadas perdidas vindas do celular.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cid: String,
    pub a: CallAction,
    /// Descrição da conexão WebRTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdp: Option<String>,
    /// 1 quando é chamada de vídeo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v: Option<u8>,
    /// Sinalização de sala de voz: `cid` vale `"room"` e aqui vai qual sala.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<RoomRef>,
}

/// Uma sala de voz: o grupo e o canal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoomRef {
    pub lid: String,
    pub ch: String,
}

impl CallSignal {
    /// Sinal de chamada comum (duas pessoas).
    pub fn new(cid: &str, a: CallAction) -> Self {
        Self { cid: cid.into(), a, sdp: None, v: None, room: None }
    }

    /// É sinalização de sala de voz?
    pub fn is_room(&self) -> bool {
        self.cid == "room" && self.room.is_some()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CallAction {
    Offer,
    Answer,
    Decline,
    Busy,
    End,
    /// Única que fica guardada: vira "chamada perdida" no histórico.
    Missed,
}

impl AppMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("mensagem interna não serializou")
    }

    pub fn parse(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }

    /// Se esta mensagem deve ir em envelope efêmero (não guardada pelo servidor).
    pub fn is_ephemeral(&self) -> bool {
        match self {
            AppMessage::Typing | AppMessage::Room { .. } | AppMessage::Listening(_) => true,
            AppMessage::Call(c) => c.a != CallAction::Missed,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ouvindo_agora_limpa_e_monta_o_link() {
        let l = Listening { s: "  Música\u{7}  ".into(), a: "Banda".into(), id: Some("4uLU6hMCjMI75M1A2tKUQC".into()), st: 5, d: Some(200), ..Default::default() };
        let c = l.clean().unwrap();
        assert_eq!(c.s, "Música");
        assert_eq!(c.url(), "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC");
        let mut sem_id = c.clone();
        sem_id.id = Some("ruim/../x".into());
        let sem_id = sem_id.clean().unwrap();
        assert!(sem_id.id.is_none());
        assert_eq!(sem_id.url(), "https://open.spotify.com/search/M%C3%BAsica%20Banda".replace("M%C3%BAsica", "Música"));
        assert!(Listening::default().clean().is_none());
        let j = AppMessage::Listening(c).to_json();
        assert!(j.starts_with(r#"{"t":"lis","s":"Música""#), "{j}");
        assert!(AppMessage::parse(&j).unwrap().is_ephemeral());
    }

    #[test]
    fn chave_de_conversa_vai_e_volta() {
        let pk = "a".repeat(64);
        let dm = ChatKey::parse(&format!("dm:{pk}")).unwrap();
        assert_eq!(dm, ChatKey::Direct { peer: pk.clone() });
        assert_eq!(dm.as_string(), format!("dm:{pk}"));
        assert!(!dm.is_group());

        let g = ChatKey::parse(&format!("g:{pk}:geral")).unwrap();
        assert_eq!(
            g,
            ChatKey::Channel { lid: pk.clone(), channel: "geral".into() }
        );
        assert!(g.is_group());
        assert_eq!(g.as_string(), format!("g:{pk}:geral"));
    }

    #[test]
    fn chave_de_conversa_malformada_e_recusada() {
        assert!(ChatKey::parse("dm:curta").is_none());
        assert!(ChatKey::parse("dm:zz").is_none());
        assert!(ChatKey::parse("g:abc:geral").is_none(), "id de grupo curto");
        assert!(ChatKey::parse(&format!("g:{}:", "a".repeat(64))).is_none(), "canal vazio");
        assert!(ChatKey::parse("qualquer coisa").is_none());
    }

    #[test]
    fn entrega_so_anda_para_frente() {
        assert_eq!(Delivery::Sent.advance(Delivery::Read), Delivery::Read);
        assert_eq!(
            Delivery::Read.advance(Delivery::Delivered),
            Delivery::Read,
            "confirmação atrasada não pode rebaixar"
        );
        assert_eq!(Delivery::Pending.advance(Delivery::Sent), Delivery::Sent);
    }

    #[test]
    fn mensagens_internas_vao_e_voltam_em_json() {
        let casos: Vec<AppMessage> = vec![
            AppMessage::Profile {
                n: "Ana".into(),
                s: "testando".into(),
                c: 3,
                p: Some("data:image/webp;base64,AAA".into()),
                r: vec!["wss://nos.lol".into()],
                g: Some(vec!["Minecraft".into()]),
            },
            AppMessage::ProfileBanner { b: "data:image/jpeg;base64,AAA".into() },
            AppMessage::ProfileRequest,
            AppMessage::Receipt {
                s: ReceiptKind::Read,
                ids: vec!["a".repeat(64)],
            },
            AppMessage::Typing,
            AppMessage::Ttl { v: 3600 },
            AppMessage::Sound { id: "aplausos".into() },
            AppMessage::GroupLeave,
            AppMessage::GroupPic { p: String::new(), v: 7 },
            AppMessage::Room { ch: "sala".into(), a: Presence::Here },
            AppMessage::Call(CallSignal {
                cid: "c1".into(),
                a: CallAction::Offer,
                sdp: Some("v=0".into()),
                v: Some(1),
                room: None,
            }),
        ];
        for m in casos {
            let json = m.to_json();
            let de = AppMessage::parse(&json)
                .unwrap_or_else(|| panic!("não voltou de {json}"));
            assert_eq!(de, m, "mudou ao ir e voltar: {json}");
        }
    }

    #[test]
    fn o_formato_no_fio_e_o_que_o_celular_espera() {
        // Estes nomes de campo são o contrato com o aplicativo mobile.
        // Mudar qualquer um deles quebra a conversa entre PC e celular.
        let j = AppMessage::Typing.to_json();
        assert_eq!(j, r#"{"t":"typ"}"#);

        let j = AppMessage::Receipt { s: ReceiptKind::Delivered, ids: vec!["ab".into()] }.to_json();
        assert_eq!(j, r#"{"t":"rcpt","s":"d","ids":["ab"]}"#);

        let j = AppMessage::Ttl { v: 0 }.to_json();
        assert_eq!(j, r#"{"t":"ttl","v":0}"#);

        let j = AppMessage::MediaChunk { mid: "m1".into(), i: 2, d: "QQ==".into() }.to_json();
        assert_eq!(j, r#"{"t":"mc","mid":"m1","i":2,"d":"QQ=="}"#);

        let j = AppMessage::Room { ch: "voz".into(), a: Presence::Leave }.to_json();
        assert_eq!(j, r#"{"t":"room","ch":"voz","a":"leave"}"#);
    }

    #[test]
    fn chamada_perdida_do_celular_e_entendida() {
        // engine.js: { t: 'call', a: 'missed', v: video ? 1 : 0 } — sem cid.
        match AppMessage::parse(r#"{"t":"call","a":"missed","v":1}"#) {
            Some(AppMessage::Call(c)) => {
                assert_eq!(c.a, CallAction::Missed);
                assert_eq!(c.v, Some(1));
            }
            outro => panic!("chamada perdida descartada: {outro:?}"),
        }
        let j = AppMessage::Call(CallSignal { v: Some(0), ..CallSignal::new("", CallAction::Missed) }).to_json();
        assert_eq!(j, r#"{"t":"call","a":"missed","v":0}"#, "o PC manda no mesmo formato");
    }

    #[test]
    fn sinal_de_sala_tem_o_formato_do_celular() {
        // calls.js: e.signal(pk, { cid: 'room', a: 'end', room: { lid, ch } })
        let s = CallSignal {
            room: Some(RoomRef { lid: "l".into(), ch: "voz".into() }),
            ..CallSignal::new("room", CallAction::End)
        };
        let j = AppMessage::Call(s.clone()).to_json();
        assert_eq!(j, r#"{"t":"call","cid":"room","a":"end","room":{"lid":"l","ch":"voz"}}"#);
        assert!(s.is_room());
        assert!(!CallSignal::new("abc", CallAction::End).is_room());
    }

    #[test]
    fn forma_de_onda_fora_da_faixa_nao_derruba_a_mensagem() {
        let j = r#"{"t":"mh","mid":"m","type":"audio","mime":"audio/webm","size":10,"n":1,"wf":[-3,40,7]}"#;
        match AppMessage::parse(j) {
            Some(AppMessage::MediaHeader(h)) => assert_eq!(h.wf, Some(vec![-3, 40, 7])),
            outro => panic!("cabeçalho perdido: {outro:?}"),
        }
    }

    #[test]
    fn cabecalho_de_midia_omite_o_que_nao_tem() {
        let h = MediaHeader {
            mid: "m1".into(),
            kind: MediaKind::Image,
            mime: "image/webp".into(),
            size: 1000,
            n: 1,
            sha: "ab".into(),
            dur: None,
            wf: None,
            w: Some(640),
            h: Some(480),
            th: None,
            sb: None,
        };
        let j = AppMessage::MediaHeader(h.clone()).to_json();
        assert!(!j.contains("\"dur\""), "campo vazio não deve ocupar espaço: {j}");
        assert!(!j.contains("\"th\""));
        assert!(j.contains(r#""type":"image""#));
        match AppMessage::parse(&j).unwrap() {
            AppMessage::MediaHeader(v) => assert_eq!(v, h),
            outro => panic!("voltou errado: {outro:?}"),
        }
    }

    #[test]
    fn sabe_o_que_vai_como_efemero() {
        assert!(AppMessage::Typing.is_ephemeral());
        assert!(AppMessage::Room { ch: "x".into(), a: Presence::Here }.is_ephemeral());
        let oferta = CallSignal::new("c", CallAction::Offer);
        assert!(AppMessage::Call(oferta).is_ephemeral());

        let perdida = CallSignal::new("c", CallAction::Missed);
        assert!(
            !AppMessage::Call(perdida).is_ephemeral(),
            "chamada perdida precisa ficar no histórico"
        );
        assert!(!AppMessage::Ttl { v: 1 }.is_ephemeral());
        assert!(!AppMessage::ProfileRequest.is_ephemeral());
    }
}
