mod components;
mod services;
mod types;

use yew::prelude::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use chrono::Utc;
use std::rc::Rc;
use std::cell::RefCell;

use components::sidebar::Sidebar;
use components::chat_window::ChatWindow;
use services::websocket::WebSocketService;
use types::ChatMessage;

#[function_component(App)]
fn app() -> Html {
    let channels = use_state(|| vec!["test-channel".to_string()]);
    let current_channel = use_state(|| "test-channel".to_string());
    let current_username = use_state(|| String::new());
    let channel_messages = use_state(|| HashMap::<String, Vec<ChatMessage>>::new());
    let unread_channels = use_state(|| HashSet::<String>::new());

    let ws_service = use_mut_ref(|| None::<Rc<RefCell<WebSocketService>>>);

    // Let's implement the fix using `use_mut_ref` for the source of truth.
    let messages_ref = use_mut_ref(|| HashMap::<String, Vec<ChatMessage>>::new());

    // We also need a mutable ref for unread channels to avoid stale closure issues
    let unread_channels_ref = use_mut_ref(|| HashSet::<String>::new());

    // We still need `channel_messages` state to trigger re-renders.
    // But we won't use its value for updates.

    {
        let current_username = current_username.clone();
        let channel_messages = channel_messages.clone(); // Used only for triggering updates
        let messages_ref = messages_ref.clone(); // Source of truth
        let channels = channels.clone();
        let ws_service_ref = ws_service.clone();
        let current_channel = current_channel.clone();
        let unread_channels = unread_channels.clone();
        let unread_channels_ref = unread_channels_ref.clone();

        use_effect_with((), move |_| {
            let on_message = {
                let channel_messages = channel_messages.clone();
                let messages_ref = messages_ref.clone();
                let current_channel = current_channel.clone();
                let unread_channels = unread_channels.clone();
                let unread_channels_ref = unread_channels_ref.clone();

                Callback::from(move |(channel, msg): (String, ChatMessage)| {
                    let mut all_messages = messages_ref.borrow_mut();
                    let messages = all_messages.entry(channel.clone()).or_insert_with(Vec::new);

                    if let Some(pos) = messages.iter().position(|m| m.id == msg.id && m.pending) {
                        messages[pos] = msg;
                    } else {
                        messages.push(msg);

                        // If message is for a different channel, mark as unread
                        // Note: *current_channel here is the value captured when effect ran (initial value).
                        // This is the stale closure problem again!
                        // We need to know the *actual* current channel.

                        // Since we can't easily access the live state in this closure without refactoring to use_reducer or context,
                        // we can use another mutable ref for current_channel that we keep in sync.
                        // OR, we can just check if the channel is the one we are viewing? No, we don't know which one we are viewing.

                        // Wait, `current_channel` is a UseStateHandle. `*current_channel` gives the value.
                        // But the handle itself is captured. Does the handle point to the *latest* value?
                        // No, UseStateHandle is immutable in the sense that it wraps a specific version of the state.

                        // So we need a `current_channel_ref` that we update whenever `current_channel` changes.
                    }

                    // Trigger re-render by updating the state with a clone of the new data
                    channel_messages.set(all_messages.clone());

                    // We can't update unread_channels correctly here because we don't know the *real* current channel.
                    // We need to fix this architecture.
                })
            };

            // ...
            || {}
        });
    }

    // To fix the stale current_channel issue, let's use a RefCell to track it.
    let current_channel_ref = use_mut_ref(|| "test-channel".to_string());

    // Sync ref with state
    {
        let current_channel_ref = current_channel_ref.clone();
        use_effect_with(current_channel.clone(), move |current_channel| {
            *current_channel_ref.borrow_mut() = (**current_channel).clone();
            || {}
        });
    }

    {
        let current_username = current_username.clone();
        let channel_messages = channel_messages.clone();
        let messages_ref = messages_ref.clone();
        let channels = channels.clone();
        let ws_service_ref = ws_service.clone();
        let unread_channels = unread_channels.clone();
        let unread_channels_ref = unread_channels_ref.clone();
        let current_channel_ref = current_channel_ref.clone();

        use_effect_with((), move |_| {
            let on_message = {
                let channel_messages = channel_messages.clone();
                let messages_ref = messages_ref.clone();
                let unread_channels = unread_channels.clone();
                let unread_channels_ref = unread_channels_ref.clone();
                let current_channel_ref = current_channel_ref.clone();

                Callback::from(move |(channel, msg): (String, ChatMessage)| {
                    let mut all_messages = messages_ref.borrow_mut();
                    let messages = all_messages.entry(channel.clone()).or_insert_with(Vec::new);

                    if let Some(pos) = messages.iter().position(|m| m.id == msg.id && m.pending) {
                        messages[pos] = msg;
                    } else {
                        messages.push(msg);

                        // Check against the *latest* current channel
                        let active_channel = current_channel_ref.borrow();
                        if channel != *active_channel {
                            let mut unread = unread_channels_ref.borrow_mut();
                            if unread.insert(channel.clone()) {
                                // Only update state if it changed to trigger re-render
                                unread_channels.set(unread.clone());
                            }
                        }
                    }

                    channel_messages.set(all_messages.clone());
                })
            };

            let on_connected = Callback::from(move |username: String| {
                current_username.set(username);
            });

            let service = Rc::new(RefCell::new(WebSocketService::connect(on_message, on_connected)));
            *ws_service_ref.borrow_mut() = Some(service.clone());

            // Subscribe to initial channels
            for channel in channels.iter() {
                let msg = format!("SUBSCRIBE\nid:0\ndestination:/topic/{}\n\n\0", channel);
                service.borrow_mut().send(msg);
            }

            || {}
        });
    }

    let on_switch_channel = {
        let current_channel = current_channel.clone();
        let unread_channels = unread_channels.clone();
        let unread_channels_ref = unread_channels_ref.clone();

        Callback::from(move |channel: String| {
            current_channel.set(channel.clone());

            // Remove from unread
            let mut unread = unread_channels_ref.borrow_mut();
            if unread.remove(&channel) {
                unread_channels.set(unread.clone());
            }
        })
    };

    let on_create_channel = {
        let channels = channels.clone();
        let ws_service = ws_service.clone();
        let on_switch_channel = on_switch_channel.clone();

        Callback::from(move |name: String| {
            if !channels.contains(&name) {
                let mut new_channels = (*channels).clone();
                new_channels.push(name.clone());
                channels.set(new_channels);

                if let Some(service) = &*ws_service.borrow() {
                    let msg = format!("SUBSCRIBE\nid:0\ndestination:/topic/{}\n\n\0", name);
                    service.borrow_mut().send(msg);
                }

                on_switch_channel.emit(name);
            }
        })
    };

    let on_send_message = {
        let channel_messages = channel_messages.clone();
        let messages_ref = messages_ref.clone();
        let current_channel = current_channel.clone();
        let current_username = current_username.clone();
        let ws_service = ws_service.clone();

        Callback::from(move |text: String| {
            let message_id = Uuid::new_v4().to_string();
            let channel_name = (*current_channel).clone();

            let pending_msg = ChatMessage {
                id: message_id.clone(),
                user: (*current_username).clone(),
                timestamp: Utc::now().timestamp_millis(),
                message: text.clone(),
                pending: true,
            };

            let mut all_messages = messages_ref.borrow_mut();
            let messages = all_messages.entry(channel_name.clone()).or_insert_with(Vec::new);
            messages.push(pending_msg);

            channel_messages.set(all_messages.clone());

            if let Some(service) = &*ws_service.borrow() {
                let msg = format!("SEND\ndestination:/topic/{}\nmessage_id:{}\n\n{}\0", channel_name, message_id, text);
                service.borrow_mut().send(msg);
            }
        })
    };

    let messages_for_current_channel = channel_messages
        .get(&*current_channel)
        .cloned()
        .unwrap_or_default();

    html! {
        <div class="app-container">
            <Sidebar
                channels={(*channels).clone()}
                current_channel={(*current_channel).clone()}
                unread_channels={(*unread_channels).clone()}
                on_switch_channel={on_switch_channel}
                on_create_channel={on_create_channel}
            />
            <ChatWindow
                current_channel={(*current_channel).clone()}
                messages={messages_for_current_channel}
                current_username={(*current_username).clone()}
                on_send_message={on_send_message}
            />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
