use yew::prelude::*;
use uuid::Uuid;
use chrono::Utc;
use std::rc::Rc;
use std::cell::RefCell;

use frontend::components::sidebar::Sidebar;
use frontend::components::chat_window::ChatWindow;
use frontend::services::websocket::{WebSocketService, ConnectionStatus, StompClient};
use frontend::types::ChatMessage;
use frontend::logic::ChatState;
use frontend::stomp;

#[function_component(App)]
fn app() -> Html {
    let chat_state = use_state(ChatState::new);
    let ws_service = use_mut_ref(|| None::<Rc<WebSocketService>>);
    let state_ref = use_mut_ref(ChatState::new);

    // Sync ref with state
    {
        let state_ref = state_ref.clone();
        use_effect_with(chat_state.clone(), move |chat_state| {
            *state_ref.borrow_mut() = (**chat_state).clone();
            || {}
        });
    }

    // Initialize WebSocket
    {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service_ref = ws_service.clone();

        use_effect_with((), move |_| {
            let on_message = create_on_message_callback(chat_state.clone(), state_ref.clone());
            let on_system_message = create_on_system_message_callback(chat_state.clone(), state_ref.clone(), ws_service_ref.clone());
            let on_connected = create_on_connected_callback(chat_state.clone(), state_ref.clone());
            let on_status_change = create_on_status_change_callback(chat_state.clone(), state_ref.clone(), ws_service_ref.clone());

            let service = Rc::new(WebSocketService::connect(on_message, on_system_message, on_connected, on_status_change));
            *ws_service_ref.borrow_mut() = Some(service);
            || {}
        });
    }

    let on_switch_channel = {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        Callback::from(move |channel: String| {
            let mut state = (*state_ref.borrow()).clone();
            state.switch_channel(channel);
            *state_ref.borrow_mut() = state.clone();
            chat_state.set(state);
        })
    };

    let on_create_channel = {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service = ws_service.clone();
        let on_switch_channel = on_switch_channel.clone();
        Callback::from(move |name: String| {
            let mut state = (*state_ref.borrow()).clone();
            if !state.channels.contains(&name) {
                state.channels.push(name.clone());
                if let Some(service) = &*ws_service.borrow() {
                    service.send(stomp::create_subscribe_frame(&name, None, None));
                }
                *state_ref.borrow_mut() = state.clone();
                chat_state.set(state);
                on_switch_channel.emit(name);
            }
        })
    };

    let on_send_message = {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service = ws_service.clone();
        Callback::from(move |text: String| {
            handle_send_message(text, &chat_state, &state_ref, &ws_service);
        })
    };

    render_app(&chat_state, on_switch_channel, on_create_channel, on_send_message)
}

fn create_on_message_callback(chat_state: UseStateHandle<ChatState>, state_ref: Rc<RefCell<ChatState>>) -> Callback<(String, ChatMessage)> {
    Callback::from(move |(channel, msg)| {
        let mut state = (*state_ref.borrow()).clone();
        state.handle_message(channel, msg);
        *state_ref.borrow_mut() = state.clone();
        chat_state.set(state);
    })
}

fn create_on_system_message_callback(
    chat_state: UseStateHandle<ChatState>, 
    state_ref: Rc<RefCell<ChatState>>, 
    ws_service_ref: Rc<RefCell<Option<Rc<WebSocketService>>>>
) -> Callback<(String, String)> {
    Callback::from(move |(_topic, body): (String, String)| {
        let mut state = (*state_ref.borrow()).clone();
                    if !state.channels.contains(&body) {
                        state.handle_system_message(body.clone());
                        if let Some(service) = &*ws_service_ref.borrow() {
                            service.send(stomp::create_subscribe_frame(&body, None, None));
                        }
                        *state_ref.borrow_mut() = state.clone();
                        chat_state.set(state);
                    }
        
    })
}

fn create_on_connected_callback(chat_state: UseStateHandle<ChatState>, state_ref: Rc<RefCell<ChatState>>) -> Callback<String> {
    Callback::from(move |username| {
        let mut state = (*state_ref.borrow()).clone();
        state.set_username(username);
        *state_ref.borrow_mut() = state.clone();
        chat_state.set(state);
    })
}

fn create_on_status_change_callback(
    chat_state: UseStateHandle<ChatState>, 
    state_ref: Rc<RefCell<ChatState>>, 
    ws_service_ref: Rc<RefCell<Option<Rc<WebSocketService>>>>
) -> Callback<ConnectionStatus> {
    Callback::from(move |status| {
        let mut state = (*state_ref.borrow()).clone();
        state.set_connection_status(status);
        if status == ConnectionStatus::Connected {
            if let Some(service) = &*ws_service_ref.borrow() {
                sync_subscriptions_on_connect(service, &state);
            }
        }
        *state_ref.borrow_mut() = state.clone();
        chat_state.set(state);
    })
}

fn sync_subscriptions_on_connect(service: &Rc<WebSocketService>, state: &ChatState) {
    service.send(stomp::create_subscribe_frame("system.channels", None, None));
    for (channel, msgs) in &state.messages {
        let last_seq = msgs.iter().rev().find_map(|m| m.seq);
        service.send(stomp::create_subscribe_frame(channel, None, last_seq));
    }
    for channel in &state.channels {
        if !state.messages.contains_key(channel) {
            service.send(stomp::create_subscribe_frame(channel, None, None));
        }
    }
}

fn handle_send_message(
    text: String, 
    chat_state: &UseStateHandle<ChatState>, 
    state_ref: &Rc<RefCell<ChatState>>, 
    ws_service: &Rc<RefCell<Option<Rc<WebSocketService>>>>
) {
    let mut state = (*state_ref.borrow()).clone();
    let message_id = Uuid::new_v4().to_string();
    let channel_name = state.current_channel.clone();

    let pending_msg = ChatMessage {
        id: message_id.clone(),
        user: state.username.clone(),
        timestamp: Utc::now().timestamp_millis(),
        message: text.clone(),
        pending: true,
        seq: None,
    };

    state.add_pending_message(channel_name.clone(), pending_msg);
    if let Some(service) = &*ws_service.borrow() {
        service.send(stomp::create_send_frame(&channel_name, &message_id, &text));
    }
    *state_ref.borrow_mut() = state.clone();
    chat_state.set(state);
}

fn render_app(
    chat_state: &UseStateHandle<ChatState>,
    on_switch_channel: Callback<String>,
    on_create_channel: Callback<String>,
    on_send_message: Callback<String>
) -> Html {
    let messages = chat_state.messages
        .get(&chat_state.current_channel)
        .cloned()
        .unwrap_or_default();

    html! {
        <div class="app-container">
            <Sidebar
                channels={chat_state.channels.clone()}
                current_channel={chat_state.current_channel.clone()}
                unread_channels={chat_state.unread_channels.clone()}
                connection_status={chat_state.connection_status}
                on_switch_channel={on_switch_channel}
                on_create_channel={on_create_channel}
            />
            <ChatWindow
                current_channel={chat_state.current_channel.clone()}
                messages={messages}
                current_username={chat_state.username.clone()}
                on_send_message={on_send_message}
            />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
