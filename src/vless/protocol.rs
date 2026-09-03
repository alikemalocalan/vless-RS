use anyhow::{bail, Context, Result};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

pub const VLESS_VERSION: u8 = 0;
#[allow(dead_code)]
pub const CMD_TCP: u8 = 1;
#[allow(dead_code)]
pub const CMD_UDP: u8 = 2;
#[allow(dead_code)]
pub const CMD_MUX: u8 = 3;

pub const ATYP_IPV4: u8 = 1;
pub const ATYP_DOMAIN: u8 = 2;
pub const ATYP_IPV6: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlessRequest {
    pub uuid: Uuid,
    pub command: u8,
    pub target_addr: String,
    pub target_port: u16,
}

/// Parses the VLESS protocol request header from an authenticated stream.
pub async fn parse_vless_request<S>(stream: &mut S, expected_uuid: &Uuid) -> Result<VlessRequest>
where
    S: AsyncRead + Unpin,
{
    // 1. Version byte
    let version = stream.read_u8().await.context("Failed to read VLESS version")?;
    if version != VLESS_VERSION {
        bail!("Unsupported VLESS version: {}", version);
    }

    // 2. 16-byte UUID
    let mut uuid_bytes = [0u8; 16];
    stream
        .read_exact(&mut uuid_bytes)
        .await
        .context("Failed to read VLESS UUID")?;
    let client_uuid = Uuid::from_bytes(uuid_bytes);
    if client_uuid != *expected_uuid {
        bail!("Unauthorized VLESS client UUID: {}", client_uuid);
    }

    // 3. Addons length & bytes
    let addons_len = stream
        .read_u8()
        .await
        .context("Failed to read VLESS addons length")? as usize;
    if addons_len > 0 {
        let mut addons_buf = vec![0u8; addons_len];
        stream
            .read_exact(&mut addons_buf)
            .await
            .context("Failed to read VLESS addons data")?;
    }

    // 4. Command byte
    let command = stream.read_u8().await.context("Failed to read VLESS command")?;

    // 5. Target Port (2 bytes Big Endian)
    let target_port = stream
        .read_u16()
        .await
        .context("Failed to read VLESS target port")?;

    // 6. Address Type & Address
    let addr_type = stream
        .read_u8()
        .await
        .context("Failed to read VLESS address type")?;
    let target_addr = match addr_type {
        ATYP_IPV4 => {
            let mut ip_bytes = [0u8; 4];
            stream
                .read_exact(&mut ip_bytes)
                .await
                .context("Failed to read IPv4 address")?;
            Ipv4Addr::from(ip_bytes).to_string()
        }
        ATYP_DOMAIN => {
            let domain_len = stream
                .read_u8()
                .await
                .context("Failed to read domain length")? as usize;
            let mut domain_buf = vec![0u8; domain_len];
            stream
                .read_exact(&mut domain_buf)
                .await
                .context("Failed to read domain data")?;
            String::from_utf8(domain_buf).context("Invalid UTF-8 in target domain name")?
        }
        ATYP_IPV6 => {
            let mut ip_bytes = [0u8; 16];
            stream
                .read_exact(&mut ip_bytes)
                .await
                .context("Failed to read IPv6 address")?;
            Ipv6Addr::from(ip_bytes).to_string()
        }
        other => bail!("Unsupported VLESS address type: {}", other),
    };

    Ok(VlessRequest {
        uuid: client_uuid,
        command,
        target_addr,
        target_port,
    })
}

/// Writes the standard 2-byte VLESS response header (version 0, addons length 0).
pub async fn send_vless_response<S>(stream: &mut S) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    stream.write_all(&[VLESS_VERSION, 0x00]).await?;
    stream.flush().await?;
    Ok(())
}

/// Connects to the destination endpoint and bridges the client and remote sockets.
pub async fn tunnel_vless_connection(
    mut client: TcpStream,
    req: VlessRequest,
) -> Result<()> {
    let dest_display = if req.target_addr.contains(':') && !req.target_addr.starts_with('[') {
        format!("[{}]:{}", req.target_addr, req.target_port)
    } else {
        format!("{}:{}", req.target_addr, req.target_port)
    };

    tracing::info!(
        "VLESS tunnel established: [UUID: {}] -> {}",
        req.uuid,
        dest_display
    );

    // Send successful response header to the client
    send_vless_response(&mut client)
        .await
        .context("Failed to send VLESS response header")?;

    // Connect to outbound destination target (supports Domain, IPv4, and IPv6)
    let mut remote = TcpStream::connect((req.target_addr.as_str(), req.target_port))
        .await
        .with_context(|| format!("Failed to connect to VLESS target {}", dest_display))?;

    // Enable TCP_NODELAY for minimum latency
    remote.set_nodelay(true).ok();
    client.set_nodelay(true).ok();

    // Bidirectional zero-copy relay
    let _ = tokio::io::copy_bidirectional(&mut client, &mut remote).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_parse_vless_ipv4_request() {
        let expected_uuid = Uuid::new_v4();
        let mut data = Vec::new();
        data.push(0x00); // Version
        data.extend_from_slice(expected_uuid.as_bytes()); // UUID
        data.push(0x00); // Addons len
        data.push(CMD_TCP); // Command
        data.extend_from_slice(&443u16.to_be_bytes()); // Port
        data.push(ATYP_IPV4); // Address type
        data.extend_from_slice(&[1, 1, 1, 1]); // IP

        let mut cursor = Cursor::new(data);
        let req = parse_vless_request(&mut cursor, &expected_uuid)
            .await
            .unwrap();

        assert_eq!(req.uuid, expected_uuid);
        assert_eq!(req.command, CMD_TCP);
        assert_eq!(req.target_port, 443);
        assert_eq!(req.target_addr, "1.1.1.1");
    }

    #[tokio::test]
    async fn test_parse_vless_domain_request() {
        let expected_uuid = Uuid::new_v4();
        let mut data = Vec::new();
        data.push(0x00); // Version
        data.extend_from_slice(expected_uuid.as_bytes()); // UUID
        data.push(0x00); // Addons len
        data.push(CMD_TCP); // Command
        data.extend_from_slice(&443u16.to_be_bytes()); // Port
        data.push(ATYP_DOMAIN); // Domain type
        let domain = b"www.instagram.com";
        data.push(domain.len() as u8);
        data.extend_from_slice(domain);

        let mut cursor = Cursor::new(data);
        let req = parse_vless_request(&mut cursor, &expected_uuid)
            .await
            .unwrap();

        assert_eq!(req.uuid, expected_uuid);
        assert_eq!(req.target_port, 443);
        assert_eq!(req.target_addr, "www.instagram.com");
    }

    #[tokio::test]
    async fn test_parse_vless_ipv6_request() {
        let expected_uuid = Uuid::new_v4();
        let mut data = Vec::new();
        data.push(0x00); // Version
        data.extend_from_slice(expected_uuid.as_bytes()); // UUID
        data.push(0x00); // Addons len
        data.push(CMD_TCP); // Command
        data.extend_from_slice(&80u16.to_be_bytes()); // Port
        data.push(ATYP_IPV6); // Address type
        let ipv6_bytes = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        data.extend_from_slice(&ipv6_bytes);

        let mut cursor = Cursor::new(data);
        let req = parse_vless_request(&mut cursor, &expected_uuid)
            .await
            .unwrap();

        assert_eq!(req.uuid, expected_uuid);
        assert_eq!(req.target_port, 80);
        assert_eq!(req.target_addr, "2001:db8::1");
    }
}
