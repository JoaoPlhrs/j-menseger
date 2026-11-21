//! Prova que o cliente de computador e o do celular são o mesmo aplicativo.
//!
//! O aplicativo mobile é escrito em JavaScript e usa as bibliotecas
//! `nostr-tools` e `@scure/bip39`. Estes testes consomem vetores gerados por
//! **essas mesmas bibliotecas** (veja `tests/fixtures/gerar-vetores.mjs`) e
//! exigem que o código em Rust chegue exatamente ao mesmo resultado.
//!
//! Se algum destes testes falhar, as duas versões deixaram de conversar: uma
//! conta restaurada no computador daria outra chave, ou uma mensagem vinda do
//! celular não abriria.

use jm_core::identity;
use jm_core::protocol;
use nostr::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Vetores {
    mnemonics: Vec<Frase>,
    nip44: Vec<Cifra>,
    wrap: Envelope,
}

#[derive(Deserialize)]
struct Frase {
    words: String,
    sk: String,
    pk: String,
    npub: String,
    nsec: String,
}

#[derive(Deserialize)]
struct Cifra {
    de_sk: String,
    para_pk: String,
    texto: String,
    cifrado: String,
}

#[derive(Deserialize)]
struct Envelope {
    destinatario_sk: String,
    rumor_esperado: serde_json::Value,
    evento: serde_json::Value,
}

fn vetores() -> Vetores {
    let bruto = include_str!("fixtures/vetores-do-mobile.json");
    serde_json::from_str(bruto).expect("vetores de teste ilegíveis")
}

/// As 12 palavras em português têm que dar a mesma chave nos dois aplicativos.
#[test]
fn doze_palavras_dao_a_mesma_conta_que_no_celular() {
    let v = vetores();
    assert!(!v.mnemonics.is_empty(), "sem vetores para comparar");

    for f in &v.mnemonics {
        let k = identity::keys_from_words(&f.words)
            .unwrap_or_else(|e| panic!("não derivou '{}': {e}", f.words));

        assert_eq!(
            k.secret_key().to_secret_hex(),
            f.sk,
            "chave privada diferente da do celular para: {}",
            f.words
        );
        assert_eq!(k.public_key().to_hex(), f.pk, "chave pública diferente");
        assert_eq!(identity::npub(&k.public_key()), f.npub, "npub diferente");
        assert_eq!(identity::nsec(&k), f.nsec, "nsec diferente");
    }
}

/// Entrar com `nsec1…` tem que cair na mesma conta das palavras.
#[test]
fn entrar_por_nsec_cai_na_mesma_conta() {
    let v = vetores();
    for f in &v.mnemonics {
        let por_palavras = identity::keys_from_words(&f.words).unwrap();
        let por_nsec = identity::keys_from_any(&f.nsec).unwrap();
        let por_hex = identity::keys_from_any(&f.sk).unwrap();
        assert_eq!(por_palavras.public_key(), por_nsec.public_key());
        assert_eq!(por_palavras.public_key(), por_hex.public_key());
    }
}

/// O Rust tem que decifrar o que o JavaScript cifrou (NIP-44 v2).
#[test]
fn decifra_o_que_o_celular_cifrou() {
    let v = vetores();
    assert!(!v.nip44.is_empty());

    for c in &v.nip44 {
        let sk = SecretKey::from_hex(&c.de_sk).unwrap();
        let pk = PublicKey::from_hex(&c.para_pk).unwrap();
        let aberto = nip44::decrypt(&sk, &pk, &c.cifrado)
            .unwrap_or_else(|e| panic!("não decifrou: {e}"));
        assert_eq!(aberto, c.texto, "texto decifrado diferente do original");
    }
}

/// E o celular tem que conseguir decifrar o que o Rust cifra: o teste faz a
/// volta completa usando as mesmas chaves dos vetores.
#[test]
fn cifra_de_forma_que_o_celular_entende() {
    let v = vetores();
    let a = SecretKey::from_hex(&v.mnemonics[0].sk).unwrap();
    let b_pk = PublicKey::from_hex(&v.mnemonics[1].pk).unwrap();
    let b = SecretKey::from_hex(&v.mnemonics[1].sk).unwrap();
    let a_pk = PublicKey::from_hex(&v.mnemonics[0].pk).unwrap();

    for texto in ["Oi do computador 💻", "acentuação", &"y".repeat(400)] {
        let cifrado = nip44::encrypt(&a, &b_pk, texto, nip44::Version::V2).unwrap();
        // Quem recebe usa a própria chave secreta e a pública de quem enviou.
        let aberto = nip44::decrypt(&b, &a_pk, &cifrado).unwrap();
        assert_eq!(aberto, texto);
    }
}

/// Um envelope de três camadas montado pelo celular tem que abrir no computador.
#[test]
fn abre_o_envelope_montado_pelo_celular() {
    let v = vetores();
    let destinatario = Keys::new(SecretKey::from_hex(&v.wrap.destinatario_sk).unwrap());
    let evento: Event = serde_json::from_value(v.wrap.evento.clone()).unwrap();

    assert_eq!(evento.kind, Kind::Custom(protocol::KIND_WRAP));

    let rumor = protocol::unwrap_event(&evento, &destinatario)
        .expect("o envelope do celular não abriu no computador");

    let esperado = &v.wrap.rumor_esperado;
    assert_eq!(rumor.content, esperado["content"].as_str().unwrap());
    assert_eq!(rumor.pubkey.to_hex(), esperado["pubkey"].as_str().unwrap());
    assert_eq!(
        rumor.id.unwrap().to_hex(),
        esperado["id"].as_str().unwrap(),
        "o id do rumor mudou: as duas versões calculam o hash de formas diferentes"
    );
    assert_eq!(rumor.kind, Kind::Custom(protocol::KIND_TEXT));
}

/// E o envelope montado pelo computador tem que ter a forma que o celular espera.
#[test]
fn monta_envelope_na_forma_que_o_celular_espera() {
    let v = vetores();
    let ana = Keys::new(SecretKey::from_hex(&v.mnemonics[0].sk).unwrap());
    let beto = Keys::new(SecretKey::from_hex(&v.mnemonics[1].sk).unwrap());

    let rumor = protocol::make_rumor(
        &ana,
        protocol::KIND_TEXT,
        "mensagem do computador",
        vec![Tag::public_key(beto.public_key())],
    );
    let wrap = protocol::wrap_rumor(
        &rumor,
        &ana,
        &beto.public_key(),
        protocol::WrapOptions::default(),
    )
    .unwrap();

    // Forma esperada pelo celular: kind 1059, marca `p` com o destinatário,
    // autor descartável e horário no passado.
    assert_eq!(wrap.kind, Kind::Custom(protocol::KIND_WRAP));
    assert_ne!(wrap.pubkey, ana.public_key());
    let destino = wrap
        .tags
        .iter()
        .find_map(|t| {
            let s = t.as_slice();
            (s.first().map(|x| x.as_str()) == Some("p")).then(|| s.get(1).cloned())?
        })
        .expect("faltou a marca 'p' com o destinatário");
    assert_eq!(destino, beto.public_key().to_hex());
    assert!(wrap.created_at <= Timestamp::now());
    assert!(wrap.verify().is_ok());

    // E abre do outro lado.
    let aberto = protocol::unwrap_event(&wrap, &beto).unwrap();
    assert_eq!(aberto.content, "mensagem do computador");
    assert_eq!(aberto.pubkey, ana.public_key());
}
