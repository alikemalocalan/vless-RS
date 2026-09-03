use anyhow::Result;
use std::sync::Arc;

use crate::config::ServerConfig;

fn generate_ascii_qr(data: &str) -> Option<String> {
    qrcode::QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::L)
        .ok()
        .map(|code| {
            code.render::<qrcode::render::unicode::Dense1x2>()
                .quiet_zone(false)
                .build()
        })
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

    println!("\n===============================================================================");
    println!("  🚀 VLESS + REALITY (XTLS-VISION) SERVER ACTIVE");
    if config.is_railway && config.has_tcp_proxy {
        println!("  🌐 TCP PROXY : {}:{}", config.public_address, config.public_port);
    } else {
        println!("  🌐 ADDRESS   : {}:{}", config.public_address, config.public_port);
    }
    println!("-------------------------------------------------------------------------------");
    println!("  📲 ANDROID LINK:");
    println!("  {}", share_link);
    if let Some(ref qr) = qr_code {
        println!("-------------------------------------------------------------------------------");
        println!("  📷 QR CODE:");
        println!("{}", qr);
    }
    println!("===============================================================================\n");

    // 1. Priority: Pure Rust shoes engine (Ultra-low ~12 MB RAM, VLESS + REALITY + Vision)
    if let Some(shoes_path) = find_shoes_binary() {
        let shoes_config = generate_shoes_config(&config);
        let config_path = std::env::temp_dir().join("shoes_config.yaml");
        tokio::fs::write(&config_path, shoes_config).await?;

        let mut child = tokio::process::Command::new(&shoes_path)
            .arg("--no-reload")
            .arg(&config_path)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // Filter out harmless port probes, scanner errors, and invalid TLS handshake attempts
                    if line.contains("Invalid TLS protocol version")
                        || line.contains("Session ID decrypt failed")
                        || line.contains("failed to setup server stream")
                    {
                        continue;
                    }
                    eprintln!("{}", line);
                }
            });
        }

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

    // REALITY encryption is strictly required to bypass censorship
    anyhow::bail!(
        "No REALITY TLS engine (shoes or xray) found. REALITY TLS 1.3 encryption is strictly required to bypass censorship."
    );
}
