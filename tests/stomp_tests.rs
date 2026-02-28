use sana::stomp::{parse, StompCommand};
use indoc::indoc;

#[test]
fn test_parse_connect() {
    let msg = indoc! {"
        CONNECT
        accept-version:1.2

        \0"};
    assert_eq!(parse(msg), StompCommand::Connect);

    let msg2 = indoc! {"
        STOMP
        accept-version:1.2

        \0"};
    assert_eq!(parse(msg2), StompCommand::Connect);
}

#[test]
fn test_parse_subscribe() {
    let msg = indoc! {"
        SUBSCRIBE
        id:0
        destination:/topic/foo

        \0"};
    assert_eq!(
        parse(msg),
        StompCommand::Subscribe {
            destination: "/topic/foo".to_string(),
            last_seen_id: None,
            last_seen_seq: None,
            headers: vec![("id".to_string(), "0".to_string()), ("destination".to_string(), "/topic/foo".to_string())]
        }
    );
}

#[test]
fn test_parse_send() {
    let msg = indoc! {"
        SEND
        destination:/topic/foo

        hello world\0"};
    assert_eq!(
        parse(msg),
        StompCommand::Send {
            destination: "/topic/foo".to_string(),
            body: "hello world".to_string(),
            headers: vec![("destination".to_string(), "/topic/foo".to_string())]
        }
    );
}

#[test]
fn test_parse_send_with_headers() {
    let msg = indoc! {"
        SEND
        destination:/topic/foo
        message_id:12345

        hello world\0"};
    assert_eq!(
        parse(msg),
        StompCommand::Send {
            destination: "/topic/foo".to_string(),
            body: "hello world".to_string(),
            headers: vec![
                ("destination".to_string(), "/topic/foo".to_string()),
                ("message_id".to_string(), "12345".to_string())
            ]
        }
    );
}

#[test]
fn test_parse_unknown() {
    assert_eq!(parse("FOOBAR\n\n"), StompCommand::Unknown);
    assert_eq!(parse(""), StompCommand::Unknown);
}

#[test]
fn test_parse_incomplete_subscribe() {
    let msg = indoc! {"
        SUBSCRIBE
        id:0

        \0"};
    assert_eq!(parse(msg), StompCommand::Unknown);
}

#[test]
fn test_parse_subscribe_with_spaces() {
    let msg = indoc! {"
        SUBSCRIBE
        destination:/topic/My Channel

        \0"};
    assert_eq!(
        parse(msg),
        StompCommand::Subscribe {
            destination: "/topic/My Channel".to_string(),
            last_seen_id: None,
            last_seen_seq: None,
            headers: vec![("destination".to_string(), "/topic/My Channel".to_string())]
        }
    );
}

#[test]
fn test_parse_send_with_spaces() {
    let msg = indoc! {"
        SEND
        destination:/topic/My Channel

        hello world\0"};
    assert_eq!(
        parse(msg),
        StompCommand::Send {
            destination: "/topic/My Channel".to_string(),
            body: "hello world".to_string(),
            headers: vec![("destination".to_string(), "/topic/My Channel".to_string())]
        }
    );
}
