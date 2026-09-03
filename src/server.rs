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


use std::path::PathBuf;

fn find_shoes_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SHOES_PATH") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for candidate in &["/usr/local/bin/shoes", "/app/shoes", "/tmp/shoes"] {
        let pb = PathBuf::from(candidate);
        if pb.exists() {
            return Some(pb);
        }
    }
    if let Ok(out) = std::process::Command::new("which").arg("shoes").output() {
        if out.status.success() {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(PathBuf::from(path_str));
            }
        }
    }
    None
}

pub fn generate_shoes_config(config: &ServerConfig) -> String {
    let priv_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        config.private_key.as_bytes(),
    );
    let short_id_hex = hex::encode(&config.short_id);

    format!(
r#"- address: "{}"
  protocol:
    type: tls
    reality_targets:
      "{}":
        private_key: "{}"
        short_ids: ["{}"]
        dest: "{}"
        vision: true
        protocol:
          type: vless
          user_id: "{}"
"#,
        config.listen_addr,
        config.server_name,
        priv_key_b64,
        short_id_hex,
        config.dest_target,
        config.user_uuid
    )
}

fn find_xray_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XRAY_PATH") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for candidate in &["/usr/local/bin/xray", "/app/xray", "/tmp/xray_bin/xray"] {
        let pb = PathBuf::from(candidate);
        if pb.exists() {
            return Some(pb);
        }
    }
    if let Ok(out) = std::process::Command::new("which").arg("xray").output() {
        if out.status.success() {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(PathBuf::from(path_str));
            }
        }
    }
    None
}

pub fn generate_xray_config(config: &ServerConfig) -> String {
    let priv_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        config.private_key.as_bytes(),
    );
    let short_id_hex = hex::encode(&config.short_id);

    format!(
r#"{{
  "log": {{
    "loglevel": "warning"
  }},
  "inbounds": [
    {{
      "port": {},
      "listen": "{}",
      "protocol": "vless",
      "settings": {{
        "clients": [
          {{
            "id": "{}",
            "flow": "xtls-rprx-vision"
          }}
        ],
        "decryption": "none"
      }},
      "streamSettings": {{
        "network": "tcp",
        "security": "reality",
        "realitySettings": {{
          "show": false,
          "dest": "{}",
          "xver": 0,
          "serverNames": [
            "{}"
          ],
          "privateKey": "{}",
          "shortIds": [
            "{}"
          ]
        }}
      }}
    }}
  ],
  "outbounds": [
    {{
      "protocol": "freedom",
      "tag": "direct"
    }}
  ]
}}"#,
        config.listen_addr.port(),
        config.listen_addr.ip(),
        config.user_uuid,
        config.dest_target,
        config.server_name,
        priv_key_b64,
        short_id_hex
    )
}

pub async fn run_server(config: Arc<ServerConfig>) -> Result<()> {
    // Print the ready-to-use Android one-click link and QR code on server startup
    let share_link = config.generate_vless_share_link();
    let qr_code = generate_ascii_qr(&share_link);
    let pub_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        config.public_key.as_bytes(),
    );
    let short_id_hex = hex::encode(&config.short_id);

    println!("\n===============================================================================");
    println!("  🚀 VLESS + RAW + REALITY (XTLS-VISION) SERVER IS ACTIVE!");
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

    // 1. Priority: Pure Rust shoes engine (Ultra-low ~12 MB RAM, VLESS + REALITY + Vision)
    if let Some(shoes_path) = find_shoes_binary() {
        tracing::info!("Starting pure Rust shoes REALITY engine (~12 MB RAM) from {:?}", shoes_path);
        let shoes_config = generate_shoes_config(&config);
        let config_path = std::env::temp_dir().join("shoes_config.yaml");
        tokio::fs::write(&config_path, shoes_config).await?;

        let mut child = tokio::process::Command::new(&shoes_path)
            .arg(&config_path)
            .spawn()?;

        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("Shoes engine exited with status: {}", status);
        }
        return Ok(());
    }

    // 2. Fallback: Xray engine if available
    if let Some(xray_path) = find_xray_binary() {
        tracing::info!("Starting Xray-core REALITY engine from {:?}", xray_path);
        let xray_config = generate_xray_config(&config);
        let config_path = std::env::temp_dir().join("vless_xray_config.json");
        tokio::fs::write(&config_path, xray_config).await?;

        let mut child = tokio::process::Command::new(&xray_path)
            .arg("run")
            .arg("-c")
            .arg(&config_path)
            .spawn()?;

        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("Xray engine exited with status: {}", status);
        }
        return Ok(());
    }

    // Fallback to internal server loop if xray binary is not present
    tracing::info!(
        "VLESS server listening on {} (Internal engine)",
        config.listen_addr
    );
    let listener = TcpListener::bind(config.listen_addr).await?;

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
