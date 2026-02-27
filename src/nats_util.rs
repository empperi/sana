pub fn encode(channel_name: &str) -> String {
    // We use hex encoding to make any channel name safe for NATS subjects
    hex::encode(channel_name)
}

pub fn decode(encoded: &str) -> Option<String> {
    let bytes = hex::decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}
