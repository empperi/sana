use futures::{SinkExt, StreamExt, Sink, Stream, FutureExt};
use gloo_net::websocket::{futures::WebSocket, Message, WebSocketError};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::types::ChatMessage;
use std::rc::Rc;
use std::cell::RefCell;
use gloo_timers::future::TimeoutFuture;
use uuid::Uuid;
use crate::stomp::{self, StompFrame};
use std::collections::HashSet;

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

type MsgCallback = Callback<(String, ChatMessage)>;
type SysMsgCallback = Callback<(String, String)>;
type ConnectedCallback = Callback<String>;
type StatusCallback = Callback<ConnectionStatus>;

impl WebSocketService {
    pub fn connect(
        on_message: MsgCallback,
        on_system_message: SysMsgCallback,
        on_connected: ConnectedCallback,
        on_status_change: StatusCallback,
    ) -> Self {
        let (tx, mut rx) = futures::channel::mpsc::channel::<String>(100);
        let outgoing_buffer = Rc::new(RefCell::new(Vec::<String>::new()));

        let outgoing_buffer_clone = outgoing_buffer.clone();
        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss:" } else { "ws:" };
            let host = location.host().unwrap();
            let ws_url = format!("{}//{}/ws", protocol, host);
            
            loop {
                on_status_change.emit(ConnectionStatus::Reconnecting);
                if let Ok(ws) = WebSocket::open(&ws_url) {
                    let (write, read) = ws.split();
                    Self::connection_loop(
                        write, read, &mut rx, outgoing_buffer_clone.clone(),
                        &on_message, &on_system_message, &on_connected, &on_status_change
                    ).await;
                }
                on_status_change.emit(ConnectionStatus::Disconnected);
                Self::downtime_drain_loop(&mut rx, outgoing_buffer_clone.clone()).await;
            }
        });

        Self { tx }
    }

    async fn downtime_drain_loop(
        rx: &mut futures::channel::mpsc::Receiver<String>,
        outgoing_buffer: Rc<RefCell<Vec<String>>>
    ) {
        let mut timer = TimeoutFuture::new(2000).fuse();
        loop {
            futures::select! {
                _ = timer => break,
                out_msg = rx.next() => {
                    if let Some(text) = out_msg {
                        outgoing_buffer.borrow_mut().push(text);
                    }
                }
            }
        }
    }

    pub async fn connection_loop<W, R>(
        mut write: W,
        read: R,
        rx: &mut futures::channel::mpsc::Receiver<String>,
        outgoing_buffer: Rc<RefCell<Vec<String>>>,
        on_message: &MsgCallback,
        on_system_message: &SysMsgCallback,
        on_connected: &ConnectedCallback,
        on_status_change: &StatusCallback,
    ) where 
        W: Sink<Message, Error = WebSocketError> + Unpin,
        R: Stream<Item = Result<Message, WebSocketError>> + Unpin,
    {
        let mut read = read.fuse();
        if write.send(Message::Text(stomp::create_connect_frame())).await.is_err() { return; }

        if let Some(username) = Self::wait_for_stomp_connected(&mut read, on_message, on_system_message, on_connected, on_status_change).await {
            on_status_change.emit(ConnectionStatus::Connected);
            on_connected.emit(username);
        } else {
            return;
        }

        if Self::sync_subscriptions(&mut write, &mut read, rx, on_message, on_system_message, on_connected, on_status_change).await.is_err() { return; }
        
        TimeoutFuture::new(500).await;
        if Self::flush_outgoing_buffer(&mut write, outgoing_buffer.clone()).await.is_err() { return; }

        Self::main_message_loop(write, read, rx, outgoing_buffer, on_message, on_system_message, on_connected, on_status_change).await;
    }

    async fn wait_for_stomp_connected<R>(
        read: &mut futures::stream::Fuse<R>,
        on_message: &MsgCallback,
        on_system_message: &SysMsgCallback,
        on_connected: &ConnectedCallback,
        on_status_change: &StatusCallback,
    ) -> Option<String> 
    where R: Stream<Item = Result<Message, WebSocketError>> + Unpin 
    {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            if let Some(frame) = stomp::parse_frame(&text) {
                if let StompFrame::Connected { username } = frame {
                    return Some(username);
                }
                handle_incoming_frame(frame, on_message, on_system_message, on_connected, on_status_change);
            }
        }
        None
    }

    async fn sync_subscriptions<W, R>(
        write: &mut W,
        read: &mut futures::stream::Fuse<R>,
        rx: &mut futures::channel::mpsc::Receiver<String>,
        on_message: &MsgCallback,
        on_system_message: &SysMsgCallback,
        on_connected: &ConnectedCallback,
        on_status_change: &StatusCallback,
    ) -> Result<(), ()>
    where 
        W: Sink<Message, Error = WebSocketError> + Unpin,
        R: Stream<Item = Result<Message, WebSocketError>> + Unpin,
    {
        let mut pending = HashSet::new();
        while let Ok(msg) = rx.try_recv() {
            let (final_msg, receipt_id) = prepare_subscription_frame(msg);
            if let Some(rid) = receipt_id { pending.insert(rid); }
            if write.send(Message::Text(final_msg)).await.is_err() { return Err(()); }
        }

        while !pending.is_empty() {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if let Some(frame) = stomp::parse_frame(&text) {
                        if let StompFrame::Receipt { receipt_id } = frame {
                            pending.remove(&receipt_id);
                        } else {
                            handle_incoming_frame(frame, on_message, on_system_message, on_connected, on_status_change);
                        }
                    }
                }
                _ => return Err(()),
            }
        }
        Ok(())
    }

    async fn flush_outgoing_buffer<W>(
        write: &mut W,
        outgoing_buffer: Rc<RefCell<Vec<String>>>
    ) -> Result<(), ()>
    where W: Sink<Message, Error = WebSocketError> + Unpin
    {
        let mut buffer = outgoing_buffer.borrow_mut();
        for msg in buffer.drain(..) {
            if write.send(Message::Text(msg)).await.is_err() { return Err(()); }
        }
        Ok(())
    }

    async fn main_message_loop<W, R>(
        mut write: W,
        mut read: futures::stream::Fuse<R>,
        rx: &mut futures::channel::mpsc::Receiver<String>,
        outgoing_buffer: Rc<RefCell<Vec<String>>>,
        on_message: &MsgCallback,
        on_system_message: &SysMsgCallback,
        on_connected: &ConnectedCallback,
        on_status_change: &StatusCallback,
    ) where 
        W: Sink<Message, Error = WebSocketError> + Unpin,
        R: Stream<Item = Result<Message, WebSocketError>> + Unpin,
    {
        loop {
            futures::select! {
                msg = read.next() => {
                    if let Some(Ok(Message::Text(text))) = msg {
                        handle_incoming_message(text, on_message, on_system_message, on_connected, on_status_change);
                    } else {
                        break;
                    }
                }
                out_msg = rx.next() => {
                    if let Some(text) = out_msg {
                        if write.send(Message::Text(text.clone())).await.is_err() {
                            outgoing_buffer.borrow_mut().push(text);
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn prepare_subscription_frame(msg: String) -> (String, Option<String>) {
    if !msg.starts_with("SUBSCRIBE") { return (msg, None); }
    
    let last_seen_seq = msg.lines()
        .find(|l| l.starts_with("last_seen_seq:"))
        .map(|l| l.strip_prefix("last_seen_seq:").unwrap().trim().to_string());

    if let Some(dest_line) = msg.lines().find(|l| l.starts_with("destination:")) {
        if let Some(dest) = dest_line.strip_prefix("destination:/topic/") {
            let channel = dest.trim().trim_end_matches('\0');
            let receipt_id = Uuid::new_v4().to_string();
            
            let mut new_frame = format!("SUBSCRIBE\nid:0\ndestination:/topic/{}\nreceipt:{}\n", channel, receipt_id);
            if let Some(lss) = last_seen_seq {
                new_frame.push_str(&format!("last_seen_seq:{}\n", lss));
            }
            new_frame.push_str("\n\0");
            return (new_frame, Some(receipt_id));
        }
    }
    (msg, None)
}

fn handle_incoming_message(
    text: String,
    on_message: &MsgCallback,
    on_system_message: &SysMsgCallback,
    on_connected: &ConnectedCallback,
    on_status_change: &StatusCallback,
) {
    if let Some(frame) = stomp::parse_frame(&text) {
        handle_incoming_frame(frame, on_message, on_system_message, on_connected, on_status_change);
    }
}

fn handle_incoming_frame(
    frame: StompFrame,
    on_message: &MsgCallback,
    on_system_message: &SysMsgCallback,
    on_connected: &ConnectedCallback,
    on_status_change: &StatusCallback,
) {
    match frame {
        StompFrame::Connected { username } => {
            on_status_change.emit(ConnectionStatus::Connected);
            on_connected.emit(username);
        }
        StompFrame::Message { destination, body, seq } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                if channel_name == "system.channels" {
                    on_system_message.emit((channel_name.to_string(), body));
                } else if let Ok(mut chat_msg) = serde_json::from_str::<ChatMessage>(&body) {
                    if seq.is_some() { chat_msg.seq = seq; }
                    on_message.emit((channel_name.to_string(), chat_msg));
                }
            }
        }
        StompFrame::Receipt { .. } => {}
        StompFrame::Error(e) => {
            web_sys::console::error_1(&format!("STOMP Error: {}", e).into());
        }
    }
}

pub struct TestStompClient {
    pub sent_frames: Rc<RefCell<Vec<String>>>,
}

impl StompClient for TestStompClient {
    fn send(&self, frame: String) {
        self.sent_frames.borrow_mut().push(frame);
    }
}
