//! Websocket accept server

use async_std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    task,
};
use async_tungstenite::{async_std::connect_async, tungstenite::Message};
use futures::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::env::RequestEnv;
use serde_json;

use libqaul_rpc::Envelope;
use libqaul::Qaul;

#[cfg(feature = "chat")]
use qaul_chat::Chat;

/// Websocket server structure
pub struct WsServer {
    running: AtomicBool,
    addr: String,

    qaul: Arc<Qaul>,
    #[cfg(feature = "chat")]
    chat: Arc<Chat>,
}

impl WsServer {
    /// Create a websocket server with a libqaul instance and services
    #[cfg(all(feature = "chat", feature = "voice", feature = "files"))]
    pub fn new<S: Into<String>>(addr: S, qaul: Arc<Qaul>, chat: Arc<Chat>) -> Arc<Self> {
        Arc::new(Self {
            running: AtomicBool::from(true),
            addr: addr.into(),
            qaul,
            chat,
        })
    }

    /// Accept connections in a detached task
    pub fn run(self: Arc<Self>) {
        task::spawn(async move {
            while self.running.load(Ordering::Relaxed) {
                let socket = TcpListener::bind(&self.addr)
                    .await
                    .expect(&format!("Failed to bind; '{}'", &self.addr));

                while let Ok((stream, _)) = socket.accept().await {
                    task::spawn(Arc::clone(&self).handle(stream));
                }
            }
        });
    }

    /// Handle an incoming websocket stream
    async fn handle(self: Arc<Self>, stream: TcpStream) {
        let ws_stream = async_tungstenite::accept_async(stream)
            .await
            .expect("Failed ws handshake");

        let mut buf = String::new();
        let (mut tx, mut rx) = ws_stream.split();

        // Read messages from this stream
        while let Some(Ok(Message::Text(msg))) = rx.next().await {
            let je: RequestEnv = serde_json::from_str(&msg).expect("Malformed json envelope");
            let env: Envelope = je.into();
            
            
        }

        // while let Ok(num) = rx.read_to_string(&mut buf).await {

        // }
    }

    /// Signal the runner to shut down
    pub fn stop(&self) {
        self.running.swap(false, Ordering::Relaxed);
    }
}
