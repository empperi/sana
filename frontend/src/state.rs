use yew::prelude::*;
use std::rc::Rc;
pub use crate::logic::ChatAction;
use crate::logic::ChatState;

#[derive(Clone, PartialEq)]
pub struct ChatStateContext {
    pub state: UseReducerHandle<ChatState>,
    pub dispatch: Callback<ChatAction>,
}

impl Reducible for ChatState {
    type Action = ChatAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut new_state = (*self).clone();
        match action {
            ChatAction::HandleMessage { channel, entry } => {
                new_state.handle_message(channel, entry);
            }
            ChatAction::PrependHistory { channel, history } => {
                new_state.prepend_historical_messages(channel, history);
            }
            ChatAction::HandleSystemMessage(body) => {
                new_state.handle_system_message(body);
            }
            ChatAction::SelectChannel(channel) => {
                new_state.switch_channel(channel);
            }
            ChatAction::SetConnectionStatus(status) => {
                new_state.set_connection_status(status);
            }
            ChatAction::SetUserInfo { username, user_id } => {
                new_state.set_user_info(username, user_id);
            }
            ChatAction::SetChannels(channels) => {
                new_state.set_channels(channels);
            }
            ChatAction::JoinChannel(channel) => {
                new_state.join_channel(channel);
            }
            ChatAction::AddPendingChannel(name) => {
                new_state.add_pending_channel(name);
            }
            ChatAction::AddPendingMessage { channel, msg } => {
                new_state.add_pending_message(channel, msg);
            }
            ChatAction::AddSubscribedChannel(channel) => {
                new_state.subscribed_channels.insert(channel);
            }
            ChatAction::ClearSubscriptions => {
                new_state.subscribed_channels.clear();
            }
        }
        Rc::new(new_state)
    }
}

pub fn reducer(state: Rc<ChatState>, action: ChatAction) -> Rc<ChatState> {
    state.reduce(action)
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub children: Children,
}

#[function_component(ChatStateProvider)]
pub fn chat_state_provider(props: &Props) -> Html {
    let state = use_reducer(ChatState::new);

    let context = ChatStateContext {
        state: state.clone(),
        dispatch: Callback::from(move |action| state.dispatch(action)),
    };

    html! {
        <ContextProvider<ChatStateContext> context={context}>
            { props.children.clone() }
        </ContextProvider<ChatStateContext>>
    }
}
