use sana::logic::ws_logic::merge_and_deduplicate;
use sana::messages::{ChannelEntry, ChatMessage, MessageType};
use uuid::Uuid;
use chrono::Utc;

#[test]
fn test_merge_and_deduplicate() {
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    
    let msg1 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id,
        user: "alice".to_string(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        seq: Some(1),
        msg_type: MessageType::Chat,
    };
    
    let msg2 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id,
        user: "bob".to_string(),
        timestamp: Utc::now(),
        message: "world".to_string(),
        seq: Some(2),
        msg_type: MessageType::Chat,
    };
    
    let db_history = vec![
        ChannelEntry::Message(msg1.clone()),
    ];
    
    let mem_history = vec![
        ChannelEntry::Message(msg1.clone()), // duplicate
        ChannelEntry::Message(msg2.clone()),
    ];
    
    let merged = merge_and_deduplicate(db_history, mem_history);
    
    assert_eq!(merged.len(), 2);
    if let ChannelEntry::Message(m) = &merged[0] {
        assert_eq!(m.id, msg1.id);
    }
    if let ChannelEntry::Message(m) = &merged[1] {
        assert_eq!(m.id, msg2.id);
    }
}

#[tokio::test]
async fn test_send_in_batches() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let channel_id = Uuid::new_v4();
    let history: Vec<ChannelEntry> = (0..25).map(|i| {
        ChannelEntry::Message(ChatMessage {
            id: Uuid::new_v4(),
            channel_id,
            user_id: Uuid::new_v4(),
            user: "user".to_string(),
            timestamp: Utc::now(),
            message: format!("msg {}", i),
            seq: Some(i),
            msg_type: MessageType::Chat,
        })
    }).collect();
    
    sana::logic::ws_logic::send_in_batches("test-chan", history, &tx).await;
    
    // Should receive 2 batches (20 + 5)
    let batch1_str = rx.recv().await.unwrap();
    let batch2_str = rx.recv().await.unwrap();
    
    assert!(batch1_str.contains("MESSAGE"));
    assert!(batch2_str.contains("MESSAGE"));
    
    // Simple check for batch sizes in the JSON part
    // Extract JSON part (after double newline)
    let json1 = batch1_str.split("\n\n").nth(1).unwrap().trim_matches('\0');
    let json2 = batch2_str.split("\n\n").nth(1).unwrap().trim_matches('\0');
    
    let entry1: ChannelEntry = serde_json::from_str(json1).unwrap();
    let entry2: ChannelEntry = serde_json::from_str(json2).unwrap();
    
    if let ChannelEntry::Batch(b1) = entry1 {
        assert_eq!(b1.len(), 20);
    } else { panic!("Expected batch"); }
    
    if let ChannelEntry::Batch(b2) = entry2 {
        assert_eq!(b2.len(), 5);
    } else { panic!("Expected batch"); }
}

#[test]
fn test_build_chat_message() {
    let msg_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let username = "tester";
    let body = "hello world";
    
    let msg = sana::logic::ws_logic::build_chat_message(
        msg_id,
        channel_id,
        user_id,
        username,
        body.to_string()
    );
    
    assert_eq!(msg.id, msg_id);
    assert_eq!(msg.channel_id, channel_id);
    assert_eq!(msg.user_id, user_id);
    assert_eq!(msg.user, username);
    assert_eq!(msg.message, body);
    assert_eq!(msg.seq, None);
}
