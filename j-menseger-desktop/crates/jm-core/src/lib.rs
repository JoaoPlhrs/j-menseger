//! Núcleo do J-Menseger: protocolo cifrado, servidores, identidade e armazenamento.
//!
//! Este módulo não conhece a interface e roda sem janela, o que permite testá-lo
//! com clientes reais em linha de comando.

pub mod engine;
pub mod groups;
pub mod identity;
pub mod media;
pub mod messages;
pub mod pin;
pub mod protocol;
pub mod relays;
pub mod safety;
pub mod shortcode;
pub mod sounds;
pub mod store;

pub use protocol::{DEFAULT_RELAYS, CHUNK};
