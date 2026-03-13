use yew::prelude::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::logic::ChatState;
use crate::services::websocket::{WebSocketService, StompClient};
use crate::stomp;
use std::rc::Rc;
use std::cell::RefCell;

async fn fetch_channels() -> Result<Vec<crate::types::Channel>, String> {
    let response = Request::get("/api/channels")
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 200 {
        response.json::<Vec<crate::types::Channel>>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("Failed to fetch channels: {}", response.status()))
    }
}

#[hook]
pub fn use_channels(
    auth_check_done: bool,
    chat_state: UseStateHandle<ChatState>,
    state_ref: Rc<RefCell<ChatState>>,
    ws_service_ref: Rc<RefCell<Option<Rc<WebSocketService>>>>,
) {
    use_effect_with(auth_check_done, move |&done| {
        if done {
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_channels().await {
                    Ok(channels) => {
                        let mut state = (*state_ref.borrow()).clone();
                        state.set_channels(channels);
                        
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
