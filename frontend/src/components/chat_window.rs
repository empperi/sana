use yew::prelude::*;
use crate::types::ChannelEntry;
use crate::components::profile_menu::ProfileMenu;
use chrono::{DateTime, Local, Utc};
use web_sys::HtmlElement;

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
    let history_ref = use_node_ref();
    let input_ref = use_node_ref();
    let show_new_messages_notification = use_state(|| false);
    let is_user_scrolled_up = use_state(|| false);
    let last_requested_history = use_state(|| None::<chrono::DateTime<Utc>>);
    let prev_messages_len = use_state(|| 0);
    let prev_channel = use_state(|| props.current_channel.clone());
    let last_scroll_height = use_state(|| 0);

    // Reset state on channel switch
    if *prev_channel != props.current_channel {
        prev_channel.set(props.current_channel.clone());
        last_requested_history.set(None);
        prev_messages_len.set(props.messages.len());
        is_user_scrolled_up.set(false);
        last_scroll_height.set(0);
    }

    // Effect to maintain scroll position when new messages arrive (history or new)
    {
        let history_ref = history_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();
        let messages_len = props.messages.len();
        let prev_len = *prev_messages_len;
        let prev_len_state = prev_messages_len.clone();
        let last_sh = last_scroll_height.clone();

        use_effect_with(messages_len, move |_| {
            if let Some(element) = history_ref.cast::<HtmlElement>() {
                let current_sh = element.scroll_height();
                
                if !*is_user_scrolled_up && messages_len > prev_len {
                    // Normal new message at bottom
                    element.set_scroll_top(current_sh);
                } else if *is_user_scrolled_up && messages_len > prev_len {
                     // Might be history load
                     let sh_diff = current_sh - *last_sh;
                     if sh_diff > 0 && element.scroll_top() < 200 {
                         // Likely history prepended
                         element.set_scroll_top(element.scroll_top() + sh_diff);
                     } else if element.scroll_top() > 100 {
                         show_new_messages_notification.set(true);
                     }
                }
                last_sh.set(current_sh);
            }
            prev_len_state.set(messages_len);
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
        let history_ref = history_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();
        let on_load_history = props.on_load_history.clone();
        let messages = props.messages.clone();
        let channel = props.current_channel.clone();
        let last_requested = last_requested_history.clone();
        let last_sh = last_scroll_height.clone();

        Callback::from(move |_| {
            if let Some(element) = history_ref.cast::<HtmlElement>() {
                let scroll_top = element.scroll_top();
                let scroll_height = element.scroll_height();
                let client_height = element.client_height();

                last_sh.set(scroll_height);

                let is_at_bottom = (scroll_height - scroll_top - client_height).abs() < 10;

                if is_at_bottom {
                    is_user_scrolled_up.set(false);
                    show_new_messages_notification.set(false);
                } else {
                    is_user_scrolled_up.set(true);
                }

                // Check if we hit the top to load history
                if scroll_top < 100 && !messages.is_empty() {
                    let oldest_msg_ts = messages.iter().find_map(|e| match e {
                        ChannelEntry::Message(m) => Some(m.timestamp),
                        ChannelEntry::UserJoined { timestamp, .. } => Some(*timestamp),
                    });

                    if let Some(ts) = oldest_msg_ts {
                        if *last_requested != Some(ts) {
                            last_requested.set(Some(ts));
                            on_load_history.emit((channel.clone(), Some(ts)));
                        }
                    }
                }
            }
        })
    };

    let scroll_to_bottom = {
        let history_ref = history_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();

        Callback::from(move |_| {
            if let Some(element) = history_ref.cast::<HtmlElement>() {
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
