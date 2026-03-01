use yew::prelude::*;
use yew_router::prelude::*;
use uuid::Uuid;
use chrono::Utc;
use std::rc::Rc;
use std::cell::RefCell;
use gloo_net::http::Request;
use web_sys::RequestCredentials;

use frontend::components::sidebar::Sidebar;
use frontend::components::chat_window::ChatWindow;
use frontend::components::auth::{Login, Register};
use frontend::components::join_channel_modal::JoinChannelModal;
use frontend::services::websocket::{WebSocketService, ConnectionStatus, StompClient};
use frontend::types::{ChatMessage, ChannelEntry};
use frontend::logic::ChatState;
use frontend::stomp;
use frontend::Route;

async fn fetch_channels() -> Result<Vec<frontend::types::Channel>, String> {
    let response = Request::get("/api/channels")
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 200 {
        response.json::<Vec<frontend::types::Channel>>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("Failed to fetch channels: {}", response.status()))
    }
}

#[function_component(ChatApp)]
pub fn chat_app() -> Html {
    let navigator = use_navigator().unwrap();
    let auth_check_done = use_state(|| false);

    // Initial Auth Check
    {
        let navigator = navigator.clone();
        let auth_check_done = auth_check_done.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let resp = Request::get(&frontend::get_api_url("/api/auth/me"))
                    .credentials(RequestCredentials::Include)
                    .send()
                    .await;
                match resp {
                    Ok(r) if r.status() == 200 => {
                        let content_type = r.headers().get("content-type").unwrap_or_default();
                        if content_type.contains("application/json") {
                            auth_check_done.set(true);
                        } else {
                            navigator.push(&Route::Login);
                        }
                    }
                    _ => {
                        navigator.push(&Route::Login);
                    }
                }
            });
            || {}
        });
    }

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

    let is_join_modal_open = use_state(|| false);
    let is_modal_create_mode = use_state(|| false);

    // Initialize WebSocket
    {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service_ref = ws_service.clone();
        let auth_check_done_val = *auth_check_done;

        use_effect_with(auth_check_done_val, move |&done| {
            if done {
                let on_message = create_on_message_callback(chat_state.clone(), state_ref.clone());
                let on_system_message = create_on_system_message_callback(chat_state.clone(), state_ref.clone(), ws_service_ref.clone());
                let on_connected = create_on_connected_callback(chat_state.clone(), state_ref.clone());
                let on_status_change = create_on_status_change_callback(chat_state.clone(), state_ref.clone(), ws_service_ref.clone());

                let service = Rc::new(WebSocketService::connect(on_message, on_system_message, on_connected, on_status_change));
                *ws_service_ref.borrow_mut() = Some(service);
            }
            
            let ws_service_ref = ws_service_ref.clone();
            move || {
                if let Some(service) = ws_service_ref.borrow_mut().take() {
                    service.stop();
                }
            }
        });
    }

    // Fetch channels from database
    {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service_ref = ws_service.clone();
        let auth_check_done_val = *auth_check_done;

        use_effect_with(auth_check_done_val, move |&done| {
            if done {
                wasm_bindgen_futures::spawn_local(async move {
                    match fetch_channels().await {
                        Ok(channels) => {
                            let mut state = (*state_ref.borrow()).clone();
                            state.set_channels(channels);
                            
                            // Subscribe to all channels
                            if let Some(service) = &*ws_service_ref.borrow() {
                                for channel in &state.channels {
                                    service.send(stomp::create_subscribe_frame(channel, None, None));
                                }
                            }
                            
                            *state_ref.borrow_mut() = state.clone();
                            chat_state.set(state);
                        },
                        Err(e) => {
                            gloo_console::error!(format!("Failed to fetch channels: {}", e));
                        }
                    }
                });
            }
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
        let is_join_modal_open = is_join_modal_open.clone();
        Callback::from(move |name: String| {
            let chat_state = chat_state.clone();
            let state_ref = state_ref.clone();
            let ws_service = ws_service.clone();
            let on_switch_channel = on_switch_channel.clone();
            let is_join_modal_open = is_join_modal_open.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let payload = serde_json::json!({
                    "name": name
                });

                let resp = Request::post("/api/channels")
                    .credentials(RequestCredentials::Include)
                    .json(&payload)
                    .unwrap()
                    .send()
                    .await;

                if let Ok(r) = resp {
                    if r.status() == 201 {
                        if let Ok(channel) = r.json::<frontend::types::Channel>().await {
                            let mut state = (*state_ref.borrow()).clone();
                            state.join_channel(channel.clone());
                            
                            if let Some(service) = &*ws_service.borrow() {
                                service.send(stomp::create_subscribe_frame(&channel.name, None, None));
                            }

                            *state_ref.borrow_mut() = state.clone();
                            chat_state.set(state);
                            is_join_modal_open.set(false);
                            on_switch_channel.emit(channel.name);
                        }
                    }
                }
            });
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

    let on_open_join_modal = {
        let is_join_modal_open = is_join_modal_open.clone();
        let is_modal_create_mode = is_modal_create_mode.clone();
        Callback::from(move |create_mode: bool| {
            is_modal_create_mode.set(create_mode);
            is_join_modal_open.set(true);
        })
    };

    let on_close_join_modal = {
        let is_join_modal_open = is_join_modal_open.clone();
        Callback::from(move |_| is_join_modal_open.set(false))
    };

    let on_join_channel = {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service = ws_service.clone();
        let is_join_modal_open = is_join_modal_open.clone();
        let on_switch_channel = on_switch_channel.clone();

        Callback::from(move |channel: frontend::types::Channel| {
            let chat_state = chat_state.clone();
            let state_ref = state_ref.clone();
            let ws_service = ws_service.clone();
            let is_join_modal_open = is_join_modal_open.clone();
            let on_switch_channel = on_switch_channel.clone();
            let channel_to_join = channel.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let payload = serde_json::json!({
                    "channel_id": channel_to_join.id
                });

                let resp = Request::post("/api/channels/join")
                    .credentials(RequestCredentials::Include)
                    .json(&payload)
                    .unwrap()
                    .send()
                    .await;

                if let Ok(r) = resp {
                    if r.status() == 200 {
                        let mut state = (*state_ref.borrow()).clone();
                        state.join_channel(channel_to_join.clone());
                        
                        if let Some(service) = &*ws_service.borrow() {
                            service.send(stomp::create_subscribe_frame(&channel_to_join.name, None, None));
                        }

                        *state_ref.borrow_mut() = state.clone();
                        chat_state.set(state);
                        is_join_modal_open.set(false);
                        on_switch_channel.emit(channel_to_join.name);
                    }
                }
            });
        })
    };

    let is_mobile_sidebar_open = use_state(|| false);

    let on_toggle_sidebar = {
        let is_mobile_sidebar_open = is_mobile_sidebar_open.clone();
        Callback::from(move |_| {
            let current = *is_mobile_sidebar_open;
            is_mobile_sidebar_open.set(!current);
        })
    };

    let on_close_sidebar = {
        let is_mobile_sidebar_open = is_mobile_sidebar_open.clone();
        Callback::from(move |_| is_mobile_sidebar_open.set(false))
    };

    if !*auth_check_done {
        return html! { <div>{ "Loading..." }</div> };
    }

    render_app(
        &chat_state, 
        on_switch_channel, 
        on_create_channel, 
        on_send_message,
        on_open_join_modal,
        *is_join_modal_open,
        *is_modal_create_mode,
        on_close_join_modal,
        on_join_channel,
        *is_mobile_sidebar_open,
        on_toggle_sidebar,
        on_close_sidebar
    )
}

fn create_on_message_callback(chat_state: UseStateHandle<ChatState>, state_ref: Rc<RefCell<ChatState>>) -> Callback<(String, ChannelEntry)> {
    Callback::from(move |(channel, entry)| {
        let mut state = (*state_ref.borrow()).clone();
        state.handle_message(channel, entry);
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
        if let Some(channel_name) = state.handle_system_message(body) {
            if let Some(service) = &*ws_service_ref.borrow() {
                service.send(stomp::create_subscribe_frame(&channel_name, None, None));
            }
            *state_ref.borrow_mut() = state.clone();
            chat_state.set(state);
        } else {
            // Even if no subscription is needed, we might have updated the ID or cleared pending state
            let current_state = (*state_ref.borrow()).clone();
            if state != current_state {
                *state_ref.borrow_mut() = state.clone();
                chat_state.set(state);
            }
        }
    })
}

fn create_on_connected_callback(chat_state: UseStateHandle<ChatState>, state_ref: Rc<RefCell<ChatState>>) -> Callback<(String, Uuid)> {
    Callback::from(move |(username, user_id)| {
        let mut state = (*state_ref.borrow()).clone();
        state.set_user_info(username, user_id);
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
    for (channel, entries) in &state.messages {
        let last_seq = entries.iter().rev().find_map(|e| {
            if let ChannelEntry::Message(m) = e {
                m.seq
            } else {
                None
            }
        });
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
    let message_id = Uuid::new_v4();
    let channel_name = state.current_channel.clone();
    
    let channel_id = state.channel_id_map.get(&channel_name).cloned().unwrap_or_else(Uuid::nil);

    let pending_msg = ChatMessage {
        id: message_id,
        channel_id,
        user_id: state.user_id,
        user: state.username.clone(),
        timestamp: Utc::now(),
        message: text.clone(),
        pending: true,
        seq: None,
    };

    state.add_pending_message(channel_name.clone(), pending_msg);
    if let Some(service) = &*ws_service.borrow() {
        let service: &Rc<WebSocketService> = service;
        service.send(stomp::create_send_frame(&channel_name, &message_id.to_string(), &text));
    }
    *state_ref.borrow_mut() = state.clone();
    chat_state.set(state);
}

fn render_app(
    chat_state: &UseStateHandle<ChatState>,
    on_switch_channel: Callback<String>,
    on_create_channel: Callback<String>,
    on_send_message: Callback<String>,
    on_open_join_modal: Callback<bool>,
    is_join_modal_open: bool,
    is_modal_create_mode: bool,
    on_close_join_modal: Callback<()>,
    on_join_channel: Callback<frontend::types::Channel>,
    is_mobile_sidebar_open: bool,
    on_toggle_sidebar: Callback<()>,
    on_close_sidebar: Callback<()>,
) -> Html {
    let messages = chat_state.messages
        .get(&chat_state.current_channel)
        .cloned()
        .unwrap_or_default();

    html! {
        <div class="app-container">
            <div class="mini-sidebar">
                <img src="/assets/Sana_logo.webp" alt="Sana Logo" class="mini-logo" />
                <button class="hamburger-menu" onclick={let on_toggle = on_toggle_sidebar.clone(); move |_| on_toggle.emit(())}>
                    <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="3" y1="12" x2="21" y2="12"></line>
                        <line x1="3" y1="6" x2="21" y2="6"></line>
                        <line x1="3" y1="18" x2="21" y2="18"></line>
                    </svg>
                </button>
            </div>
            <Sidebar 
                channels={chat_state.channels.clone()} 
                current_channel={chat_state.current_channel.clone()}
                unread_channels={chat_state.unread_channels.clone()}
                connection_status={chat_state.connection_status}
                on_switch_channel={on_switch_channel}
                on_open_join_modal={on_open_join_modal}
                is_mobile_open={is_mobile_sidebar_open}
                on_close_sidebar={on_close_sidebar}
            />
            <ChatWindow 
                current_channel={chat_state.current_channel.clone()}
                messages={messages}
                current_username={chat_state.username.clone()}
                on_send_message={on_send_message}
                on_toggle_sidebar={on_toggle_sidebar}
            />
            <JoinChannelModal 
                is_open={is_join_modal_open}
                is_create_focus={is_modal_create_mode}
                on_close={on_close_join_modal}
                on_join={on_join_channel}
                on_create={on_create_channel}
            />
        </div>
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Chat => html! { <ChatApp /> },
        Route::Login => html! { <Login /> },
        Route::Register => html! { <Register /> },
        Route::NotFound => html! { <h1>{ "404 Not Found" }</h1> },
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
