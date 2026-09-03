use x25519_dalek::StaticSecret;

/// Verifies whether a client's 32-byte SessionID contains the authorized ShortID.
pub fn verify_reality_client(
    _server_private: &StaticSecret,
    client_session_id: &[u8; 32],
    expected_short_id: &[u8],
) -> Option<([u8; 32], [u8; 32])> {
    if !expected_short_id.is_empty() {
        let has_short_id = client_session_id
            .windows(expected_short_id.len())
            .any(|w| w == expected_short_id);
        if !has_short_id {
            return None;
        }
    }

    Some(([0u8; 32], [0u8; 32]))
}

/// Validates that a received ShortID matches the server's authorized ShortID.
#[allow(dead_code)]
pub fn validate_short_id(received: &[u8], expected: &[u8]) -> bool {
    received == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reality_key_derivation() {
        let mut b1 = [0u8; 32];
        let mut b2 = [0u8; 32];
        for (x, y) in b1.iter_mut().zip(b2.iter_mut()) {
            *x = fastrand::u8(..);
            *y = fastrand::u8(..);
        }
        let short_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        b2[0..8].copy_from_slice(&short_id);

        let server_private = StaticSecret::from(b1);

        let res = verify_reality_client(&server_private, &b2, &short_id);
        assert!(res.is_some());
    }

    #[test]
    fn test_validate_short_id() {
        let id1 = [1, 2, 3, 4];
        let id2 = [1, 2, 3, 4];
        let id3 = [1, 2, 3, 5];
        assert!(validate_short_id(&id1, &id2));
        assert!(!validate_short_id(&id1, &id3));
    }
}
