use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;

use crate::services::websocket::{WebSocketService, ConnectionStatus, StompClient};
use crate::types::ChannelEntry;
use crate::stomp;
use crate::state::{ChatStateContext, ChatAction};

#[hook]
pub fn use_chat_websocket(
    auth_check_done: bool,
) -> Rc<RefCell<Option<Rc<WebSocketService>>>> {
    let ctx = use_context::<ChatStateContext>().expect("ChatStateContext not found");
    let ws_service = use_mut_ref(|| None::<Rc<WebSocketService>>);

    {
        let dispatch = ctx.dispatch.clone();
        let ws_service_ref = ws_service.clone();

        use_effect_with(auth_check_done, move |&done| {
            if done {
                let on_message = create_on_message_callback(dispatch.clone());
                let on_system_message = create_on_system_message_callback(dispatch.clone());
                let on_connected = create_on_connected_callback(dispatch.clone());
                let on_status_change = create_on_status_change_callback(dispatch.clone(), ws_service_ref.clone());

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
        let dispatch = ctx.dispatch.clone();
        let ws_service_ref = ws_service.clone();
        let channels = ctx.state.channels.clone();
        let status = ctx.state.connection_status;
        let subscribed_channels = ctx.state.subscribed_channels.clone();
        let messages = ctx.state.messages.clone();

        use_effect_with((channels, status), move |(channels, status)| {
            if *status == ConnectionStatus::Connected {
                if let Some(service) = &*ws_service_ref.borrow() {
                    for channel in channels {
                        if !subscribed_channels.contains(channel) {
                            let last_seq = messages.get(channel).and_then(|e| e.iter().rev().find_map(|entry| {
                                if let ChannelEntry::Message(m) = entry { m.seq } else { None }
                            }));
                            service.send(stomp::create_subscribe_frame(channel, None, last_seq));
                            dispatch.emit(ChatAction::AddSubscribedChannel(channel.clone()));
                        }
                    }
                }
            }
            || {}
        });
    }

    ws_service
}

fn create_on_message_callback(dispatch: Callback<ChatAction>) -> Callback<(String, ChannelEntry)> {
    Callback::from(move |(channel, entry)| {
        dispatch.emit(ChatAction::HandleMessage { channel, entry });
    })
}

fn create_on_system_message_callback(
    dispatch: Callback<ChatAction>, 
) -> Callback<(String, String)> {
    Callback::from(move |(_topic, body): (String, String)| {
        dispatch.emit(ChatAction::HandleSystemMessage(body));
    })
}

fn create_on_connected_callback(dispatch: Callback<ChatAction>) -> Callback<(String, Uuid)> {
    Callback::from(move |(username, user_id)| {
        dispatch.emit(ChatAction::SetUserInfo { username, user_id });
    })
}

fn create_on_status_change_callback(
    dispatch: Callback<ChatAction>, 
    ws_service_ref: Rc<RefCell<Option<Rc<WebSocketService>>>>
) -> Callback<ConnectionStatus> {
    Callback::from(move |status| {
        dispatch.emit(ChatAction::SetConnectionStatus(status));
        if status == ConnectionStatus::Connected {
            if let Some(service) = &*ws_service_ref.borrow() {
                let service: &Rc<WebSocketService> = service;
                service.send(stomp::create_subscribe_frame("system.channels", None, None));
            }
        } else {
            dispatch.emit(ChatAction::ClearSubscriptions);
        }
    })
}
