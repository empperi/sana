use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::types::ChatMessage;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

pub struct WebSocketService {
    tx: futures::channel::mpsc::Sender<String>,
}

impl WebSocketService {
    pub fn new(
        on_message: Callback<String>,
        on_connected: Callback<String>,
    ) -> Self {
        let (tx, mut rx) = futures::channel::mpsc::channel::<String>(10);

        spawn_local(async move {
            let ws_url = "ws://localhost:3000/ws";
            let ws = WebSocket::open(ws_url).unwrap();
            let (mut write, read) = ws.split();
            let mut read = read.fuse();

            write.send(Message::Text("CONNECT\naccept-version:1.2\n\n\0".to_string())).await.unwrap();

            loop {
                futures::select! {
                    msg = read.next() => {
                        if let Some(Ok(Message::Text(text))) = msg {
                            if text.starts_with("CONNECTED") {
                                on_connected.emit(text);
                            } else if text.starts_with("MESSAGE") {
                                on_message.emit(text);
                            }
                        }
                    }
                    out_msg = rx.next() => {
                        if let Some(text) = out_msg {
                            write.send(Message::Text(text)).await.unwrap();
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub fn send(&mut self, msg: String) {
        let mut tx = self.tx.clone();
        spawn_local(async move {
            let _ = tx.send(msg).await;
        });
    }
}
