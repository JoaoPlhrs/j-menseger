//! Camada de protocolo, portada do motor em JavaScript do aplicativo mobile.
//!
//! Toda mensagem viaja dentro de um envelope de três camadas (NIP-59):
//!
//! 1. **rumor** — evento *sem assinatura*, com a chave pública de quem escreveu.
//!    É o conteúdo de verdade. Não assinar é proposital: se o envelope vazar,
//!    não existe prova criptográfica de autoria para terceiros.
//! 2. **selo** (kind 13) — o rumor cifrado com NIP-44 para o destinatário e
//!    assinado por quem envia. Prova a autoria só para o destinatário.
//! 3. **envelope** (kind 1059, ou 21059 efêmero) — o selo cifrado de novo e
//!    assinado por uma chave descartável. É só isso que o servidor vê.
//!
//! Os horários do selo e do envelope são embaralhados no passado para que o
//! servidor não saiba quando a mensagem foi escrita de verdade.

use nostr::prelude::*;
use rand::Rng;
use std::time::Duration;

/// Mensagem de texto (NIP-17).
pub const KIND_TEXT: u16 = 14;
/// Reação a uma mensagem (NIP-25).
pub const KIND_REACTION: u16 = 7;
/// Mensagens internas do aplicativo, com JSON no conteúdo.
pub const KIND_APP: u16 = 1400;
/// Selo (NIP-59).
pub const KIND_SEAL: u16 = 13;
/// Envelope guardado pelos servidores (NIP-59).
pub const KIND_WRAP: u16 = 1059;
/// Envelope efêmero: os servidores repassam ao vivo e não guardam.
///
/// Usado para "digitando…", sinalização de chamada e presença em sala de voz.
/// Está na faixa 20000–29999, que a NIP-01 reserva para eventos efêmeros.
pub const KIND_WRAP_EPH: u16 = 21059;

/// Quanto os horários podem recuar, em segundos (2 dias).
const TIME_FUZZ: u64 = 2 * 24 * 60 * 60;

/// Servidores padrão, os mesmos do aplicativo mobile.
pub const DEFAULT_RELAYS: [&str; 4] = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
    "wss://relay.0xchat.com",
];

/// Bytes por bloco de mídia.
///
/// Um bloco de 20 000 bytes vira cerca de 55 KB depois de virar base64 e passar
/// pelas duas camadas de cifra, o que cabe no limite de 64 KB por evento que os
/// servidores públicos costumam impor.
pub const CHUNK: usize = 20_000;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("erro de evento: {0}")]
    Event(String),
    #[error("erro de cifra: {0}")]
    Crypto(String),
    #[error("envelope inválido: {0}")]
    Invalid(&'static str),
}

/// Um horário recuado aleatoriamente, para esconder quando a mensagem saiu.
fn fuzzed_timestamp() -> Timestamp {
    let back: u64 = rand::rng().random_range(0..TIME_FUZZ);
    Timestamp::now() - back
}

/// Monta o rumor: o evento sem assinatura que carrega o conteúdo.
pub fn make_rumor(
    author: &Keys,
    kind: u16,
    content: impl Into<String>,
    tags: Vec<Tag>,
) -> UnsignedEvent {
    let mut rumor = EventBuilder::new(Kind::Custom(kind), content)
        .tags(tags)
        .finalize_unsigned(author.public_key());
    rumor.ensure_id();
    rumor
}

/// Opções de embrulho.
#[derive(Debug, Clone, Copy, Default)]
pub struct WrapOptions {
    /// Envelope efêmero: kind 21059 e horário igual a agora.
    ///
    /// Os servidores recusam eventos efêmeros com mais de 60 segundos, então
    /// aqui o horário **não** pode ser embaralhado.
    pub ephemeral: bool,
    /// Momento em que o servidor deve descartar o evento (NIP-40).
    pub expiration: Option<Timestamp>,
}

/// Embrulha um rumor para um destinatário.
///
/// O envelope externo é assinado por uma chave descartável, gerada só para este
/// evento e jogada fora em seguida: o servidor não consegue ligar dois
/// envelopes à mesma pessoa.
pub fn wrap_rumor(
    rumor: &UnsignedEvent,
    sender: &Keys,
    recipient: &PublicKey,
    opts: WrapOptions,
) -> Result<Event, ProtocolError> {
    // Camada 2: selo, assinado por quem envia.
    let sealed_content = nip44::encrypt(
        sender.secret_key(),
        recipient,
        rumor.as_json(),
        nip44::Version::V2,
    )
    .map_err(|e| ProtocolError::Crypto(e.to_string()))?;

    let seal_time = if opts.ephemeral {
        Timestamp::now()
    } else {
        fuzzed_timestamp()
    };
    let seal = EventBuilder::new(Kind::Custom(KIND_SEAL), sealed_content)
        .custom_created_at(seal_time)
        .finalize(sender)
        .map_err(|e| ProtocolError::Event(e.to_string()))?;

    // Camada 3: envelope, assinado por uma chave descartável.
    let throwaway = Keys::generate();
    let wrapped_content = nip44::encrypt(
        throwaway.secret_key(),
        recipient,
        seal.as_json(),
        nip44::Version::V2,
    )
    .map_err(|e| ProtocolError::Crypto(e.to_string()))?;

    let kind = if opts.ephemeral { KIND_WRAP_EPH } else { KIND_WRAP };
    let wrap_time = if opts.ephemeral {
        Timestamp::now()
    } else {
        fuzzed_timestamp()
    };

    let mut tags = vec![Tag::public_key(*recipient)];
    if let Some(exp) = opts.expiration {
        tags.push(Tag::expiration(exp));
    }

    EventBuilder::new(Kind::Custom(kind), wrapped_content)
        .tags(tags)
        .custom_created_at(wrap_time)
        .finalize(&throwaway)
        .map_err(|e| ProtocolError::Event(e.to_string()))
}

/// Abre um envelope e devolve o rumor de dentro.
///
/// As três verificações abaixo são o que impede alguém de se passar por outra
/// pessoa. Sem elas, qualquer um poderia selar um rumor com a chave pública de
/// um terceiro e o aplicativo mostraria a mensagem como vinda dele.
pub fn unwrap_event(wrap: &Event, recipient: &Keys) -> Result<UnsignedEvent, ProtocolError> {
    wrap.verify()
        .map_err(|_| ProtocolError::Invalid("assinatura do envelope não confere"))?;

    let seal_json = nip44::decrypt(recipient.secret_key(), &wrap.pubkey, &wrap.content)
        .map_err(|e| ProtocolError::Crypto(e.to_string()))?;
    let seal = Event::from_json(&seal_json).map_err(|e| ProtocolError::Event(e.to_string()))?;

    seal.verify()
        .map_err(|_| ProtocolError::Invalid("assinatura do selo não confere"))?;
    if seal.kind != Kind::Custom(KIND_SEAL) {
        return Err(ProtocolError::Invalid("o selo não é do tipo esperado"));
    }

    let rumor_json = nip44::decrypt(recipient.secret_key(), &seal.pubkey, &seal.content)
        .map_err(|e| ProtocolError::Crypto(e.to_string()))?;
    let mut rumor =
        UnsignedEvent::from_json(&rumor_json).map_err(|e| ProtocolError::Event(e.to_string()))?;

    // Quem selou precisa ser quem escreveu.
    if rumor.pubkey != seal.pubkey {
        return Err(ProtocolError::Invalid(
            "o autor do rumor não é quem selou o envelope",
        ));
    }

    // O id do rumor é o hash do próprio conteúdo: recalcular impede que alguém
    // troque o texto mantendo um id conhecido.
    let claimed = rumor.id;
    rumor.ensure_id();
    if let (Some(a), Some(b)) = (claimed, rumor.id) {
        if a != b {
            return Err(ProtocolError::Invalid("o id do rumor não bate com o conteúdo"));
        }
    }

    Ok(rumor)
}

/// Momento em que um evento expira, se ele tiver essa marca (NIP-40).
pub fn expiration_of(event: &UnsignedEvent) -> Option<Timestamp> {
    event.tags.iter().find_map(|t| {
        let s = t.as_slice();
        if s.first().map(|x| x.as_str()) == Some("expiration") {
            s.get(1)?.parse::<u64>().ok().map(Timestamp::from)
        } else {
            None
        }
    })
}

/// Valor de uma marca simples, como `p` (destinatário) ou `ch` (canal).
pub fn tag_value<'a>(event: &'a UnsignedEvent, name: &str) -> Option<&'a str> {
    event.tags.iter().find_map(|t| {
        let s = t.as_slice();
        if s.first().map(|x| x.as_str()) == Some(name) {
            s.get(1).map(|x| x.as_str())
        } else {
            None
        }
    })
}

/// Duração até um instante futuro, usada para a expiração do envelope.
pub fn duration_until(ts: Timestamp) -> Duration {
    let now = Timestamp::now().as_secs();
    Duration::from_secs(ts.as_secs().saturating_sub(now))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_e_volta_de_uma_mensagem() {
        let ana = Keys::generate();
        let beto = Keys::generate();

        let rumor = make_rumor(
            &ana,
            KIND_TEXT,
            "Oi Beto! 👋",
            vec![Tag::public_key(beto.public_key())],
        );
        let wrap = wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();

        assert_eq!(wrap.kind, Kind::Custom(KIND_WRAP));
        assert_ne!(wrap.pubkey, ana.public_key(), "o envelope esconde quem enviou");

        let aberto = unwrap_event(&wrap, &beto).unwrap();
        assert_eq!(aberto.content, "Oi Beto! 👋");
        assert_eq!(aberto.pubkey, ana.public_key());
        assert_eq!(aberto.id, rumor.id);
    }

    #[test]
    fn so_o_destinatario_abre() {
        let ana = Keys::generate();
        let beto = Keys::generate();
        let intruso = Keys::generate();

        let rumor = make_rumor(&ana, KIND_TEXT, "segredo", vec![]);
        let wrap = wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();

        assert!(unwrap_event(&wrap, &intruso).is_err());
        assert!(unwrap_event(&wrap, &beto).is_ok());
    }

    #[test]
    fn envelope_efemero_usa_o_horario_de_agora() {
        let ana = Keys::generate();
        let beto = Keys::generate();
        let rumor = make_rumor(&ana, KIND_APP, r#"{"t":"typ"}"#, vec![]);
        let wrap = wrap_rumor(
            &rumor,
            &ana,
            &beto.public_key(),
            WrapOptions { ephemeral: true, expiration: None },
        )
        .unwrap();

        assert_eq!(wrap.kind, Kind::Custom(KIND_WRAP_EPH));
        let idade = Timestamp::now().as_secs().saturating_sub(wrap.created_at.as_secs());
        assert!(idade < 60, "os servidores recusam efêmeros com mais de 60 s");
    }

    #[test]
    fn envelope_guardado_tem_horario_embaralhado() {
        let ana = Keys::generate();
        let beto = Keys::generate();
        // Com 2 dias de janela, 30 amostras iguais seriam praticamente impossíveis.
        let mut distintos = std::collections::HashSet::new();
        for _ in 0..30 {
            let rumor = make_rumor(&ana, KIND_TEXT, "oi", vec![]);
            let w = wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();
            assert!(w.created_at <= Timestamp::now());
            distintos.insert(w.created_at.as_secs());
        }
        assert!(distintos.len() > 20, "os horários não estão sendo embaralhados");
    }

    #[test]
    fn rumor_adulterado_e_recusado() {
        let ana = Keys::generate();
        let beto = Keys::generate();
        let falsario = Keys::generate();

        // O falsário sela um rumor que diz ter sido escrito pela Ana.
        let mentira = make_rumor(&ana, KIND_TEXT, "transfere o dinheiro", vec![]);
        let wrap = wrap_rumor(&mentira, &falsario, &beto.public_key(), WrapOptions::default())
            .unwrap();

        let erro = unwrap_event(&wrap, &beto).unwrap_err();
        assert!(matches!(erro, ProtocolError::Invalid(_)));
    }

    #[test]
    fn expiracao_e_lida_de_volta() {
        let ana = Keys::generate();
        let beto = Keys::generate();
        let exp = Timestamp::now() + 3600u64;
        let rumor = make_rumor(&ana, KIND_TEXT, "some depois", vec![Tag::expiration(exp)]);
        let wrap =
            wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();
        let aberto = unwrap_event(&wrap, &beto).unwrap();
        assert_eq!(expiration_of(&aberto), Some(exp));
    }
}

/// Valor de uma marca em um evento já assinado (o envelope).
pub fn tag_value_event<'a>(event: &'a Event, name: &str) -> Option<&'a str> {
    event.tags.iter().find_map(|t| {
        let s = t.as_slice();
        if s.first().map(|x| x.as_str()) == Some(name) {
            s.get(1).map(|x| x.as_str())
        } else {
            None
        }
    })
}
