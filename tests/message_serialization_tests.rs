use sana::messages::{ChatMessage, MessageType, AttachmentMeta};
use uuid::Uuid;
use chrono::Utc;

#[test]
fn test_chat_message_serde_with_attachments() {
    let msg_id = Uuid::new_v4();
    let att_id = Uuid::new_v4();
    
    let msg = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "tester".to_string(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        seq: Some(1),
        msg_type: MessageType::Chat,
        attachments: vec![
            AttachmentMeta {
                id: att_id,
                original_filename: "test.png".to_string(),
                file_size: 1024,
                mime_type: "image/png".to_string(),
            }
        ],
    };

    let serialized = serde_json::to_string(&msg).unwrap();
    let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

    assert_eq!(msg, deserialized);
    assert_eq!(deserialized.attachments.len(), 1);
    assert_eq!(deserialized.attachments[0].id, att_id);
}

#[test]
fn test_chat_message_serde_backward_compatibility() {
    // JSON payload without the "attachments" field
    let json = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "channel_id": "550e8400-e29b-41d4-a716-446655440001",
        "user_id": "550e8400-e29b-41d4-a716-446655440002",
        "user": "old_user",
        "timestamp": "2026-04-16T12:00:00Z",
        "message": "old message",
        "seq": 100,
        "msg_type": "Chat"
    }"#;

    let deserialized: ChatMessage = serde_json::from_str(json).unwrap();

    assert_eq!(deserialized.user, "old_user");
    assert_eq!(deserialized.message, "old message");
    assert!(deserialized.attachments.is_empty(), "Attachments should be empty by default");
}

#[test]
fn test_chat_message_serde_empty_attachments() {
    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "tester".to_string(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        seq: None,
        msg_type: MessageType::Chat,
        attachments: vec![],
    };

    let serialized = serde_json::to_string(&msg).unwrap();
    let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

    assert_eq!(msg, deserialized);
    assert!(deserialized.attachments.is_empty());
}
