use std::str::FromStr;

use super::ContentHash;

#[test]
fn content_hash_hex_round_trips() {
    let hash = ContentHash::new([0xab; 32]);
    assert_eq!(ContentHash::from_str(&hash.to_string()), Ok(hash));
}

#[test]
fn content_hash_rejects_wrong_length_and_non_hex_text() {
    assert!(ContentHash::from_str("ab").is_err());
    assert!(ContentHash::from_str(&"z".repeat(64)).is_err());
}
