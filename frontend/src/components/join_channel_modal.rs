use yew::prelude::*;
use crate::types::Channel;
use gloo_net::http::Request;
use web_sys::{HtmlInputElement, RequestCredentials};

#[derive(Properties, PartialEq)]
pub struct JoinChannelModalProps {
    pub is_open: bool,
    pub is_create_focus: bool,
    pub on_close: Callback<()>,
    pub on_join: Callback<Channel>,
    pub on_create: Callback<String>,
}

#[function_component(JoinChannelModal)]
pub fn join_channel_modal(props: &JoinChannelModalProps) -> Html {
    let search_query = use_state(String::new);
    let channels = use_state(Vec::<Channel>::new);
    let error = use_state(|| None::<String>);
    let loading = use_state(|| false);
    
    let new_channel_name = use_state(String::new);

    let search_input_ref = use_node_ref();
    let create_input_ref = use_node_ref();

    {
        let search_query = search_query.clone();
        let new_channel_name = new_channel_name.clone();
        let is_open = props.is_open;
        let is_create_focus = props.is_create_focus;
        let search_input_ref = search_input_ref.clone();
        let create_input_ref = create_input_ref.clone();

        use_effect_with(is_open, move |&open| {
            if !open {
                search_query.set(String::new());
                new_channel_name.set(String::new());
            } else {
                if is_create_focus {
                    if let Some(input) = create_input_ref.cast::<HtmlInputElement>() {
                        let _ = input.focus();
                    }
                } else {
                    if let Some(input) = search_input_ref.cast::<HtmlInputElement>() {
                        let _ = input.focus();
                    }
                }
            }
            || {}
        });
    }

    let fetch_unjoined = {
        let search_query = search_query.clone();
        let channels = channels.clone();
        let error = error.clone();
        let loading = loading.clone();
        
        move || {
            let search_query = search_query.clone();
            let channels = channels.clone();
            let error = error.clone();
            let loading = loading.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                loading.set(true);
                let url = format!("/api/channels/unjoined?q={}", *search_query);
                let resp = Request::get(&url)
                    .credentials(RequestCredentials::Include)
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.status() == 200 => {
                        if let Ok(data) = r.json::<Vec<Channel>>().await {
                            channels.set(data);
                            error.set(None);
                        }
                    }
                    _ => {
                        error.set(Some("Failed to fetch channels".to_string()));
                    }
                }
                loading.set(false);
            });
        }
    };

    {
        let fetch_unjoined = fetch_unjoined.clone();
        let is_open = props.is_open;
        use_effect_with((is_open, (*search_query).clone()), move |_| {
            if is_open {
                fetch_unjoined();
            }
            || {}
        });
    }

    let on_search_input = {
        let search_query = search_query.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            search_query.set(input.value());
        })
    };

    let on_create_input = {
        let new_channel_name = new_channel_name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            new_channel_name.set(input.value());
        })
    };

    let on_create_submit = {
        let new_channel_name = new_channel_name.clone();
        let on_create = props.on_create.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let name = (*new_channel_name).clone();
            if !name.is_empty() {
                on_create.emit(name);
                new_channel_name.set(String::new());
            }
        })
    };

    if !props.is_open {
        return html! {};
    }

    html! {
        <div class="modal-overlay" onclick={let on_close = props.on_close.clone(); move |_| on_close.emit(())}>
            <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                <header class="modal-header">
                    <h2>{ "Channels" }</h2>
                    <button class="close-button" onclick={let on_close = props.on_close.clone(); move |_| on_close.emit(())}>{ "×" }</button>
                </header>
                
                <div class="modal-body">
                    <section class="create-section">
                        <h3>{ "Create New Channel" }</h3>
                        <form onsubmit={on_create_submit} class="create-form">
                            <input 
                                type="text" 
                                ref={create_input_ref}
                                placeholder="New channel name..." 
                                value={(*new_channel_name).clone()}
                                oninput={on_create_input}
                            />
                            <button type="submit" disabled={new_channel_name.is_empty()}>{ "Create" }</button>
                        </form>
                    </section>

                    <hr class="modal-divider" />

                    <section class="join-section">
                        <h3>{ "Join Existing Channels" }</h3>
                        <input 
                            type="text" 
                            ref={search_input_ref}
                            placeholder="Search channels..." 
                            class="search-input"
                            value={(*search_query).clone()}
                            oninput={on_search_input}
                        />

                        if *loading {
                            <div class="modal-message">{ "Loading..." }</div>
                        } else if let Some(err) = &*error {
                            <div class="modal-error">{ err }</div>
                        } else if channels.is_empty() {
                            <div class="modal-message">{ "No unjoined channels found" }</div>
                        } else {
                            <ul class="unjoined-channel-list">
                                { for channels.iter().map(|channel| {
                                    let channel_clone = channel.clone();
                                    let on_join = props.on_join.clone();
                                    html! {
                                        <li key={channel.id.to_string()}>
                                            <span class="channel-name">{ format!("# {}", channel.name) }</span>
                                            <button class="join-button" onclick={move |_| on_join.emit(channel_clone.clone())}>
                                                { "Join" }
                                            </button>
                                        </li>
                                    }
                                }) }
                            </ul>
                        }
                    </section>
                </div>
            </div>
        </div>
    }
}
