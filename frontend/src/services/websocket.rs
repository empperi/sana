use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::types::ChatMessage;

pub struct WebSocketService {
    tx: futures::channel::mpsc::Sender<String>,
}

impl WebSocketService {
    pub fn connect(
        on_message: Callback<(String, ChatMessage)>,
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
                            handle_incoming_message(text, &on_message, &on_connected);
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

fn handle_incoming_message(
    text: String,
    on_message: &Callback<(String, ChatMessage)>,
    on_connected: &Callback<String>,
) {
    if text.starts_with("CONNECTED") {
        handle_connected_frame(&text, on_connected);
    } else if text.starts_with("MESSAGE") {
        handle_message_frame(&text, on_message);
    }
}

fn handle_connected_frame(text: &str, on_connected: &Callback<String>) {
    for line in text.lines() {
        if line.starts_with("username:") {
            let username = line.strip_prefix("username:").unwrap().trim().to_string();
            on_connected.emit(username);
        }
    }
}

fn handle_message_frame(text: &str, on_message: &Callback<(String, ChatMessage)>) {
    let (destination, body) = parse_message_frame(text);

    if let Some(channel_name) = destination.strip_prefix("/topic/") {
        if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&body) {
            on_message.emit((channel_name.to_string(), chat_msg));
        }
    }
}

fn parse_message_frame(text: &str) -> (String, String) {
    let mut destination = String::new();
    let mut body_start = false;
    let mut body = String::new();

    let lines: Vec<&str> = text.lines().collect();
    for line in lines.iter() {
        if line.is_empty() {
            body_start = true;
            continue;
        }
        if !body_start {
            if line.starts_with("destination:") {
                destination = line.strip_prefix("destination:").unwrap().trim().to_string();
            }
        }
    }

    let parts: Vec<&str> = text.split("\n\n").collect();
    if parts.len() > 1 {
        body = parts[1].trim_end_matches('\0').to_string();
    }

    (destination, body)
}
