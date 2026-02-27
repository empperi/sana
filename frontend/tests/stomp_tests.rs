use frontend::stomp::*;

#[test]
fn test_parse_connected() {
    let frame = "CONNECTED\nversion:1.2\nusername:Alice\n\n\0";
    assert_eq!(parse_frame(frame), Some(StompFrame::Connected { username: "Alice".to_string() }));
}

#[test]
fn test_parse_message() {
    let frame = "MESSAGE\ndestination:/topic/test\n\n{\"id\":\"1\",\"user\":\"Alice\",\"timestamp\":123,\"message\":\"hello\"}\0";
    assert_eq!(parse_frame(frame), Some(StompFrame::Message { 
        destination: "/topic/test".to_string(), 
        body: "{\"id\":\"1\",\"user\":\"Alice\",\"timestamp\":123,\"message\":\"hello\"}".to_string() 
    }));
}

#[test]
fn test_create_subscribe() {
    assert_eq!(create_subscribe_frame("general"), "SUBSCRIBE\nid:0\ndestination:/topic/general\n\n\0");
}

#[test]
fn test_create_subscribe_with_spaces() {
    assert_eq!(create_subscribe_frame("My Channel"), "SUBSCRIBE\nid:0\ndestination:/topic/My Channel\n\n\0");
}

#[test]
fn test_parse_message_with_spaces() {
    let frame = "MESSAGE\ndestination:/topic/My Channel\n\n{\"id\":\"1\",\"user\":\"Alice\",\"timestamp\":123,\"message\":\"hello\"}\0";
    assert_eq!(parse_frame(frame), Some(StompFrame::Message { 
        destination: "/topic/My Channel".to_string(), 
        body: "{\"id\":\"1\",\"user\":\"Alice\",\"timestamp\":123,\"message\":\"hello\"}".to_string() 
    }));
}
