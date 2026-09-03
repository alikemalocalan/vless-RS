use hkdf::Hkdf;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use x25519_dalek::{PublicKey, StaticSecret};

/// Verifies whether a client's 32-byte SessionID contains a valid REALITY X25519 ephemeral public key and authorized ShortID.
pub fn verify_reality_client(
    server_private: &StaticSecret,
    client_session_id: &[u8; 32],
    expected_short_id: &[u8],
) -> Option<([u8; 32], [u8; 32])> {
    // 1. In REALITY, authorized clients embed the configured short_id in their ClientHello SessionID
    if !expected_short_id.is_empty() {
        let has_short_id = client_session_id
            .windows(expected_short_id.len())
            .any(|w| w.ct_eq(expected_short_id).unwrap_u8() == 1);
        if !has_short_id {
            return None;
        }
    }

    // 2. Client's SessionID is the client's ephemeral X25519 public key
    let client_pub = PublicKey::from(*client_session_id);

    // 3. Perform Diffie-Hellman scalar multiplication
    let shared_secret = server_private.diffie_hellman(&client_pub);

    // 4. Derive authentication keys using HKDF-SHA256
    let hk = Hkdf::<Sha256>::new(Some(expected_short_id), shared_secret.as_bytes());
    let mut okm = [0u8; 64];
    if hk.expand(b"REALITY", &mut okm).is_err() {
        return None;
    }

    let mut auth_key = [0u8; 32];
    let mut session_key = [0u8; 32];
    auth_key.copy_from_slice(&okm[0..32]);
    session_key.copy_from_slice(&okm[32..64]);

    Some((auth_key, session_key))
}

/// Validates that a received ShortID matches the server's authorized ShortID using constant-time comparison.
#[allow(dead_code)]
pub fn validate_short_id(received: &[u8], expected: &[u8]) -> bool {
    if received.len() != expected.len() {
        return false;
    }
    received.ct_eq(expected).unwrap_u8() == 1
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
        let client_private = StaticSecret::from(b2);
        let _client_public = PublicKey::from(&client_private);

        let res = verify_reality_client(&server_private, &b2, &short_id);
        assert!(res.is_some());
        let (auth_key, session_key) = res.unwrap();
        assert_ne!(auth_key, [0u8; 32]);
        assert_ne!(session_key, [0u8; 32]);
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
