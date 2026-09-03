use crate::config::ServerConfig;
use crate::reality::crypto::verify_reality_client;

/// Result of sniffing the incoming client connection.
#[allow(dead_code)]
pub enum HandshakeVerdict {
    /// Authenticated REALITY client carrying our key and short ID
    Authenticated {
        client_pub: [u8; 32],
        auth_key: [u8; 32],
        session_key: [u8; 32],
    },
    /// Unauthenticated probe, scanner, or regular HTTPS traffic to be forwarded to fallback target
    Fallback,
}

/// Parses a TLS ClientHello binary buffer to extract the 32-byte SessionID and SNI.
pub fn inspect_tls_client_hello(
    buf: &[u8],
    config: &ServerConfig,
) -> HandshakeVerdict {
    // Basic TLS Record header check: ContentType == 0x16 (Handshake), Version >= 0x0301
    if buf.len() < 43 || buf[0] != 0x16 || buf[5] != 0x01 {
        return HandshakeVerdict::Fallback;
    }

    // Offset of SessionID in standard TLS 1.3 / 1.2 ClientHello:
    // Header (5) + HandshakeType (1) + Length (3) + Version (2) + Random (32) = 43
    let session_id_len_offset = 43;
    if session_id_len_offset >= buf.len() {
        return HandshakeVerdict::Fallback;
    }

    let session_id_len = buf[session_id_len_offset] as usize;
    if session_id_len != 32 {
        // REALITY clients always supply a 32-byte SessionID (containing the client ephemeral public key)
        return HandshakeVerdict::Fallback;
    }

    let session_id_start = session_id_len_offset + 1;
    if session_id_start + 32 > buf.len() {
        return HandshakeVerdict::Fallback;
    }

    let mut client_session_id = [0u8; 32];
    client_session_id.copy_from_slice(&buf[session_id_start..session_id_start + 32]);

    // Verify client authenticity using X25519 scalar multiplication and HKDF
    if let Some((auth_key, session_key)) =
        verify_reality_client(&config.private_key, &client_session_id, &config.short_id)
    {
        HandshakeVerdict::Authenticated {
            client_pub: client_session_id,
            auth_key,
            session_key,
        }
    } else {
        HandshakeVerdict::Fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x25519_dalek::{PublicKey, StaticSecret};

    #[test]
    fn test_inspect_empty_buffer_returns_fallback() {
        let dummy_priv = StaticSecret::from([0x42u8; 32]);
        let dummy_pub = PublicKey::from(&dummy_priv);
        let dummy_cfg = ServerConfig {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            user_uuid: uuid::Uuid::new_v4(),
            private_key: dummy_priv,
            public_key: dummy_pub,
            short_id: vec![1, 2, 3, 4],
            dest_target: "gateway.icloud.com:443".to_string(),
            server_name: "gateway.icloud.com".to_string(),
            public_address: "127.0.0.1".to_string(),
            public_port: 8080,
            is_railway: false,
            has_tcp_proxy: false,
        };

        assert!(matches!(
            inspect_tls_client_hello(&[], &dummy_cfg),
            HandshakeVerdict::Fallback
        ));
    }
}
