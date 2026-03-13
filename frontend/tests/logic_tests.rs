use frontend::logic::*;
use frontend::types::{ChatMessage, Channel, ChannelEntry};
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
    let entry = ChannelEntry::Message(msg.clone());
    
    state.handle_message("General".to_string(), entry.clone());
    
    assert_eq!(state.messages.get("General").unwrap()[0], entry);
    assert!(state.unread_channels.is_empty());
}


#[test]
fn test_prepend_historical_messages() {
    let mut state = ChatState::new();
    let channel_id = Uuid::new_v4();
    state.channel_id_map.insert("General".to_string(), channel_id);

    let msg1 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id: Uuid::new_v4(),
        user: "Alice".to_string(),
        timestamp: Utc::now() - chrono::Duration::minutes(20),
        message: "msg 1 (oldest)".to_string(),
        pending: false,
        seq: Some(1),
    };
    let msg2 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id: Uuid::new_v4(),
        user: "Alice".to_string(),
        timestamp: Utc::now() - chrono::Duration::minutes(10),
        message: "msg 2 (middle)".to_string(),
        pending: false,
        seq: Some(2),
    };
    let msg3 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id: Uuid::new_v4(),
        user: "Bob".to_string(),
        timestamp: Utc::now(),
        message: "msg 3 (newest)".to_string(),
        pending: false,
        seq: Some(3),
    };

    // Add msg3 first (simulating what's already in the channel)
    state.handle_message("General".to_string(), ChannelEntry::Message(msg3.clone()));

    // Prepend msg2 and msg1 in DESC order (simulating REST API response: newest of history first)
    let history = vec![
        ChannelEntry::Message(msg2.clone()),
        ChannelEntry::Message(msg1.clone()),
    ];
    state.prepend_historical_messages("General".to_string(), history);

    let msgs = state.messages.get("General").unwrap();
    assert_eq!(msgs.len(), 3);

    // Final order should be ASC: msg1, msg2, msg3
    if let ChannelEntry::Message(m1) = &msgs[0] {
        assert_eq!(m1.message, "msg 1 (oldest)");
    } else { panic!("expected message 1"); }

    if let ChannelEntry::Message(m2) = &msgs[1] {
        assert_eq!(m2.message, "msg 2 (middle)");
    } else { panic!("expected message 2"); }

    if let ChannelEntry::Message(m3) = &msgs[2] {
        assert_eq!(m3.message, "msg 3 (newest)");
    } else { panic!("expected message 3"); }
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
    let entry = ChannelEntry::Message(msg);
    
    state.handle_message("other".to_string(), entry);
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
fn test_handle_system_message_updates_metadata_only() {
    let mut state = ChatState::new();
    let channel = Channel {
        id: Uuid::new_v4(),
        name: "new-room".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let payload = serde_json::to_string(&channel).unwrap();
    let result = state.handle_system_message(payload);
    
    // Should NOT return a channel name for subscription
    assert_eq!(result, None);
    // Should NOT be in the sidebar channels list
    assert!(!state.channels.contains(&"new-room".to_string()));
    // SHOULD be in the metadata map for when the user decides to join
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

    // 3. State should be updated, but result should be None because we already have it
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
fn test_join_channel() {
    let mut state = ChatState::new();
    let c1 = Channel {
        id: Uuid::new_v4(),
        name: "new-joined".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    
    state.join_channel(c1.clone());
    
    assert!(state.channels.contains(&"new-joined".to_string()));
    assert_eq!(state.channel_id_map.get("new-joined"), Some(&c1.id));
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
    let confirmed_msg = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "User1111".to_string(),
        timestamp: Utc::now(),
        message: "hi".to_string(),
        pending: false,
        seq: None,
    };
    let confirmed_entry = ChannelEntry::Message(confirmed_msg.clone());
    state.handle_message("General".to_string(), confirmed_entry.clone());
    
    let entries = state.messages.get("General").unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], confirmed_entry);
    
    if let ChannelEntry::Message(ref m) = entries[0] {
        assert!(!m.pending);
        assert_eq!(m.user, "User1111");
    } else {
        panic!("Expected Message variant");
    }
}

#[test]
fn test_message_update_with_seq() {
    let mut state = ChatState::new();
    let msg_id = Uuid::new_v4();
    let initial_msg = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "Alice".to_string(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        pending: false,
        seq: None,
    };
    state.handle_message("General".to_string(), ChannelEntry::Message(initial_msg));
    
    let updated_msg = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "Alice".to_string(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        pending: false,
        seq: Some(123),
    };
    let updated_entry = ChannelEntry::Message(updated_msg);
    state.handle_message("General".to_string(), updated_entry.clone());
    
    let entries = state.messages.get("General").unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], updated_entry);
    
    if let ChannelEntry::Message(ref m) = entries[0] {
        assert_eq!(m.seq, Some(123));
    } else {
        panic!("Expected Message variant");
    }
}
