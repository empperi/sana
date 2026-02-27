#[derive(Debug, PartialEq, Clone)]
pub enum StompFrame {
    Connected { username: String },
    Message { destination: String, body: String },
    Error(String),
}

pub fn parse_frame(text: &str) -> Option<StompFrame> {
    if text.starts_with("CONNECTED") {
        let mut username = String::new();
        for line in text.lines() {
            if line.starts_with("username:") {
                username = line.strip_prefix("username:").unwrap().trim().to_string();
            }
        }
        if !username.is_empty() {
            return Some(StompFrame::Connected { username });
        }
    } else if text.starts_with("MESSAGE") {
        let mut destination = String::new();
        let parts: Vec<&str> = text.split("\n\n").collect();
        let headers = parts[0];
        for line in headers.lines() {
            if line.starts_with("destination:") {
                destination = line.strip_prefix("destination:").unwrap().trim().to_string();
            }
        }
        let body = if parts.len() > 1 {
            parts[1].trim_end_matches('\0').to_string()
        } else {
            String::new()
        };
        if !destination.is_empty() {
            return Some(StompFrame::Message { destination, body });
        }
    }
    None
}

pub fn create_connect_frame() -> String {
    "CONNECT\naccept-version:1.2\n\n\0".to_string()
}

pub fn create_subscribe_frame(channel: &str) -> String {
    format!("SUBSCRIBE\nid:0\ndestination:/topic/{}\n\n\0", channel)
}

pub fn create_send_frame(channel: &str, message_id: &str, text: &str) -> String {
    format!("SEND\ndestination:/topic/{}\nmessage_id:{}\n\n{}\0", channel, message_id, text)
}
