use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use gloo_net::http::Request;
use web_sys::RequestCredentials;

#[function_component(Login)]
pub fn login() -> Html {
    let navigator = use_navigator().unwrap();
    let error_msg = use_state(|| None::<String>);
    
    let username = use_state(String::new);
    let password = use_state(String::new);

    let onsubmit = {
        let username = username.clone();
        let password = password.clone();
        let error_msg = error_msg.clone();
        let navigator = navigator.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let username_val = (*username).clone();
            let password_val = (*password).clone();
            let error_msg = error_msg.clone();
            let navigator = navigator.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let payload = serde_json::json!({
                    "username": username_val,
                    "password": password_val,
                });

                let resp = Request::post(&crate::get_api_url("/api/auth/login"))
                    .credentials(RequestCredentials::Include)
                    .json(&payload)
                    .unwrap()
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.ok() => {
                        let content_type = r.headers().get("content-type").unwrap_or_default();
                        if content_type.contains("application/json") {
                            navigator.push(&Route::Chat);
                        } else {
                            error_msg.set(Some("Invalid server response (not JSON)".to_string()));
                        }
                    }
                    Ok(r) => {
                        let mut text = r.text().await.unwrap_or_else(|_| "Login failed".to_string());
                        if text.trim().is_empty() {
                            text = format!("Login failed (Status: {})", r.status());
                        }
                        error_msg.set(Some(text));
                    }
                    Err(_) => {
                        error_msg.set(Some("Network error".to_string()));
                    }
                }
            });
        })
    };

    let on_username_input = {
        let username = username.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_dyn_into::<HtmlInputElement>();
            if let Some(input) = input {
                username.set(input.value());
            }
        })
    };

    let on_password_input = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_dyn_into::<HtmlInputElement>();
            if let Some(input) = input {
                password.set(input.value());
            }
        })
    };

    html! {
        <div class="auth-container">
            <div class="auth-box">
                <img src="/assets/Sana_logo.webp" alt="Sana Logo" class="auth-logo" />
                <h2>{ "Login to Sana" }</h2>
                
                if let Some(err) = &*error_msg {
                    <div class="auth-error">{ err }</div>
                }

                <form {onsubmit} class="auth-form">
                    <input 
                        type="text" 
                        placeholder="Username" 
                        value={(*username).clone()} 
                        oninput={on_username_input}
                        required=true
                    />
                    <input 
                        type="password" 
                        placeholder="Password" 
                        value={(*password).clone()} 
                        oninput={on_password_input}
                        required=true
                    />
                    <button type="submit" disabled={username.is_empty() || password.is_empty()}>
                        { "Login" }
                    </button>
                </form>
                
                <p class="auth-link">
                    { "Don't have an account? " }
                    <Link<Route> to={Route::Register}>{ "Create one" }</Link<Route>>
                </p>
            </div>
        </div>
    }
}

#[function_component(Register)]
pub fn register() -> Html {
    let navigator = use_navigator().unwrap();
    let error_msg = use_state(|| None::<String>);
    
    let username = use_state(String::new);
    let password = use_state(String::new);

    let onsubmit = {
        let username = username.clone();
        let password = password.clone();
        let error_msg = error_msg.clone();
        let navigator = navigator.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let username_val = (*username).clone();
            let password_val = (*password).clone();
            let error_msg = error_msg.clone();
            let navigator = navigator.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let payload = serde_json::json!({
                    "username": username_val,
                    "password": password_val,
                });

                let resp = Request::post(&crate::get_api_url("/api/auth/register"))
                    .credentials(RequestCredentials::Include)
                    .json(&payload)
                    .unwrap()
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.ok() => {
                        let content_type = r.headers().get("content-type").unwrap_or_default();
                        if content_type.contains("application/json") {
                            navigator.push(&Route::Chat);
                        } else {
                            error_msg.set(Some("Invalid server response (not JSON)".to_string()));
                        }
                    }
                    Ok(r) => {
                        let mut text = r.text().await.unwrap_or_else(|_| "Registration failed".to_string());
                        if text.trim().is_empty() {
                            text = format!("Registration failed (Status: {})", r.status());
                        }
                        error_msg.set(Some(text));
                    }
                    Err(_) => {
                        error_msg.set(Some("Network error".to_string()));
                    }
                }
            });
        })
    };

    let on_username_input = {
        let username = username.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_dyn_into::<HtmlInputElement>();
            if let Some(input) = input {
                username.set(input.value());
            }
        })
    };

    let on_password_input = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_dyn_into::<HtmlInputElement>();
            if let Some(input) = input {
                password.set(input.value());
            }
        })
    };

    html! {
        <div class="auth-container">
            <div class="auth-box">
                <img src="/assets/Sana_logo.webp" alt="Sana Logo" class="auth-logo" />
                <h2>{ "Create an Account" }</h2>
                
                if let Some(err) = &*error_msg {
                    <div class="auth-error">{ err }</div>
                }

                <form {onsubmit} class="auth-form">
                    <input 
                        type="text" 
                        placeholder="Choose a Username" 
                        value={(*username).clone()} 
                        oninput={on_username_input}
                        required=true
                    />
                    <input 
                        type="password" 
                        placeholder="Choose a Password" 
                        value={(*password).clone()} 
                        oninput={on_password_input}
                        required=true
                    />
                    <button type="submit" disabled={username.is_empty() || password.is_empty()}>
                        { "Register" }
                    </button>
                </form>
                
                <p class="auth-link">
                    { "Already have an account? " }
                    <Link<Route> to={Route::Login}>{ "Login here" }</Link<Route>>
                </p>
            </div>
        </div>
    }
}
