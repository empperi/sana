#[derive(Debug, PartialEq)]
pub enum StompCommand {
    Connect,
    Subscribe { 
        destination: String, 
        last_seen_id: Option<String>, 
        last_seen_seq: Option<u64>,
        headers: Vec<(String, String)> 
    },
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
            let mut last_seen_id = None;
            let mut last_seen_seq = None;
            let mut headers = Vec::new();
            for line in lines {
                if line.is_empty() { break; }
                if let Some((key, value)) = line.split_once(':') {
                    let k = key.trim();
                    let v = value.trim();
                    if k == "destination" {
                        destination = v.to_string();
                    } else if k == "last_seen_id" {
                        last_seen_id = Some(v.to_string());
                    } else if k == "last_seen_seq" {
                        if v.is_empty() {
                            last_seen_seq = None;
                        } else {
                            last_seen_seq = v.parse::<u64>().ok();
                        }
                    }
                    headers.push((k.to_string(), v.to_string()));
                }
            }
            if !destination.is_empty() {
                StompCommand::Subscribe { destination, last_seen_id, last_seen_seq, headers }
            } else {
                StompCommand::Unknown
            }
        }
        "SEND" => {
            let mut destination = String::new();
            let mut body = String::new();
            let mut headers = Vec::new();
            let mut lines_iter = lines.peekable();

            while let Some(line) = lines_iter.next() {
                if line.is_empty() {
                    // Body starts after the first empty line
                    if let Some(next_line) = lines_iter.next() {
                        body = next_line.trim_end_matches('\0').to_string();
                    }
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    let k = key.trim();
                    let v = value.trim();
                    if k == "destination" {
                        destination = v.to_string();
                    }
                    headers.push((k.to_string(), v.to_string()));
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
