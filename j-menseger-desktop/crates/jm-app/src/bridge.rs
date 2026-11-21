//! Ponte entre o canal de avisos do motor e a fila de mensagens da interface.
//!
//! O motor só existe depois que a conta é aberta, então o fluxo de avisos é
//! instalado nesse momento e guardado aqui para a interface consumir.

use jm_core::engine::EngineEvent;
use std::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

static PENDING: Mutex<Option<UnboundedReceiver<EngineEvent>>> = Mutex::new(None);
static SINK: Mutex<Option<UnboundedSender<EngineEvent>>> = Mutex::new(None);

/// Guarda o canal recém-criado pelo motor.
pub fn install(rx: UnboundedReceiver<EngineEvent>) {
    if let Some(tx) = SINK.lock().unwrap().clone() {
        // A interface já está escutando: repassa para ela.
        let mut rx = rx;
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                if tx.send(ev).is_err() {
                    break;
                }
            }
        });
        return;
    }
    *PENDING.lock().unwrap() = Some(rx);
}

/// Registra para onde os avisos devem ir e entrega o que já estava guardado.
pub fn subscribe(tx: UnboundedSender<EngineEvent>) {
    if let Some(mut rx) = PENDING.lock().unwrap().take() {
        let t = tx.clone();
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                if t.send(ev).is_err() {
                    break;
                }
            }
        });
    }
    *SINK.lock().unwrap() = Some(tx);
}
