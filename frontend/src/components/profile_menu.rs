use yew::prelude::*;
use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use gloo_net::http::Request;
use yew_router::prelude::*;
use crate::Route;
use web_sys::RequestCredentials;
use gloo_timers::future::TimeoutFuture;
use futures::FutureExt;

#[derive(Properties, PartialEq)]
pub struct ProfileMenuProps {
    pub username: String,
}

#[function_component(ProfileMenu)]
pub fn profile_menu(props: &ProfileMenuProps) -> Html {
    let is_open = use_state(|| false);
    let menu_ref = use_node_ref();
    let navigator = use_navigator().unwrap();

    {
        let is_open = is_open.clone();
        let menu_ref = menu_ref.clone();
        use_effect_with(is_open.clone(), move |open| {
            let mut _listener = None;
            if **open {
                let document = web_sys::window().unwrap().document().unwrap();
                let is_open = is_open.clone();
                let menu_ref = menu_ref.clone();
                _listener = Some(EventListener::new(&document, "mousedown", move |event| {
                    let target = event.target().unwrap().dyn_into::<web_sys::Node>().unwrap();
                    if let Some(menu_el) = menu_ref.cast::<web_sys::HtmlElement>() {
                        if !menu_el.contains(Some(&target)) {
                            is_open.set(false);
                        }
                    }
                }));
            }
            move || drop(_listener)
        });
    }

    let toggle_menu = {
        let is_open = is_open.clone();
        Callback::from(move |_| is_open.set(!*is_open))
    };

    let logout = {
        let is_open = is_open.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
            let is_open = is_open.clone();
            let navigator = navigator.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let request_fut = Request::post(&crate::get_api_url("/api/auth/logout"))
                    .credentials(RequestCredentials::Include)
                    .send();

                let _ = futures::select! {
                    r = request_fut.fuse() => r,
                    _ = TimeoutFuture::new(10000).fuse() => {
                        gloo_console::error!("Logout request timed out");
                        Err(gloo_net::Error::GlooError("Timeout".to_string()))
                    }
                };
                
                is_open.set(false);
                navigator.push(&Route::Login);
            });
        })
    };

    let initial = props.username.chars().next().unwrap_or('?').to_uppercase().to_string();

    html! {
        <div class="profile-menu-container" ref={menu_ref} data-testid="profile-menu">
            <button class="profile-button" data-testid="profile-button" onclick={toggle_menu}>
                <div class="avatar">{ &initial }</div>
                <span class="username">{ &props.username }</span>
                <svg viewBox="0 0 20 20" fill="currentColor" class="chevron">
                    <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
                </svg>
            </button>
            if *is_open {
                <div class="profile-dropdown" data-testid="profile-dropdown">
                    <div class="dropdown-header">
                        <div class="avatar-large">{ &initial }</div>
                        <div class="user-info">
                            <div class="full-name">{ &props.username }</div>
                            <div class="status-indicator">
                                <span class="status-dot"></span>
                                { "Active" }
                            </div>
                        </div>
                    </div>
                    <div class="dropdown-divider"></div>
                    <button class="dropdown-item" data-testid="logout-button" onclick={logout}>
                        { "Logout" }
                    </button>
                </div>
            }
        </div>
    }
}
