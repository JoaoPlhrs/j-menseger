//! Áudio, vídeo e captura de tela do J-Menseger.
//!
//! Esta camada é separada do motor de mensagens de propósito: captura de som e
//! codificação de vídeo são trabalho pesado e contínuo, e precisam rodar em
//! threads próprias, longe da interface e da rede.

pub mod audio;
pub mod audiofile;
pub mod builtin;
pub mod camera;
pub mod codec;
pub mod container;
pub mod gif;
pub mod peer;
pub mod photo;
pub mod player;
pub mod recorder;
pub mod screen;
pub mod video;

pub use codec::{AudioQuality, Decoder, Encoder, VoiceDetector, FRAME_SAMPLES, SAMPLE_RATE};
pub mod sysaudio;
