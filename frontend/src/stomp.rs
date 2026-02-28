#[derive(Debug, PartialEq, Clone)]
pub enum StompFrame {
    Connected { username: String },
    Message { destination: String, body: String, seq: Option<u64> },
    Receipt { receipt_id: String },
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
    } else if text.starts_with("RECEIPT") {
        let mut receipt_id = String::new();
        for line in text.lines() {
            if line.starts_with("receipt-id:") {
                receipt_id = line.strip_prefix("receipt-id:").unwrap().trim().to_string();
            }
        }
        if !receipt_id.is_empty() {
            return Some(StompFrame::Receipt { receipt_id });
        }
    } else if text.starts_with("MESSAGE") {
        let mut destination = String::new();
        let mut seq = None;
        let parts: Vec<&str> = text.split("\n\n").collect();
        let headers = parts[0];
        for line in headers.lines() {
            if line.starts_with("destination:") {
                destination = line.strip_prefix("destination:").unwrap().trim().to_string();
            } else if line.starts_with("seq:") {
                seq = line.strip_prefix("seq:").unwrap().trim().parse::<u64>().ok();
            }
        }
        let body = if parts.len() > 1 {
            parts[1].trim_end_matches('\0').to_string()
        } else {
            String::new()
        };
        if !destination.is_empty() {
            return Some(StompFrame::Message { destination, body, seq });
        }
    }
    None
}

pub fn create_connect_frame() -> String {
    "CONNECT\naccept-version:1.2\n\n\0".to_string()
}

pub fn create_subscribe_frame(channel: &str, receipt_id: Option<&str>, last_seen_seq: Option<u64>) -> String {
    let mut frame = format!("SUBSCRIBE\nid:0\ndestination:/topic/{}\n", channel);
    if let Some(rid) = receipt_id {
        frame.push_str(&format!("receipt:{}\n", rid));
    }
    
    let seq_val = last_seen_seq.map(|s| s.to_string()).unwrap_or_default();
    frame.push_str(&format!("last_seen_seq:{}\n", seq_val));
    
    frame.push_str("\n\0");
    frame
}

pub fn create_send_frame(channel: &str, message_id: &str, text: &str) -> String {
    format!("SEND\ndestination:/topic/{}\nmessage_id:{}\n\n{}\0", channel, message_id, text)
}
