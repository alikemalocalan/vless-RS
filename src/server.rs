use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::ServerConfig;
use crate::engine_config::generate_shoes_config;

fn generate_ascii_qr(data: &str) -> Option<String> {
    qrcode::QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::L)
        .ok()
        .map(|code| {
            code.render::<qrcode::render::unicode::Dense1x2>()
                .quiet_zone(false)
                .build()
        })
}

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

pub async fn run_server(config: Arc<ServerConfig>) -> Result<()> {
    // Print the ready-to-use Android one-click link and QR code on server startup
    let share_link = config.generate_share_link();
    let qr_code = generate_ascii_qr(&share_link);

    println!("\n===============================================================================");
    println!("  🚀 VLESS-RS ENHANCED ANTI-CENSORSHIP PROXY SERVER");
    println!("  🔐 PROTOCOL   : {}", config.protocol.to_uppercase());
    if config.is_railway && config.has_tcp_proxy {
        println!("  🌐 TCP PROXY  : {}:{}", config.public_address, config.public_port);
    } else {
        println!("  🌐 ADDRESS    : {}:{}", config.public_address, config.public_port);
    }
    println!("  ⚡ XUDP / UoT : {}", if config.udp_enabled { "✅ Active (UDP multiplexed over TCP)" } else { "❌ Disabled" });
    if let Some(ref cf) = config.cf_clean_ip {
        println!("  🛡️  CF CLEAN   : ✅ Active (104.21.x/172.67.x -> {})", cf);
    }
    if let Some(ref up) = config.upstream_proxy {
        println!("  🔗 UPSTREAM   : ✅ Chained via greentunnelRS ({})", up);
    }
    if let Some(mc) = config.max_connections {
        println!("  🚦 MAX CONNS  : {} connections", mc);
    }
    if let Some(ms) = config.max_streams {
        println!("  📊 MAX STREAMS: {} streams", ms);
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
        let shoes_version = match tokio::process::Command::new(&shoes_path).arg("--version").output().await {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if v.is_empty() {
                    "shoes".to_string()
                } else {
                    v
                }
            }
            _ => "shoes".to_string(),
        };
        tracing::info!("Starting {} engine from {:?}", shoes_version, shoes_path);

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

    // REALITY encryption is strictly required to bypass censorship
    anyhow::bail!(
        "No pure Rust shoes engine binary found. REALITY TLS 1.3 encryption is strictly required to bypass censorship. Ensure shoes is installed at /usr/local/bin/shoes or specify SHOES_PATH."
    );
}
