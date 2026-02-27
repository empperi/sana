use frontend::logic::*;
use frontend::types::ChatMessage;

#[test]
fn test_initial_state() {
    let state = ChatState::new();
    assert_eq!(state.channels, vec!["test-channel".to_string()]);
    assert_eq!(state.current_channel, "test-channel");
}

#[test]
fn test_handle_message_current_channel() {
    let mut state = ChatState::new();
    let msg = ChatMessage {
        id: "1".to_string(),
        user: "Alice".to_string(),
        timestamp: 100,
        message: "hi".to_string(),
        pending: false,
    };
    
    state.handle_message("test-channel".to_string(), msg.clone());
    
    assert_eq!(state.messages.get("test-channel").unwrap()[0], msg);
    assert!(state.unread_channels.is_empty());
}

#[test]
fn test_handle_message_other_channel() {
    let mut state = ChatState::new();
    state.channels.push("other".to_string());
    let msg = ChatMessage {
        id: "1".to_string(),
        user: "Alice".to_string(),
        timestamp: 100,
        message: "hi".to_string(),
        pending: false,
    };
    
    state.handle_message("other".to_string(), msg);
    
    assert!(state.unread_channels.contains("other"));
}

#[test]
fn test_switch_channel_clears_unread() {
    let mut state = ChatState::new();
    state.unread_channels.insert("test-channel".to_string());
    state.switch_channel("test-channel".to_string());
    assert!(state.unread_channels.is_empty());
}

#[test]
fn test_handle_system_message_adds_channel() {
    let mut state = ChatState::new();
    state.handle_system_message("new-room".to_string());
    assert!(state.channels.contains(&"new-room".to_string()));
}

#[test]
fn test_pending_message_replacement() {
    let mut state = ChatState::new();
    let pending = ChatMessage {
        id: "1".to_string(),
        user: "Alice".to_string(),
        timestamp: 100,
        message: "hi".to_string(),
        pending: true,
    };
    state.add_pending_message("test-channel".to_string(), pending);
    
    let confirmed = ChatMessage {
        id: "1".to_string(),
        user: "Alice".to_string(),
        timestamp: 105,
        message: "hi".to_string(),
        pending: false,
    };
    state.handle_message("test-channel".to_string(), confirmed.clone());
    
    let msgs = state.messages.get("test-channel").unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], confirmed);
}
