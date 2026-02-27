use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::types::ChatMessage;
use std::rc::Rc;
use std::cell::RefCell;
use gloo_timers::future::TimeoutFuture;
use crate::stomp::{self, StompFrame};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
}

pub trait StompClient {
    fn send(&self, frame: String);
}

pub struct WebSocketService {
    tx: futures::channel::mpsc::Sender<String>,
}

impl StompClient for WebSocketService {
    fn send(&self, frame: String) {
        let mut tx = self.tx.clone();
        spawn_local(async move {
            let _ = tx.send(frame).await;
        });
    }
}

impl WebSocketService {
    pub fn connect(
        on_message: Callback<(String, ChatMessage)>,
        on_system_message: Callback<(String, String)>,
        on_connected: Callback<String>,
        on_status_change: Callback<ConnectionStatus>,
    ) -> Self {
        let (tx, mut rx) = futures::channel::mpsc::channel::<String>(100);
        let outgoing_buffer = Rc::new(RefCell::new(Vec::<String>::new()));

        let outgoing_buffer_clone = outgoing_buffer.clone();
        spawn_local(async move {
            let ws_url = "ws://localhost:3000/ws";
            
            loop {
                on_status_change.emit(ConnectionStatus::Reconnecting);
                
                let ws_result = WebSocket::open(ws_url);
                
                if let Ok(ws) = ws_result {
                    on_status_change.emit(ConnectionStatus::Connected);
                    let (mut write, read) = ws.split();
                    let mut read = read.fuse();

                    // Send STOMP CONNECT frame
                    if let Err(_) = write.send(Message::Text(stomp::create_connect_frame())).await {
                        on_status_change.emit(ConnectionStatus::Disconnected);
                        TimeoutFuture::new(2000).await;
                        continue;
                    }

                    // Flush buffer
                    {
                        let mut buffer = outgoing_buffer_clone.borrow_mut();
                        for msg in buffer.drain(..) {
                            if let Err(_) = write.send(Message::Text(msg)).await {
                                break;
                            }
                        }
                    }

                    let mut disconnected = false;
                    while !disconnected {
                        futures::select! {
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        handle_incoming_message(text, &on_message, &on_system_message, &on_connected);
                                    }
                                    _ => {
                                        disconnected = true;
                                    }
                                }
                            }
                            out_msg = rx.next() => {
                                if let Some(text) = out_msg {
                                    if let Err(_) = write.send(Message::Text(text.clone())).await {
                                        outgoing_buffer_clone.borrow_mut().push(text);
                                        disconnected = true;
                                    }
                                }
                            }
                        }
                    }
                }

                on_status_change.emit(ConnectionStatus::Disconnected);
                TimeoutFuture::new(2000).await;
            }
        });

        Self { tx }
    }
}

fn handle_incoming_message(
    text: String,
    on_message: &Callback<(String, ChatMessage)>,
    on_system_message: &Callback<(String, String)>,
    on_connected: &Callback<String>,
) {
    if let Some(frame) = stomp::parse_frame(&text) {
        match frame {
            StompFrame::Connected { username } => {
                on_connected.emit(username);
            }
            StompFrame::Message { destination, body } => {
                if let Some(channel_name) = destination.strip_prefix("/topic/") {
                    if channel_name == "system.channels" {
                        on_system_message.emit((channel_name.to_string(), body));
                    } else if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&body) {
                        on_message.emit((channel_name.to_string(), chat_msg));
                    }
                }
            }
            StompFrame::Error(e) => {
                web_sys::console::error_1(&format!("STOMP Error: {}", e).into());
            }
        }
    }
}

// For unit testing without mocking libraries
pub struct TestStompClient {
    pub sent_frames: Rc<RefCell<Vec<String>>>,
}

impl StompClient for TestStompClient {
    fn send(&self, frame: String) {
        self.sent_frames.borrow_mut().push(frame);
    }
}
