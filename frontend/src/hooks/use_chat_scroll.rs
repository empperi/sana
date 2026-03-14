use yew::prelude::*;
use web_sys::HtmlElement;
use crate::types::ChannelEntry;

#[hook]
pub fn use_chat_scroll(
    messages: Vec<ChannelEntry>,
    current_channel: String,
    on_load_history: Callback<(String, Option<chrono::DateTime<chrono::Utc>>)>,
) -> (NodeRef, UseStateHandle<bool>, UseStateHandle<bool>, Callback<Event>, Callback<MouseEvent>) {
    let history_ref = use_node_ref();
    let show_new_messages_notification = use_state(|| false);
    let is_user_scrolled_up = use_state(|| false);
    let last_requested_history = use_state(|| None::<chrono::DateTime<chrono::Utc>>);
    let prev_messages_len = use_state(|| 0);
    let prev_channel = use_state(|| current_channel.clone());
    let last_scroll_height = use_state(|| 0);

    if *prev_channel != current_channel {
        prev_channel.set(current_channel.clone());
        last_requested_history.set(None);
        prev_messages_len.set(0); // Reset to 0 so initial messages trigger scroll
        is_user_scrolled_up.set(false);
        last_scroll_height.set(0);
    }

    {
        let history_ref = history_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();
        let messages_len = messages.len();
        let prev_len = *prev_messages_len;
        let prev_len_state = prev_messages_len.clone();
        let last_sh = last_scroll_height.clone();

        use_effect_with(messages_len, move |_| {
            if let Some(element) = history_ref.cast::<HtmlElement>() {
                let current_sh = element.scroll_height();
                
                if !*is_user_scrolled_up && messages_len > prev_len {
                    element.set_scroll_top(current_sh);
                } else if *is_user_scrolled_up && messages_len > prev_len {
                     let sh_diff = current_sh - *last_sh;
                     if sh_diff > 0 && element.scroll_top() < 200 {
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

    let on_scroll = {
        let history_ref = history_ref.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let show_new_messages_notification = show_new_messages_notification.clone();
        let messages = messages.clone();
        let channel = current_channel.clone();
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

                if scroll_top < 100 && !messages.is_empty() {
                    let oldest_msg_ts = messages.iter().find_map(|e| match e {
                        ChannelEntry::Message(m) => Some(m.timestamp),
                        ChannelEntry::UserJoined { timestamp, .. } => Some(*timestamp),
                        _ => None,
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

    (history_ref, show_new_messages_notification, is_user_scrolled_up, on_scroll, scroll_to_bottom)
}
