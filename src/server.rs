use anyhow::Result;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

use crate::config::ServerConfig;
use crate::reality::fallback::handle_fallback;
use crate::reality::handshake::{inspect_tls_client_hello, HandshakeVerdict};
use crate::vless::protocol::{parse_vless_request, tunnel_vless_connection};

fn generate_ascii_qr(data: &str) -> Option<String> {
    qrcode::QrCode::new(data.as_bytes())
        .ok()
        .map(|code| code.render::<qrcode::render::unicode::Dense1x2>().build())
}


pub async fn run_server(config: Arc<ServerConfig>) -> Result<()> {
    let listener = TcpListener::bind(config.listen_addr).await?;
    tracing::info!(
        "VLESS+REALITY Server running on {} [Camouflage: {}]",
        config.listen_addr,
        config.dest_target
    );

    // Print the ready-to-use Android one-click link and QR code on server startup
    let share_link = config.generate_vless_share_link();
    let qr_code = generate_ascii_qr(&share_link);
    let pub_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        config.public_key.as_bytes(),
    );
    let short_id_hex = hex::encode(&config.short_id);

    println!("\n===============================================================================");
    println!("  🚀 VLESS + RAW + REALITY SERVER IS ACTIVE!");
    println!("===============================================================================");
    if config.is_railway {
        println!("  ☁️  ENVIRONMENT       : Railway Cloud Deployment");
        if config.has_tcp_proxy {
            println!("  🌐 TCP PROXY         : ✅ Active ({}:{})", config.public_address, config.public_port);
        } else {
            println!("  🌐 TCP PROXY         : ⚠️  Action Required!");
            println!("                         Go to Railway Dashboard -> Settings -> Networking -> Enable 'TCP Proxy'");
            println!("                         (Railway will assign your public domain and port)");
        }
    } else {
        println!("  🖥️  ENVIRONMENT       : Standalone / Local Server");
    }
    println!("-------------------------------------------------------------------------------");
    println!("  🔑 PARAMETERS:");
    println!("  UUID                 : {}", config.user_uuid);
    println!("  Public Host          : {}", config.public_address);
    println!("  Public Port          : {}", config.public_port);
    println!("  Camouflage Target    : {}", config.dest_target);
    println!("  SNI                  : {}", config.server_name);
    println!("  Public Key (pbk)     : {}", pub_key_b64);
    println!("  Short ID (sid)       : {}", short_id_hex);
    println!("-------------------------------------------------------------------------------");
    println!("  📲 ANDROID IMPORT LINK (Copy to Clipboard):");
    println!("  {}", share_link);
    println!("-------------------------------------------------------------------------------");
    if let Some(ref qr) = qr_code {
        println!("  📷 SCAN QR CODE WITH ANDROID CAMERA (v2rayNG / NekoBox):");
        println!("{}", qr);
        println!("-------------------------------------------------------------------------------");
    }
    println!("  📱 ANDROID SETUP GUIDE:");
    println!("  1. Open v2rayNG or NekoBox on your Android device");
    println!("  2. Tap '+' -> 'Scan QR code' (or 'Import config from clipboard')");
    println!("  3. Select the profile and tap the 'V' connect button!");
    println!("-------------------------------------------------------------------------------");
    println!("  💡 TIP (Railway Redeployment Persistence):");
    println!("  Set these variables in your Railway Project Settings -> Variables");
    println!("  to prevent keys from changing on future redeployments:");
    println!("  UUID={}", config.user_uuid);
    println!("  SHORT_ID={}", short_id_hex);
    println!("===============================================================================\n");


    loop {
        let (client, peer_addr) = match listener.accept().await {
            Ok(res) => res,
            Err(e) => {
                tracing::warn!("TCP accept error: {}", e);
                continue;
            }
        };

        let cfg = Arc::clone(&config);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, cfg).await {
                tracing::debug!("Connection ended from {}: {}", peer_addr, e);
            }
        });
    }
}

async fn handle_connection(mut client: TcpStream, config: Arc<ServerConfig>) -> Result<()> {
    // Enable TCP_NODELAY immediately
    client.set_nodelay(true).ok();

    // Read initial TLS ClientHello frame (peek or initial buffer)
    let mut initial_buf = [0u8; 1024];
    let n = client.read(&mut initial_buf).await?;
    if n == 0 {
        return Ok(());
    }

    let raw_initial = &initial_buf[..n];

    // Sniff the TLS ClientHello and determine whether it is an authorized REALITY client or scanner
    match inspect_tls_client_hello(raw_initial, &config) {
        HandshakeVerdict::Authenticated { .. } => {
            tracing::info!("Authenticated REALITY client handshake verified");
            // The client continues directly into the VLESS protocol layer
            let req = parse_vless_request(&mut client, &config.user_uuid).await?;
            tunnel_vless_connection(client, req).await?;
        }
        HandshakeVerdict::Fallback => {
            // Forward scanner or web probe transparently to legitimate destination (e.g. gateway.icloud.com)
            handle_fallback(client, raw_initial, &config.dest_target).await?;
        }
    }

    Ok(())
}
