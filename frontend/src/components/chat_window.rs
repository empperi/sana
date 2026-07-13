use yew::prelude::*;
use crate::types::ChannelEntry;
use crate::components::profile_menu::ProfileMenu;
use crate::components::attachment_button::AttachmentButton;
use crate::components::attachment_renderer::AttachmentRenderer;
use chrono::{DateTime, Local, Utc};
use crate::hooks::use_chat_scroll;
use uuid::Uuid;
use crate::state::ChatStateContext;
use crate::logic::ChatAction;
use web_sys::{DragEvent, ClipboardEvent};
use wasm_bindgen::JsCast;

#[derive(Properties, PartialEq)]
pub struct ChatWindowProps {
    pub current_channel: String,
    pub messages: Vec<ChannelEntry>,
    pub current_username: String,
    pub on_send_message: Callback<String>,
    pub on_mark_read: Callback<(String, Uuid)>,
    pub on_toggle_sidebar: Callback<()>,
    pub on_load_history: Callback<(String, Option<chrono::DateTime<Utc>>)>,
}

#[function_component(ChatWindow)]
pub fn chat_window(props: &ChatWindowProps) -> Html {
    let ctx = use_context::<ChatStateContext>().expect("No ChatStateContext found");
    
    let input_value = use_state(String::new);
    let input_ref = use_node_ref();

    let (history_ref, show_new_messages_notification, is_user_scrolled_up, on_scroll, scroll_to_bottom) = use_chat_scroll(
        props.messages.clone(),
        props.current_channel.clone(),
        props.on_load_history.clone(),
    );

    let last_read_emitted = use_state(|| None::<Uuid>);
    let last_emit_time = use_state(|| 0.0);

    let prev_state_ref = use_mut_ref(|| (props.current_channel.clone(), props.messages.clone(), !*is_user_scrolled_up));

    {
        let (ref old_channel, ref old_msgs, old_was_at_bottom) = *prev_state_ref.borrow();
        if old_channel != &props.current_channel && old_was_at_bottom {
            if let Some(last_entry) = old_msgs.last() {
                let entry_id = match last_entry {
                    ChannelEntry::Message(m) if !m.pending => Some(m.id),
                    ChannelEntry::UserJoined { id, .. } => Some(*id),
                    _ => None,
                };
                if let Some(id) = entry_id {
                    props.on_mark_read.emit((old_channel.clone(), id));
                }
            }
        }
    }
    *prev_state_ref.borrow_mut() = (props.current_channel.clone(), props.messages.clone(), !*is_user_scrolled_up);

    {
        let messages = props.messages.clone();
        let is_user_scrolled_up = is_user_scrolled_up.clone();
        let on_mark_read = props.on_mark_read.clone();
        let last_read_emitted = last_read_emitted.clone();
        let last_emit_time = last_emit_time.clone();
        let current_channel = props.current_channel.clone();
        let prev_channel = use_state(|| props.current_channel.clone());

        if *prev_channel != props.current_channel {
            prev_channel.set(props.current_channel.clone());
            last_read_emitted.set(None);
            last_emit_time.set(0.0);
        }

        use_effect_with((messages, is_user_scrolled_up.clone(), current_channel.clone()), move |(msgs, scrolled_up, _)| {
            if !**scrolled_up {
                if let Some(last_entry) = msgs.last() {
                    let (entry_id, is_pending) = match last_entry {
                        ChannelEntry::Message(m) => (Some(m.id), m.pending),
                        ChannelEntry::UserJoined { id, .. } => (Some(*id), false),
                        _ => (None, false),
                    };

                    if let Some(id) = entry_id {
                        if !is_pending {
                            let now = web_sys::window().unwrap().performance().unwrap().now();
                            let should_emit = last_read_emitted.as_ref() != Some(&id) 
                                              && (*last_emit_time == 0.0 || now - *last_emit_time > 5000.0);

                            if should_emit {
                                on_mark_read.emit((current_channel.clone(), id));
                                last_read_message_id_to_storage(&id);
                                last_read_emitted.set(Some(id));
                                last_emit_time.set(now);
                            }
                        }
                    }
                }
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
        let has_attachments = !ctx.state.pending_attachments.is_empty();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let text = (*input_value).clone();
            if !text.is_empty() || has_attachments {
                on_send_message.emit(text);
                input_value.set(String::new());
            }
        })
    };

    let on_drag_over = Callback::from(|e: DragEvent| {
        e.prevent_default();
    });

    let on_drop = Callback::from(move |e: DragEvent| {
        e.prevent_default();
        if let Some(data_transfer) = e.data_transfer() {
            let document = web_sys::window().expect("no window").document().expect("no document");
            if let Some(input_element) = document.query_selector("input[data-testid='file-input']").unwrap() {
                let input: web_sys::HtmlInputElement = input_element.dyn_into().unwrap();
                
                if let Ok(new_dt) = web_sys::DataTransfer::new() {
                    if let Some(dropped_files) = data_transfer.files() {
                        let items = new_dt.items();
                        for i in 0..dropped_files.length() {
                            if let Some(file) = dropped_files.get(i) {
                                let _ = items.add_with_file(&file);
                            }
                        }
                        input.set_files(new_dt.files().as_ref());
                        let init = web_sys::EventInit::new();
                        init.set_bubbles(true);
                        if let Ok(event) = web_sys::Event::new_with_event_init_dict(
                            "change",
                            &init
                        ) {
                            let _ = input.dispatch_event(&event);
                        }
                    }
                }
            }
        }
    });

    let on_paste = Callback::from(move |e: Event| {
        if let Ok(ce) = e.clone().dyn_into::<ClipboardEvent>() {
            if let Some(data_transfer) = ce.clipboard_data() {
                if let Some(files) = data_transfer.files() {
                    if files.length() > 0 {
                        ce.prevent_default();
                        let document = web_sys::window().expect("no window").document().expect("no document");
                        if let Some(input_element) = document.query_selector("input[data-testid='file-input']").unwrap() {
                            let input: web_sys::HtmlInputElement = input_element.dyn_into().unwrap();
                            
                            if let Ok(new_dt) = web_sys::DataTransfer::new() {
                                let items = new_dt.items();
                                for i in 0..files.length() {
                                    if let Some(file) = files.get(i) {
                                        let _ = items.add_with_file(&file);
                                    }
                                }
                                input.set_files(new_dt.files().as_ref());
                                let init = web_sys::EventInit::new();
                                init.set_bubbles(true);
                                if let Ok(event) = web_sys::Event::new_with_event_init_dict(
                                    "change",
                                    &init
                                ) {
                                    let _ = input.dispatch_event(&event);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    html! {
        <div class="chat-container" data-testid="chat-area" ondragover={on_drag_over} ondrop={on_drop}>
            <header data-testid="chat-header">
                <h1>{ format!("# {}", props.current_channel) }</h1>
                <ProfileMenu username={props.current_username.clone()} />
            </header>
            <div class="chat-history" data-testid="chat-history" ref={history_ref} onscroll={on_scroll}>
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
                                <div key={msg.id.to_string()} class={wrapper_class} data-testid="chat-message">
                                    <div class="meta">
                                        <span class="user">{ &msg.user }</span>
                                        <span class="time">{ time_str }</span>
                                    </div>
                                    <div class="message">{ &msg.message }</div>
                                    if !msg.attachments.is_empty() {
                                        <AttachmentRenderer attachments={msg.attachments.clone()} />
                                    }
                                </div>
                            }
                        },
                        ChannelEntry::UserJoined { id, username, timestamp, .. } => {
                            let local_time: DateTime<Local> = DateTime::from(*timestamp);
                            let time_str = local_time.format("%H:%M:%S").to_string();
                            html! {
                                <div key={id.to_string()} class="message-wrapper system" data-testid="system-message">
                                    <div class="system-message">
                                        { format!("{} has joined", username) }
                                        <span class="time">{ format!(" ({})", time_str) }</span>
                                    </div>
                                </div>
                            }
                        }
                        ChannelEntry::Metadata { .. } | ChannelEntry::Batch(_) | ChannelEntry::ReadMarker { .. } => html! {}
                    }
                }) }
            </div>
            if *show_new_messages_notification {
                <div class="new-messages-notification" onclick={scroll_to_bottom}>
                    { "New messages ↓" }
                </div>
            }
            <footer data-testid="chat-footer">
                if let Some(err) = &ctx.state.attachment_error {
                    <div class="attachment-error" data-testid="attachment-error" style="color: red; padding: 4px;">{ err }</div>
                }
                if !ctx.state.pending_attachments.is_empty() {
                    <div class="pending-attachments" style="display: flex; gap: 8px; padding: 4px;">
                        { for ctx.state.pending_attachments.iter().map(|att| {
                            let id = att.id;
                            let dispatch = ctx.dispatch.clone();
                            let on_remove = Callback::from(move |_| {
                                dispatch.emit(ChatAction::RemovePendingAttachment(id));
                            });
                            html! {
                                <div class="pending-attachment" data-testid={format!("pending-attachment-{}", id)} style="background: #eee; padding: 4px; border-radius: 4px; display: flex; align-items: center; gap: 4px;">
                                    <span>{ &att.original_filename }</span>
                                    <button onclick={on_remove} style="border: none; background: transparent; cursor: pointer; color: red;">{ "x" }</button>
                                </div>
                            }
                        }) }
                    </div>
                }
                <form onsubmit={on_submit} style="display: flex; gap: 8px; width: 100%;">
                    <AttachmentButton />
                    <input
                        type="text"
                        ref={input_ref}
                        data-testid="chat-input"
                        value={(*input_value).clone()}
                        oninput={on_input}
                        onpaste={on_paste}
                        style="flex-grow: 1;"
                        placeholder={format!("Message #{}", props.current_channel)}
                    />
                    <button type="submit" data-testid="send-message-button" disabled={input_value.is_empty() && ctx.state.pending_attachments.is_empty()}>{ "Send" }</button>
                </form>
            </footer>
        </div>
    }
}

fn last_read_message_id_to_storage(_id: &Uuid) {
    // Optional: persist to local storage if desired, but for now we just emit to STOMP
}
