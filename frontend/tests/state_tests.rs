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

#[test]
fn test_attachment_actions() {
    let state = Rc::new(ChatState::new());
    
    let att1 = frontend::types::AttachmentMeta {
        id: Uuid::new_v4(),
        original_filename: "test1.png".to_string(),
        file_size: 1024,
        mime_type: "image/png".to_string(),
    };
    
    let att2 = frontend::types::AttachmentMeta {
        id: Uuid::new_v4(),
        original_filename: "test2.pdf".to_string(),
        file_size: 2048,
        mime_type: "application/pdf".to_string(),
    };

    // Add first attachment
    let new_state = reducer(state, ChatAction::AddPendingAttachment(att1.clone()));
    assert_eq!(new_state.pending_attachments.len(), 1);
    assert_eq!(new_state.pending_attachments[0].id, att1.id);

    // Add second attachment
    let new_state = reducer(new_state, ChatAction::AddPendingAttachment(att2.clone()));
    assert_eq!(new_state.pending_attachments.len(), 2);
    
    // Set error
    let new_state = reducer(new_state, ChatAction::SetAttachmentError(Some("Too large".to_string())));
    assert_eq!(new_state.attachment_error, Some("Too large".to_string()));

    // Remove first attachment
    let new_state = reducer(new_state, ChatAction::RemovePendingAttachment(att1.id));
    assert_eq!(new_state.pending_attachments.len(), 1);
    assert_eq!(new_state.pending_attachments[0].id, att2.id);

    // Clear all
    let new_state = reducer(new_state, ChatAction::ClearPendingAttachments);
    assert!(new_state.pending_attachments.is_empty());
}

