use sana::nats_util::{encode, decode};

#[test]
fn test_encode_decode() {
    let name = "My Channel";
    let encoded = encode(name);
    assert!(!encoded.contains(' '));
    assert_eq!(decode(&encoded), Some(name.to_string()));
}

#[test]
fn test_encode_system() {
    let name = "system.channels";
    let encoded = encode(name);
    assert_eq!(decode(&encoded), Some(name.to_string()));
}
