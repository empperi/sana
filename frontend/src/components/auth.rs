use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;
use web_sys::HtmlInputElement;
use gloo_net::http::Request;
use web_sys::RequestCredentials;

#[derive(Properties, PartialEq)]
struct AuthFormProps {
    title: String,
    api_endpoint: String,
    submit_label: String,
    alt_text: String,
    alt_route_text: String,
    alt_route: Route,
}

#[function_component(AuthForm)]
fn auth_form(props: &AuthFormProps) -> Html {
    let navigator = use_navigator().unwrap();
    let error_msg = use_state(|| None::<String>);
    
    let username = use_state(String::new);
    let password = use_state(String::new);

    let onsubmit = {
        let username = username.clone();
        let password = password.clone();
        let error_msg = error_msg.clone();
        let navigator = navigator.clone();
        let api_endpoint = props.api_endpoint.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let username_val = (*username).clone();
            let password_val = (*password).clone();
            let error_msg = error_msg.clone();
            let navigator = navigator.clone();
            let api_endpoint = api_endpoint.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let payload = serde_json::json!({
                    "username": username_val,
                    "password": password_val,
                });

                let resp = Request::post(&crate::get_api_url(&api_endpoint))
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
                        let default_err = if api_endpoint.contains("login") { "Login failed" } else { "Registration failed" };
                        let mut text = r.text().await.unwrap_or_else(|_| default_err.to_string());
                        if text.trim().is_empty() {
                            text = format!("{} (Status: {})", default_err, r.status());
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
        <div class="auth-container" data-testid="auth-container">
            <div class="auth-box">
                <img src="/assets/Sana_logo.webp" alt="Sana Logo" class="auth-logo" />
                <h2>{ &props.title }</h2>
                
                if let Some(err) = &*error_msg {
                    <div class="auth-error" data-testid="auth-error">{ err }</div>
                }

                <form {onsubmit} class="auth-form">
                    <input 
                        type="text" 
                        data-testid="username-input"
                        placeholder={if props.api_endpoint.contains("login") { "Username" } else { "Choose a Username" }}
                        value={(*username).clone()} 
                        oninput={on_username_input}
                        required=true
                    />
                    <input 
                        type="password" 
                        data-testid="password-input"
                        placeholder={if props.api_endpoint.contains("login") { "Password" } else { "Choose a Password" }}
                        value={(*password).clone()} 
                        oninput={on_password_input}
                        required=true
                    />
                    <button type="submit" data-testid="auth-submit" disabled={username.is_empty() || password.is_empty()}>
                        { &props.submit_label }
                    </button>
                </form>
                
                <p class="auth-link">
                    { &props.alt_text }
                    <Link<Route> to={props.alt_route.clone()}>{ &props.alt_route_text }</Link<Route>>
                </p>
            </div>
        </div>
    }
}

#[function_component(Login)]
pub fn login() -> Html {
    html! {
        <AuthForm 
            title="Login to Sana"
            api_endpoint="/api/auth/login"
            submit_label="Login"
            alt_text="Don't have an account? "
            alt_route_text="Create one"
            alt_route={Route::Register}
        />
    }
}

#[function_component(Register)]
pub fn register() -> Html {
    html! {
        <AuthForm 
            title="Create an Account"
            api_endpoint="/api/auth/register"
            submit_label="Register"
            alt_text="Already have an account? "
            alt_route_text="Login here"
            alt_route={Route::Login}
        />
    }
}
