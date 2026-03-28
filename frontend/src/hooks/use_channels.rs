use yew::prelude::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::state::{ChatStateContext, ChatAction};

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
) {
    let ctx = use_context::<ChatStateContext>().expect("ChatStateContext not found");
    let dispatch = ctx.dispatch.clone();

    use_effect_with(auth_check_done, move |&done| {
        if done {
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_channels().await {
                    Ok(channels) => {
                        dispatch.emit(ChatAction::SetChannels(channels));
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
