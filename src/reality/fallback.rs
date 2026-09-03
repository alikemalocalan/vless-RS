use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Transparently relays an unauthenticated connection (active scanner, DPI probe, or web crawler)
/// to the legitimate camouflage destination (e.g. `gateway.icloud.com:443`).
pub async fn handle_fallback(
    mut client: TcpStream,
    initial_bytes: &[u8],
    dest_target: &str,
) -> Result<()> {
    tracing::warn!(
        "DPI active probe or unauthenticated TLS ClientHello detected — forwarding transparently to camouflage target: {}",
        dest_target
    );

    // Connect to the authentic camouflage target
    let mut upstream = TcpStream::connect(dest_target)
        .await
        .with_context(|| format!("Failed to connect to fallback target {}", dest_target))?;

    // Forward the already-read ClientHello bytes so the upstream server sees the full initial packet
    if !initial_bytes.is_empty() {
        upstream.write_all(initial_bytes).await?;
        upstream.flush().await?;
    }

    // Bidirectional raw stream copy between scanner and legitimate server
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;

    Ok(())
}
