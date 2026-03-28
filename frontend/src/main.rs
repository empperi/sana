use yew::prelude::*;
use yew_router::prelude::*;
use uuid::Uuid;
use chrono::{Utc, DateTime};
use std::rc::Rc;
use std::cell::RefCell;
use gloo_net::http::Request;
use web_sys::RequestCredentials;

use frontend::components::sidebar::Sidebar;
use frontend::components::chat_window::ChatWindow;
use frontend::components::auth::{Login, Register};
use frontend::components::join_channel_modal::JoinChannelModal;
use frontend::services::websocket::{WebSocketService, StompClient};
use frontend::types::{ChatMessage, ChannelEntry, MessageType};
use frontend::logic::{self, ChatState, ChatAction};
use frontend::stomp;
use frontend::Route;
use frontend::hooks::{use_auth_check, use_chat_websocket, use_channels};
use frontend::state::{ChatStateContext, ChatStateProvider};

async fn fetch_historical_messages(
    channel_id: Uuid, 
    limit: i64, 
    before: Option<chrono::DateTime<Utc>>
) -> Result<Vec<frontend::types::ChatMessage>, String> {
    let mut url = format!("/api/channels/{}/messages?limit={}", channel_id, limit);
    if let Some(ts) = before {
        let ts_str = ts.to_rfc3339().replace(':', "%3A").replace('+', "%2B");
        url.push_str(&format!("&before={}", ts_str));
    }

    let response = Request::get(&url)
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 200 {
        response.json::<Vec<frontend::types::ChatMessage>>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("Failed to fetch historical messages: {}", response.status()))
    }
}

#[function_component(ChatApp)]
pub fn chat_app() -> Html {
    let auth_check_done = use_auth_check();
    let ctx = use_context::<ChatStateContext>().expect("ChatStateContext not found");

    let is_join_modal_open = use_state(|| false);
    let is_modal_create_mode = use_state(|| false);

    let ws_service = use_chat_websocket(auth_check_done);

    use_channels(auth_check_done);

    let on_switch_channel = {
        let dispatch = ctx.dispatch.clone();
        Callback::from(move |channel: String| {
            dispatch.emit(ChatAction::SelectChannel(channel));
        })
    };

    let on_create_channel = {
        let dispatch = ctx.dispatch.clone();
        let ws_service = ws_service.clone();
        let on_switch_channel = on_switch_channel.clone();
        let is_join_modal_open = is_join_modal_open.clone();
        Callback::from(move |name: String| {
            let dispatch = dispatch.clone();
            let ws_service = ws_service.clone();
            let on_switch_channel = on_switch_channel.clone();
            let is_join_modal_open = is_join_modal_open.clone();

            wasm_bindgen_futures::spawn_local(async move {
                logic::create_channel(
                    name, 
                    dispatch, 
                    ws_service, 
                    Callback::from(move |channel_name| {
                        is_join_modal_open.set(false);
                        on_switch_channel.emit(channel_name);
                    })
                ).await;
            });
        })
    };

    let on_send_message = {
        let dispatch = ctx.dispatch.clone();
        let state = ctx.state.clone();
        let ws_service = ws_service.clone();
        Callback::from(move |text: String| {
            handle_send_message(text, &state, &dispatch, &ws_service);
        })
    };

    let on_mark_read = {
        let ws_service = ws_service.clone();
        Callback::from(move |(channel, message_id): (String, Uuid)| {
            if let Some(service) = &*ws_service.borrow() {
                service.send(stomp::create_read_marker_frame(&channel, &message_id.to_string()));
            }
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
        let dispatch = ctx.dispatch.clone();
        let ws_service = ws_service.clone();
        let is_join_modal_open = is_join_modal_open.clone();
        let on_switch_channel = on_switch_channel.clone();

        Callback::from(move |channel: frontend::types::Channel| {
            let dispatch = dispatch.clone();
            let ws_service = ws_service.clone();
            let is_join_modal_open = is_join_modal_open.clone();
            let on_switch_channel = on_switch_channel.clone();

            wasm_bindgen_futures::spawn_local(async move {
                logic::join_channel(
                    channel, 
                    dispatch, 
                    ws_service, 
                    Callback::from(move |channel_name| {
                        is_join_modal_open.set(false);
                        on_switch_channel.emit(channel_name);
                    })
                ).await;
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

    let on_load_history = {
        let state = ctx.state.clone();
        let dispatch = ctx.dispatch.clone();
        Callback::from(move |(channel_name, before): (String, Option<DateTime<Utc>>)| {
            let state = state.clone();
            let dispatch = dispatch.clone();
            
            if let Some(channel_id) = state.channel_id_map.get(&channel_name).cloned() {
                wasm_bindgen_futures::spawn_local(async move {
                    match fetch_historical_messages(channel_id, 100, before).await {
                        Ok(messages) => {
                            let entries = messages.into_iter().map(|msg| {
                                match msg.msg_type {
                                    MessageType::Join => ChannelEntry::UserJoined {
                                        id: msg.id,
                                        user_id: msg.user_id,
                                        username: msg.user.clone(),
                                        timestamp: msg.timestamp,
                                    },
                                    MessageType::Chat => ChannelEntry::Message(msg),
                                }
                            }).collect();
                            dispatch.emit(ChatAction::PrependHistory { channel: channel_name, history: entries });
                        }
                        Err(e) => gloo_console::error!(format!("Failed to load history: {}", e)),
                    }
                });
            }
        })
    };

    if !auth_check_done {
        return html! { <div>{ "Loading..." }</div> };
    }

    render_app(
        &ctx.state, 
        on_switch_channel, 
        on_create_channel, 
        on_send_message,
        on_mark_read,
        on_open_join_modal,
        *is_join_modal_open,
        *is_modal_create_mode,
        on_close_join_modal,
        on_join_channel,
        *is_mobile_sidebar_open,
        on_toggle_sidebar,
        on_close_sidebar,
        on_load_history
    )
}

fn handle_send_message(
    text: String, 
    state: &ChatState, 
    dispatch: &Callback<ChatAction>,
    ws_service: &Rc<RefCell<Option<Rc<WebSocketService>>>>
) {
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
        msg_type: MessageType::Chat,
    };

    dispatch.emit(ChatAction::AddPendingMessage { channel: channel_name.clone(), msg: pending_msg });
    if let Some(service) = &*ws_service.borrow() {
        let service: &Rc<WebSocketService> = service;
        service.send(stomp::create_send_frame(&channel_name, &message_id.to_string(), &text));
    }
}

fn render_app(
    chat_state: &ChatState,
    on_switch_channel: Callback<String>,
    on_create_channel: Callback<String>,
    on_send_message: Callback<String>,
    on_mark_read: Callback<(String, Uuid)>,
    on_open_join_modal: Callback<bool>,
    is_join_modal_open: bool,
    is_modal_create_mode: bool,
    on_close_join_modal: Callback<()>,
    on_join_channel: Callback<frontend::types::Channel>,
    is_mobile_sidebar_open: bool,
    on_toggle_sidebar: Callback<()>,
    on_close_sidebar: Callback<()>,
    on_load_history: Callback<(String, Option<DateTime<Utc>>)>,
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
                on_mark_read={on_mark_read}
                on_toggle_sidebar={on_toggle_sidebar}
                on_load_history={on_load_history}
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
            <ChatStateProvider>
                <Switch<Route> render={switch} />
            </ChatStateProvider>
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
