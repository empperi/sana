use yew::prelude::*;
use crate::types::ChatMessage;
use chrono::{DateTime, Local};
use web_sys::{HtmlElement, HtmlInputElement};
use gloo_events::EventListener;
use wasm_bindgen::JsCast;

#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub current_channel: String,
    pub messages: Vec<ChatMessage>,
    pub current_username: String,
    pub on_send_message: Callback<String>,
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let input_value = use_state(String::new);
    let chat_container_ref = use_node_ref();
    let input_ref = use_node_ref();
    let show_new_messages_notification = use_state(|| false);
    let is_user_scrolled_up = use_state(|| false);

    // Effect to redirect global typing to the message input
    {
        let input_ref = input_ref.clone();
        use_effect_with((), move |_| {
            let document = web_sys::window().unwrap().document().unwrap();
            let listener = EventListener::new(&document, "keydown", move |event| {
                let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
                
                // Don't intercept if user is using a modifier key (Cmd, Ctrl, Alt)
                if event.meta_key() || event.ctrl_key() || event.alt_key() {
                    return;
                }

                // Check if the key is a single character (printable)
                if event.key().chars().count() == 1 {
                    let active_element = web_sys::window()
                        .unwrap()
                        .document()
                        .unwrap()
                        .active_element();

                    if let Some(active) = active_element {
                        let tag_name = active.tag_name().to_uppercase();
                        if tag_name == "INPUT" || tag_name == "TEXTAREA" {
                            // Already focusing an input
                            return;
                        }
                    }

                    // Not focusing an input, redirect to message input
                    if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                        input.focus().unwrap();
                    }
                }
            });

            move || drop(listener)
        });
    }

    // Effect to handle scrolling when messages change
    {
        let chat_container_ref = chat_container_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();
        let messages_len = props.messages.len();

        use_effect_with(messages_len, move |_| {
            if !*is_user_scrolled_up {
                if let Some(element) = chat_container_ref.cast::<HtmlElement>() {
                    element.set_scroll_top(element.scroll_height());
                }
            } else {
                 // If scrolled up and new message arrives (length increased), show notification
                 // Note: This logic is simplified; ideally we'd check if the new message is ours or others
                 show_new_messages_notification.set(true);
            }
            || {}
        });
    }

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            input_value.set(input.value());
        })
    };

    let on_submit = {
        let input_value = input_value.clone();
        let on_send_message = props.on_send_message.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let text = (*input_value).clone();
            if !text.is_empty() {
                on_send_message.emit(text);
                input_value.set(String::new());
                is_user_scrolled_up.set(false);
                show_new_messages_notification.set(false);
            }
        })
    };

    let on_scroll = {
        let chat_container_ref = chat_container_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();

        Callback::from(move |_| {
            if let Some(element) = chat_container_ref.cast::<HtmlElement>() {
                let scroll_top = element.scroll_top();
                let scroll_height = element.scroll_height();
                let client_height = element.client_height();

                let is_at_bottom = (scroll_height - scroll_top - client_height).abs() < 10;

                if is_at_bottom {
                    is_user_scrolled_up.set(false);
                    show_new_messages_notification.set(false);
                } else {
                    is_user_scrolled_up.set(true);
                }
            }
        })
    };

    let scroll_to_bottom = {
        let chat_container_ref = chat_container_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();

        Callback::from(move |_| {
            if let Some(element) = chat_container_ref.cast::<HtmlElement>() {
                element.set_scroll_top(element.scroll_height());
                is_user_scrolled_up.set(false);
                show_new_messages_notification.set(false);
            }
        })
    };

    html! {
        <div class="chat-container">
            <header>
                <h1>{ format!("# {}", props.current_channel) }</h1>
            </header>
            <div class="chat-history" ref={chat_container_ref} onscroll={on_scroll}>
                { for props.messages.iter().map(|msg| {
                    let local_time: DateTime<Local> = DateTime::from(msg.timestamp);
                    let time_str = local_time.format("%H:%M:%S").to_string();

                    let is_me = msg.user == props.current_username;
                    let mut wrapper_class = if is_me { "message-wrapper me".to_string() } else { "message-wrapper".to_string() };
                    if msg.pending {
                        wrapper_class = format!("{} pending", wrapper_class);
                    }

                    html! {
                        <div class={wrapper_class}>
                            <div class="meta">
                                <span class="user">{ &msg.user }</span>
                                <span class="time">{ time_str }</span>
                            </div>
                            <div class="message">{ &msg.message }</div>
                        </div>
                    }
                }) }
            </div>
            if *show_new_messages_notification {
                <div class="new-messages-notification" onclick={scroll_to_bottom}>
                    { "New messages ↓" }
                </div>
            }
            <footer>
                <form onsubmit={on_submit}>
                    <input
                        type="text"
                        ref={input_ref}
                        value={(*input_value).clone()}
                        oninput={on_input}
                        placeholder={format!("Message #{}", props.current_channel)}
                    />
                    <button type="submit" disabled={input_value.is_empty()}>{ "Send" }</button>
                </form>
            </footer>
        </div>
    }
}
