use yew::prelude::*;
use crate::types::ChannelEntry;
use crate::components::profile_menu::ProfileMenu;
use chrono::{DateTime, Local, Utc};
use crate::hooks::use_chat_scroll;

#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub current_channel: String,
    pub messages: Vec<ChannelEntry>,
    pub current_username: String,
    pub on_send_message: Callback<String>,
    pub on_toggle_sidebar: Callback<()>,
    pub on_load_history: Callback<(String, Option<chrono::DateTime<Utc>>)>,
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let input_value = use_state(String::new);
    let input_ref = use_node_ref();

    let (history_ref, show_new_messages_notification, on_scroll, scroll_to_bottom) = use_chat_scroll(
        props.messages.clone(),
        props.current_channel.clone(),
        props.on_load_history.clone(),
    );

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

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let text = (*input_value).clone();
            if !text.is_empty() {
                on_send_message.emit(text);
                input_value.set(String::new());
                // After send we usually scroll to bottom but it's handled by auto-scroll on message append
            }
        })
    };

    html! {
        <div class="chat-container">
            <header>
                <h1>{ format!("# {}", props.current_channel) }</h1>
                <ProfileMenu username={props.current_username.clone()} />
            </header>
            <div class="chat-history" ref={history_ref} onscroll={on_scroll}>
                { for props.messages.iter().map(|entry| {
                    match entry {
                        ChannelEntry::Message(msg) => {
                            let local_time: DateTime<Local> = DateTime::from(msg.timestamp);
                            let time_str = local_time.format("%H:%M:%S").to_string();

                            let is_me = msg.user == props.current_username;
                            let wrapper_class = classes!(
                                "message-wrapper",
                                if is_me { Some("me") } else { None },
                                if msg.pending { Some("pending") } else { None }
                            );

                            html! {
                                <div key={msg.id.to_string()} class={wrapper_class}>
                                    <div class="meta">
                                        <span class="user">{ &msg.user }</span>
                                        <span class="time">{ time_str }</span>
                                    </div>
                                    <div class="message">{ &msg.message }</div>
                                </div>
                            }
                        },
                        ChannelEntry::UserJoined { id, username, timestamp } => {
                            let local_time: DateTime<Local> = DateTime::from(*timestamp);
                            let time_str = local_time.format("%H:%M:%S").to_string();
                            html! {
                                <div key={id.to_string()} class="message-wrapper system">
                                    <div class="system-message">
                                        { format!("{} has joined", username) }
                                        <span class="time">{ format!(" ({})", time_str) }</span>
                                    </div>
                                </div>
                            }
                        }
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
