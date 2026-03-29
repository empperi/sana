use futures::{SinkExt, StreamExt, Sink, Stream, FutureExt};
use gloo_net::websocket::{futures::WebSocket, Message, WebSocketError};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use crate::types::ChannelEntry;
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
    stop_tx: futures::channel::mpsc::Sender<()>,
}

impl StompClient for WebSocketService {
    fn send(&self, frame: String) {
        let mut tx = self.tx.clone();
        spawn_local(async move {
            let _ = tx.send(frame).await;
        });
    }
}

type MsgCallback = Callback<(String, ChannelEntry)>;
type SysMsgCallback = Callback<(String, String)>;
type ConnectedCallback = Callback<(String, Uuid)>;
type StatusCallback = Callback<ConnectionStatus>;

impl WebSocketService {
    pub fn connect(
        on_message: MsgCallback,
        on_system_message: SysMsgCallback,
        on_connected: ConnectedCallback,
        on_status_change: StatusCallback,
    ) -> Self {
        let (tx, mut rx) = futures::channel::mpsc::channel::<String>(100);
        let (stop_tx, mut stop_rx) = futures::channel::mpsc::channel::<()>(1);
        let outgoing_buffer = Rc::new(RefCell::new(Vec::<String>::new()));

        let outgoing_buffer_clone = outgoing_buffer.clone();
        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss:" } else { "ws:" };
            let host = location.host().unwrap();
            let ws_url = format!("{}//{}/ws", protocol, host);
            
            let mut attempts = 0;
            const MAX_ATTEMPTS: u32 = 20;
            
            loop {
                // Check if we should stop before trying to connect
                if stop_rx.try_recv().is_ok() { break; }

                if attempts >= MAX_ATTEMPTS {
                    web_sys::console::error_1(&format!("WebSocket: Maximum reconnection attempts ({}) reached. Stopping.", MAX_ATTEMPTS).into());
                    on_status_change.emit(ConnectionStatus::Disconnected);
                    
                    // Wait for manual stop or just hang until component unmounts
                    while stop_rx.next().await.is_none() {
                        TimeoutFuture::new(1000).await;
                    }
                    break;
                }

                on_status_change.emit(ConnectionStatus::Reconnecting);
                let mut connection_result = false;
                let mut connection_stopped = false;

                if let Ok(ws) = WebSocket::open(&ws_url) {
                    let (write, read) = ws.split();
                    
                    futures::select! {
                        res = Self::connection_loop(
                            write, read, &mut rx, outgoing_buffer_clone.clone(),
                            &on_message, &on_system_message, &on_connected, &on_status_change
                        ).fuse() => {
                            connection_result = res;
                        },
                        _ = stop_rx.next() => {
                            connection_stopped = true;
                        }
                    }
                }
                
                if connection_stopped { break; }

                if connection_result {
                    attempts = 0;
                } else {
                    on_status_change.emit(ConnectionStatus::Disconnected);
                    attempts += 1;
                    let backoff_ms = (2u32.pow(attempts.min(4)) * 1000).min(30000); // 2s, 4s, 8s, 16s, 30s, 30s...
                    
                    web_sys::console::debug_1(&format!("WebSocket: Reconnection attempt {} failed. Retrying in {}ms...", attempts, backoff_ms).into());

                    // Wait backoff or stop
                    let mut timer = TimeoutFuture::new(backoff_ms).fuse();
                    let mut stopped = false;
                    loop {
                        futures::select! {
                            _ = timer => break,
                            _ = stop_rx.next() => {
                                stopped = true;
                                break;
                            }
                            out_msg = rx.next() => {
                                if let Some(text) = out_msg {
                                    outgoing_buffer_clone.borrow_mut().push(text);
                                }
                            }
                        }
                    }
                    if stopped { break; }
                }
            }
        });

        Self { tx, stop_tx }
    }

    pub fn stop(&self) {
        let mut stop_tx = self.stop_tx.clone();
        spawn_local(async move {
            let _ = stop_tx.send(()).await;
        });
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
    ) -> bool where 
        W: Sink<Message, Error = WebSocketError> + Unpin,
        R: Stream<Item = Result<Message, WebSocketError>> + Unpin,
    {
        let mut read = read.fuse();
        if write.send(Message::Text(stomp::create_connect_frame())).await.is_err() { return false; }

        if let Some((username, user_id)) = Self::wait_for_stomp_connected(&mut read, on_message, on_system_message, on_connected, on_status_change).await {
            on_status_change.emit(ConnectionStatus::Connected);
            on_connected.emit((username, user_id));
        } else {
            return false;
        }

        if Self::sync_subscriptions(&mut write, &mut read, rx, outgoing_buffer.clone(), on_message, on_system_message, on_connected, on_status_change).await.is_err() { return true; }
        
        TimeoutFuture::new(500).await;
        if Self::flush_outgoing_buffer(&mut write, outgoing_buffer.clone()).await.is_err() { return true; }

        Self::main_message_loop(write, read, rx, outgoing_buffer, on_message, on_system_message, on_connected, on_status_change).await;
        true
    }

    async fn wait_for_stomp_connected<R>(
        read: &mut futures::stream::Fuse<R>,
        on_message: &MsgCallback,
        on_system_message: &SysMsgCallback,
        on_connected: &ConnectedCallback,
        on_status_change: &StatusCallback,
    ) -> Option<(String, Uuid)> 
    where R: Stream<Item = Result<Message, WebSocketError>> + Unpin 
    {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            if let Some(frame) = stomp::parse_frame(&text) {
                if let StompFrame::Connected { username, user_id } = frame {
                    return Some((username, user_id));
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
        outgoing_buffer: Rc<RefCell<Vec<String>>>,
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
            if msg.starts_with("SUBSCRIBE") {
                let (final_msg, receipt_id) = prepare_subscription_frame(msg);
                if let Some(rid) = receipt_id { pending.insert(rid); }
                if write.send(Message::Text(final_msg)).await.is_err() { return Err(()); }
            } else {
                outgoing_buffer.borrow_mut().push(msg);
            }
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

    pub async fn flush_outgoing_buffer<W>(
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

pub fn prepare_subscription_frame(msg: String) -> (String, Option<String>) {
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
        StompFrame::Connected { username, user_id } => {
            on_status_change.emit(ConnectionStatus::Connected);
            on_connected.emit((username, user_id));
        }
        StompFrame::Message { destination, body, seq } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                if channel_name == "system.channels" {
                    on_system_message.emit((channel_name.to_string(), body));
                } else {
                    match serde_json::from_str::<ChannelEntry>(&body) {
                        Ok(mut entry) => {
                            if let ChannelEntry::Message(ref mut chat_msg) = entry {
                                if seq.is_some() { chat_msg.seq = seq; }
                            }
                            on_message.emit((channel_name.to_string(), entry));
                        }
                        Err(e) => {
                            web_sys::console::error_1(&format!("Failed to deserialize ChannelEntry: {}. Body: {}", e, body).into());
                        }
                    }
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
