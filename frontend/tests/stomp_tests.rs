use frontend::stomp::*;

#[test]
fn test_parse_connected() {
    let frame = "CONNECTED\nversion:1.2\nusername:alice\n\n\0";
    assert_eq!(parse_frame(frame), Some(StompFrame::Connected { username: "alice".to_string() }));
}

#[test]
fn test_parse_message() {
    let frame = "MESSAGE\ndestination:/topic/general\nseq:42\n\n{\"id\":\"1\",\"user\":\"alice\",\"timestamp\":100,\"message\":\"hi\"}\0";
    assert_eq!(parse_frame(frame), Some(StompFrame::Message {
        destination: "/topic/general".to_string(),
        body: "{\"id\":\"1\",\"user\":\"alice\",\"timestamp\":100,\"message\":\"hi\"}".to_string(),
        seq: Some(42)
    }));
}

#[test]
fn test_create_subscribe() {
    assert_eq!(create_subscribe_frame("general", None, None), "SUBSCRIBE\nid:0\ndestination:/topic/general\nlast_seen_seq:\n\n\0");
}

#[test]
fn test_create_subscribe_with_spaces() {
    assert_eq!(create_subscribe_frame("My Channel", None, None), "SUBSCRIBE\nid:0\ndestination:/topic/My Channel\nlast_seen_seq:\n\n\0");
}

#[test]
fn test_parse_message_with_spaces() {
    let frame = "MESSAGE\ndestination:/topic/My Channel\nseq:100\n\nhi\0";
    assert_eq!(parse_frame(frame), Some(StompFrame::Message {
        destination: "/topic/My Channel".to_string(),
        body: "hi".to_string(),
        seq: Some(100)
    }));
}
