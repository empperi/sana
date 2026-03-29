use yew::prelude::*;
use yew_router::prelude::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::Route;
use gloo_timers::future::TimeoutFuture;
use futures::FutureExt;

#[hook]
pub fn use_auth_check() -> bool {
    let navigator = use_navigator().unwrap();
    let auth_check_done = use_state(|| false);

    {
        let navigator = navigator.clone();
        let auth_check_done = auth_check_done.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let request_fut = Request::get(&crate::get_api_url("/api/auth/me"))
                    .credentials(RequestCredentials::Include)
                    .send();

                let resp = futures::select! {
                    r = request_fut.fuse() => r,
                    _ = TimeoutFuture::new(10000).fuse() => {
                        gloo_console::error!("Auth check timed out");
                        navigator.push(&Route::Login);
                        return;
                    }
                };

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

    *auth_check_done
}
