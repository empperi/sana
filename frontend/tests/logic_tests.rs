use frontend::logic::*;
use frontend::types::{ChatMessage, Channel};
use uuid::Uuid;
use chrono::Utc;

#[test]
fn test_initial_state() {
    let state = ChatState::new();
    assert_eq!(state.channels, vec!["General".to_string()]);
    assert_eq!(state.current_channel, "General");
}

#[test]
fn test_handle_message_current_channel() {
    let mut state = ChatState::new();
    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "Alice".to_string(),
        timestamp: Utc::now(),
        message: "hi".to_string(),
        pending: false,
        seq: None,
    };
    
    state.handle_message("General".to_string(), msg.clone());
    
    assert_eq!(state.messages.get("General").unwrap()[0], msg);
    assert!(state.unread_channels.is_empty());
}

#[test]
fn test_handle_message_other_channel() {
    let mut state = ChatState::new();
    state.channels.push("other".to_string());
    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "Alice".to_string(),
        timestamp: Utc::now(),
        message: "hi".to_string(),
        pending: false,
        seq: None,
    };
    
    state.handle_message("other".to_string(), msg);
    
    assert!(state.unread_channels.contains("other"));
}

#[test]
fn test_switch_channel_clears_unread() {
    let mut state = ChatState::new();
    state.unread_channels.insert("General".to_string());
    state.switch_channel("General".to_string());
    assert!(state.unread_channels.is_empty());
}

#[test]
fn test_handle_system_message_adds_channel() {
    let mut state = ChatState::new();
    let channel = Channel {
        id: Uuid::new_v4(),
        name: "new-room".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let payload = serde_json::to_string(&channel).unwrap();
    let result = state.handle_system_message(payload);
    
    assert_eq!(result, Some("new-room".to_string()));
    assert!(state.channels.contains(&"new-room".to_string()));
    assert_eq!(state.channel_id_map.get("new-room"), Some(&channel.id));
}

#[test]
fn test_channel_creation_pending_and_confirmation() {
    let mut state = ChatState::new();
    let channel_name = "local-room".to_string();
    
    // 1. User creates channel locally
    state.add_pending_channel(channel_name.clone());
    assert!(state.channels.contains(&channel_name));
    assert!(state.pending_channels.contains(&channel_name));
    assert!(state.channel_id_map.get(&channel_name).is_none());

    // 2. Backend confirms channel via NATS/STOMP
    let confirmed_id = Uuid::new_v4();
    let channel = Channel {
        id: confirmed_id,
        name: channel_name.clone(),
        is_private: false,
        created_at: Utc::now(),
    };
    let payload = serde_json::to_string(&channel).unwrap();
    let result = state.handle_system_message(payload);

    // 3. State should be updated, but result should be None because we already have it (no new subscription needed)
    assert_eq!(result, None);
    assert!(state.channels.contains(&channel_name));
    assert!(!state.pending_channels.contains(&channel_name));
    assert_eq!(state.channel_id_map.get(&channel_name), Some(&confirmed_id));
}

#[test]
fn test_handle_system_message_ignore_duplicates() {
    let mut state = ChatState::new();
    let channel = Channel {
        id: Uuid::new_v4(),
        name: "General".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let payload = serde_json::to_string(&channel).unwrap();
    
    // Should return None because "General" already exists
    let result = state.handle_system_message(payload);
    assert_eq!(result, None);
}

#[test]
fn test_set_channels() {
    let mut state = ChatState::new();
    let c1 = Channel {
        id: Uuid::new_v4(),
        name: "test-1".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    
    state.set_channels(vec![c1.clone()]);
    
    assert!(state.channels.contains(&"test-1".to_string()));
    assert!(state.channels.contains(&"General".to_string())); // Should be preserved
    assert_eq!(state.channel_id_map.get("test-1"), Some(&c1.id));
}

#[test]
fn test_pending_message_replacement_different_user_id() {
    let mut state = ChatState::new();
    let msg_id = Uuid::new_v4();
    let pending = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "User9999".to_string(),
        timestamp: Utc::now(),
        message: "hi".to_string(),
        pending: true,
        seq: None,
    };
    state.add_pending_message("General".to_string(), pending);
    
    // On reconnect, we might get a different user_id
    let confirmed = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "User1111".to_string(),
        timestamp: Utc::now(),
        message: "hi".to_string(),
        pending: false,
        seq: None,
    };
    state.handle_message("General".to_string(), confirmed.clone());
    
    let msgs = state.messages.get("General").unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], confirmed);
    assert!(!msgs[0].pending);
    assert_eq!(msgs[0].user, "User1111");
}
