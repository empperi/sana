use yew::prelude::*;
use uuid::Uuid;
use chrono::Utc;
use std::rc::Rc;

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
    
    // For stale closures, we still need refs for parts of the state we access in callbacks
    let state_ref = use_mut_ref(ChatState::new);

    // Sync ref with state
    {
        let state_ref = state_ref.clone();
        use_effect_with(chat_state.clone(), move |chat_state| {
            *state_ref.borrow_mut() = (**chat_state).clone();
            || {}
        });
    }

    {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service_ref = ws_service.clone();

        use_effect_with((), move |_| {
            let on_message = {
                let chat_state = chat_state.clone();
                let state_ref = state_ref.clone();
                Callback::from(move |(channel, msg): (String, ChatMessage)| {
                    let mut state = (*state_ref.borrow()).clone();
                    state.handle_message(channel, msg);
                    *state_ref.borrow_mut() = state.clone();
                    chat_state.set(state);
                })
            };

            let on_system_message = {
                let chat_state = chat_state.clone();
                let state_ref = state_ref.clone();
                let ws_service_ref = ws_service_ref.clone();
                Callback::from(move |(_topic, body): (String, String)| {
                    let mut state = (*state_ref.borrow()).clone();
                    if !state.channels.contains(&body) {
                        state.handle_system_message(body.clone());
                        
                        if let Some(service) = &*ws_service_ref.borrow() {
                            service.send(stomp::create_subscribe_frame(&body));
                        }
                        
                        *state_ref.borrow_mut() = state.clone();
                        chat_state.set(state);
                    }
                })
            };

            let on_connected = {
                let chat_state = chat_state.clone();
                let state_ref = state_ref.clone();
                Callback::from(move |username: String| {
                    let mut state = (*state_ref.borrow()).clone();
                    state.set_username(username);
                    *state_ref.borrow_mut() = state.clone();
                    chat_state.set(state);
                })
            };

            let on_status_change = {
                let chat_state = chat_state.clone();
                let state_ref = state_ref.clone();
                let ws_service_ref = ws_service_ref.clone();

                Callback::from(move |status: ConnectionStatus| {
                    let mut state = (*state_ref.borrow()).clone();
                    state.set_connection_status(status);
                    
                    if status == ConnectionStatus::Connected {
                        if let Some(service) = &*ws_service_ref.borrow() {
                            service.send(stomp::create_subscribe_frame("system.channels"));
                            for channel in &state.channels {
                                service.send(stomp::create_subscribe_frame(channel));
                            }
                        }
                    }
                    
                    *state_ref.borrow_mut() = state.clone();
                    chat_state.set(state);
                })
            };

            let service = Rc::new(WebSocketService::connect(on_message, on_system_message, on_connected, on_status_change));
            *ws_service_ref.borrow_mut() = Some(service.clone());

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
                    service.send(stomp::create_subscribe_frame(&name));
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
            let mut state = (*state_ref.borrow()).clone();
            let message_id = Uuid::new_v4().to_string();
            let channel_name = state.current_channel.clone();

            let pending_msg = ChatMessage {
                id: message_id.clone(),
                user: state.username.clone(),
                timestamp: Utc::now().timestamp_millis(),
                message: text.clone(),
                pending: true,
            };

            state.add_pending_message(channel_name.clone(), pending_msg);

            if let Some(service) = &*ws_service.borrow() {
                service.send(stomp::create_send_frame(&channel_name, &message_id, &text));
            }

            *state_ref.borrow_mut() = state.clone();
            chat_state.set(state);
        })
    };

    let messages_for_current_channel = chat_state.messages
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
                messages={messages_for_current_channel}
                current_username={chat_state.username.clone()}
                on_send_message={on_send_message}
            />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
