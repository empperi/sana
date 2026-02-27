#[derive(Debug, PartialEq)]
pub enum StompCommand {
    Connect,
    Subscribe { destination: String },
    Send { destination: String, body: String, headers: Vec<(String, String)> },
    Unknown,
}

pub fn parse(text: &str) -> StompCommand {
    let mut lines = text.lines();
    let command_line = match lines.next() {
        Some(line) => line.trim(),
        None => return StompCommand::Unknown,
    };

    match command_line {
        "CONNECT" | "STOMP" => StompCommand::Connect,
        "SUBSCRIBE" => {
            let mut destination = String::new();
            for line in lines {
                if line.starts_with("destination:") {
                    destination = line.strip_prefix("destination:").unwrap().trim().to_string();
                }
            }
            if !destination.is_empty() {
                StompCommand::Subscribe { destination }
            } else {
                StompCommand::Unknown
            }
        }
        "SEND" => {
            let mut destination = String::new();
            let mut body = String::new();
            let mut headers = Vec::new();
            let mut body_start = false;

            for line in lines {
                if body_start {
                    body = line.trim_end_matches('\0').to_string();
                    break;
                }

                if line.starts_with("destination:") {
                    destination = line.strip_prefix("destination:").unwrap().trim().to_string();
                } else if line.is_empty() {
                    body_start = true;
                } else if let Some((key, value)) = line.split_once(':') {
                    headers.push((key.trim().to_string(), value.trim().to_string()));
                }
            }

            if !destination.is_empty() {
                StompCommand::Send { destination, body, headers }
            } else {
                StompCommand::Unknown
            }
        }
        _ => StompCommand::Unknown,
    }
}
