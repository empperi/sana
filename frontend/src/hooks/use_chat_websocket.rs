use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;

use crate::logic::ChatState;
use crate::services::websocket::{WebSocketService, ConnectionStatus, StompClient};
use crate::types::ChannelEntry;
use crate::stomp;

#[hook]
pub fn use_chat_websocket(
    auth_check_done: bool,
    chat_state: UseStateHandle<ChatState>,
    state_ref: Rc<RefCell<ChatState>>,
) -> Rc<RefCell<Option<Rc<WebSocketService>>>> {
    let ws_service = use_mut_ref(|| None::<Rc<WebSocketService>>);

    {
        let chat_state = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service_ref = ws_service.clone();

        use_effect_with(auth_check_done, move |&done| {
            if done {
                let on_message = create_on_message_callback(chat_state.clone(), state_ref.clone());
                let on_system_message = create_on_system_message_callback(chat_state.clone(), state_ref.clone(), ws_service_ref.clone());
                let on_connected = create_on_connected_callback(chat_state.clone(), state_ref.clone());
                let on_status_change = create_on_status_change_callback(chat_state.clone(), state_ref.clone(), ws_service_ref.clone());

                let service = Rc::new(WebSocketService::connect(on_message, on_system_message, on_connected, on_status_change));
                *ws_service_ref.borrow_mut() = Some(service);
            }
            
            let ws_service_ref = ws_service_ref.clone();
            move || {
                if let Some(service) = ws_service_ref.borrow_mut().take() {
                    service.stop();
                }
            }
        });
    }

    {
        let chat_state_val = chat_state.clone();
        let state_ref = state_ref.clone();
        let ws_service_ref = ws_service.clone();
        let channels = chat_state.channels.clone();
        let status = chat_state.connection_status;

        use_effect_with((channels, status), move |(channels, status)| {
            if *status == ConnectionStatus::Connected {
                if let Some(service) = &*ws_service_ref.borrow() {
                    let mut state = (*state_ref.borrow()).clone();
                    let mut changed = false;

                    for channel in channels {
                        if !state.subscribed_channels.contains(channel) {
                            let entries = state.messages.get(channel);
                            let last_seq = entries.and_then(|e| e.iter().rev().find_map(|entry| {
                                if let ChannelEntry::Message(m) = entry { m.seq } else { None }
                            }));
                            service.send(stomp::create_subscribe_frame(channel, None, last_seq));
                            state.subscribed_channels.insert(channel.clone());
                            changed = true;
                        }
                    }

                    if changed {
                        *state_ref.borrow_mut() = state.clone();
                        chat_state_val.set(state);
                    }
                }
            }
            || {}
        });
    }

    ws_service
}

fn create_on_message_callback(chat_state: UseStateHandle<ChatState>, state_ref: Rc<RefCell<ChatState>>) -> Callback<(String, ChannelEntry)> {
    Callback::from(move |(channel, entry)| {
        let mut state = (*state_ref.borrow()).clone();
        state.handle_message(channel, entry);
        *state_ref.borrow_mut() = state.clone();
        chat_state.set(state);
    })
}

fn create_on_system_message_callback(
    chat_state: UseStateHandle<ChatState>, 
    state_ref: Rc<RefCell<ChatState>>, 
    ws_service_ref: Rc<RefCell<Option<Rc<WebSocketService>>>>
) -> Callback<(String, String)> {
    Callback::from(move |(_topic, body): (String, String)| {
        let mut state = (*state_ref.borrow()).clone();
        if let Some(channel_name) = state.handle_system_message(body) {
            if let Some(service) = &*ws_service_ref.borrow() {
                service.send(stomp::create_subscribe_frame(&channel_name, None, None));
            }
            *state_ref.borrow_mut() = state.clone();
            chat_state.set(state);
        } else {
            let current_state = (*state_ref.borrow()).clone();
            if state != current_state {
                *state_ref.borrow_mut() = state.clone();
                chat_state.set(state);
            }
        }
    })
}

fn create_on_connected_callback(chat_state: UseStateHandle<ChatState>, state_ref: Rc<RefCell<ChatState>>) -> Callback<(String, Uuid)> {
    Callback::from(move |(username, user_id)| {
        let mut state = (*state_ref.borrow()).clone();
        state.set_user_info(username, user_id);
        *state_ref.borrow_mut() = state.clone();
        chat_state.set(state);
    })
}

fn create_on_status_change_callback(
    chat_state: UseStateHandle<ChatState>, 
    state_ref: Rc<RefCell<ChatState>>, 
    ws_service_ref: Rc<RefCell<Option<Rc<WebSocketService>>>>
) -> Callback<ConnectionStatus> {
    Callback::from(move |status| {
        let mut state = (*state_ref.borrow()).clone();
        state.set_connection_status(status);
        if status == ConnectionStatus::Connected {
            if let Some(service) = &*ws_service_ref.borrow() {
                service.send(stomp::create_subscribe_frame("system.channels", None, None));
            }
        } else {
            state.subscribed_channels.clear();
        }
        *state_ref.borrow_mut() = state.clone();
        chat_state.set(state);
    })
}
