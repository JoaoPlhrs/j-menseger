//! Finge ser o Spotify no D-Bus (MPRIS), para testar o "Ouvindo Spotify"
//! sem o Spotify instalado. Só para testes no Linux.
//!
//! cargo run --example spotify_de_teste -p jm-app -- "Música" "Artista" "Álbum" 200

#[cfg(target_os = "linux")]
fn main() -> zbus::Result<()> {
    use std::collections::HashMap;
    use zbus::zvariant::{ObjectPath, OwnedValue, Value};

    struct Player {
        meta: HashMap<String, OwnedValue>,
    }

    #[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
    impl Player {
        #[zbus(property)]
        fn playback_status(&self) -> String {
            "Playing".into()
        }
        #[zbus(property)]
        fn metadata(&self) -> HashMap<String, OwnedValue> {
            self.meta.iter().map(|(k, v)| (k.clone(), v.try_clone().unwrap())).collect()
        }
    }

    let a: Vec<String> = std::env::args().skip(1).collect();
    let titulo = a.first().cloned().unwrap_or_else(|| "Tempo Perdido".into());
    let artista = a.get(1).cloned().unwrap_or_else(|| "Legião Urbana".into());
    let album = a.get(2).cloned().unwrap_or_else(|| "Dois".into());
    let segs: u64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(301);
    let mut meta: HashMap<String, OwnedValue> = HashMap::new();
    meta.insert("xesam:title".into(), Value::from(titulo).try_into().unwrap());
    meta.insert("xesam:artist".into(), Value::from(vec![artista]).try_into().unwrap());
    meta.insert("xesam:album".into(), Value::from(album).try_into().unwrap());
    meta.insert("mpris:length".into(), Value::from(segs * 1_000_000).try_into().unwrap());
    meta.insert("mpris:trackid".into(), Value::from(ObjectPath::try_from("/com/spotify/track/4uLU6hMCjMI75M1A2tKUQC").unwrap()).try_into().unwrap());
    let _con = zbus::blocking::connection::Builder::session()?
        .name("org.mpris.MediaPlayer2.spotify")?
        .serve_at("/org/mpris/MediaPlayer2", Player { meta })?
        .build()?;
    println!("Spotify de teste no ar");
    loop {
        std::thread::park();
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
