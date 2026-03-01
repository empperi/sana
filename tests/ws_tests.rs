use sana::logic::ws_logic::{decide, WsAction};
use sana::stomp::StompCommand;
use indoc::indoc;
use uuid::Uuid;

#[test]
fn test_decide_connect() {
    let user_id = Uuid::new_v4();
    let actions = decide(StompCommand::Connect, user_id, "User1234");
    let expected_response = format!(indoc! {"
        CONNECTED
        version:1.2
        user_id:{}
        username:User1234

        \0"}, user_id);
    assert_eq!(actions, vec![WsAction::SendToClient(expected_response)]);
}

#[test]
fn test_decide_subscribe() {
    let actions = decide(
        StompCommand::Subscribe { destination: "/topic/foo".to_string(), last_seen_id: None, last_seen_seq: None, headers: vec![] },
        Uuid::new_v4(),
        "User1234"
    );
    assert_eq!(actions, vec![WsAction::Subscribe("foo".to_string(), None)]);
}

#[test]
fn test_decide_subscribe_invalid() {
    let actions = decide(
        StompCommand::Subscribe { destination: "/queue/foo".to_string(), last_seen_id: None, last_seen_seq: None, headers: vec![] },
        Uuid::new_v4(),
        "User1234"
    );
    assert!(actions.is_empty());
}

#[test]
fn test_decide_send() {
    let actions = decide(
        StompCommand::Send { destination: "/topic/foo".to_string(), body: "bar".to_string(), headers: vec![] },
        Uuid::new_v4(),
        "User1234"
    );
    assert_eq!(actions, vec![WsAction::PublishToNats("topic.666f6f".to_string(), "bar".to_string(), None, "foo".to_string())]);
}

#[test]
fn test_decide_send_with_message_id() {
    let headers = vec![("message_id".to_string(), "00000000-0000-0000-0000-000000000001".to_string())];
    let actions = decide(
        StompCommand::Send { destination: "/topic/foo".to_string(), body: "bar".to_string(), headers },
        Uuid::new_v4(),
        "User1234"
    );
    assert_eq!(actions, vec![WsAction::PublishToNats("topic.666f6f".to_string(), "bar".to_string(), Some("00000000-0000-0000-0000-000000000001".to_string()), "foo".to_string())]);
}

#[test]
fn test_decide_send_invalid() {
    let actions = decide(
        StompCommand::Send { destination: "/queue/foo".to_string(), body: "bar".to_string(), headers: vec![] },
        Uuid::new_v4(),
        "User1234"
    );
    assert!(actions.is_empty());
}

#[test]
fn test_decide_unknown() {
    let actions = decide(StompCommand::Unknown, Uuid::new_v4(), "User1234");
    assert!(actions.is_empty());
}
