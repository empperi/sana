use frontend::state::*;
use frontend::logic::ChatState;
use frontend::types::Channel;
use uuid::Uuid;
use chrono::Utc;
use std::rc::Rc;

#[test]
fn test_chat_action_set_channels() {
    let state = Rc::new(ChatState::new());
    let channels = vec![
        Channel {
            id: Uuid::new_v4(),
            name: "General".to_string(),
            is_private: false,
            created_at: Utc::now(),
        },
        Channel {
            id: Uuid::new_v4(),
            name: "Rust".to_string(),
            is_private: false,
            created_at: Utc::now(),
        },
    ];
    
    let action = ChatAction::SetChannels(channels);
    let new_state = reducer(state, action);
    
    assert_eq!(new_state.channels.len(), 2);
    assert!(new_state.channels.contains(&"General".to_string()));
    assert!(new_state.channels.contains(&"Rust".to_string()));
}

#[test]
fn test_chat_action_switch_channel() {
    let mut state = ChatState::new();
    state.channels.push("General".to_string());
    state.channels.push("Rust".to_string());
    let state = Rc::new(state);
    
    let action = ChatAction::SelectChannel("Rust".to_string());
    let new_state = reducer(state, action);
    
    assert_eq!(new_state.current_channel, "Rust");
}
