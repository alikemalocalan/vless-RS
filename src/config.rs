use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use x25519_dalek::{PublicKey, StaticSecret};

/// Default legitimate camouflage destination (Apple's worldwide push/iCloud edge)
pub const DEFAULT_DEST: &str = "gateway.icloud.com:443";
/// Default legitimate Server Name Indication (SNI)
pub const DEFAULT_SNI: &str = "gateway.icloud.com";

#[derive(Parser, Debug, Clone)]
#[command(name = "vless-RS")]
#[command(about = "High-performance standalone VLESS + RAW + REALITY server in Rust for TSPU/DPI circumvention")]
pub struct Opts {
    #[arg(short, long, env = "PORT", default_value = "8080")]
    pub port: u16,

    #[arg(short, long, env = "BIND", default_value = "0.0.0.0")]
    pub bind: String,

    #[arg(short, long, env = "UUID")]
    pub uuid: Option<String>,

    #[arg(long, env = "PRIVATE_KEY")]
    pub private_key: Option<String>,

    #[arg(long, env = "SHORT_ID")]
    pub short_id: Option<String>,

    #[arg(short, long, env = "DEST", default_value = DEFAULT_DEST)]
    pub dest: String,

    #[arg(long, env = "SNI", default_value = DEFAULT_SNI)]
    pub sni: String,

    /// External server address / IP (e.g. Railway TCP domain: roundhouse.proxy.rlwy.net)
    #[arg(long, env = "SERVER_ADDRESS")]
    pub server_address: Option<String>,

    /// External public port if different from internal listen port (e.g. Railway TCP port)
    #[arg(long, env = "SERVER_PORT")]
    pub server_port: Option<u16>,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub listen_addr: SocketAddr,
    pub user_uuid: uuid::Uuid,
    pub private_key: StaticSecret,
    pub public_key: PublicKey,
    pub short_id: Vec<u8>,
    pub dest_target: String,
    pub server_name: String,
    pub public_address: String,
    pub public_port: u16,
    pub is_railway: bool,
    pub has_tcp_proxy: bool,
}

impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerConfig")
            .field("listen_addr", &self.listen_addr)
            .field("user_uuid", &self.user_uuid)
            .field("public_key", &self.public_key)
            .field("short_id", &hex::encode(&self.short_id))
            .field("dest_target", &self.dest_target)
            .field("server_name", &self.server_name)
            .field("public_address", &self.public_address)
            .field("public_port", &self.public_port)
            .field("is_railway", &self.is_railway)
            .field("has_tcp_proxy", &self.has_tcp_proxy)
            .finish_non_exhaustive()
    }
}

impl ServerConfig {
    pub fn from_opts(opts: Opts) -> Result<Self> {
        let bind_str = opts.bind.trim();
        let clean_bind = bind_str.trim_matches(|c| c == '[' || c == ']');
        let listen_addr: SocketAddr = if let Ok(ip) = clean_bind.parse::<std::net::IpAddr>() {
            SocketAddr::new(ip, opts.port)
        } else {
            format!("{}:{}", bind_str, opts.port).parse()?
        };

        // 1. UUID handling
        let user_uuid = match opts.uuid {
            Some(ref s) if !s.trim().is_empty() => uuid::Uuid::parse_str(s.trim())?,
            _ => uuid::Uuid::new_v4(),
        };

        // 2. REALITY Keypair handling
        let (private_key, public_key) = match opts.private_key {
            Some(ref s) if !s.trim().is_empty() => {
                let trimmed = s.trim();
                let bytes = if let Ok(h) = hex::decode(trimmed) {
                    h
                } else {
                    base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, trimmed)
                        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, trimmed))?
                };
                if bytes.len() != 32 {
                    anyhow::bail!("Private key must be 32 bytes (got {})", bytes.len());
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                let priv_key = StaticSecret::from(arr);
                let pub_key = PublicKey::from(&priv_key);
                (priv_key, pub_key)
            }
            _ => {
                let mut bytes = [0u8; 32];
                for b in &mut bytes {
                    *b = fastrand::u8(..);
                }
                let priv_key = StaticSecret::from(bytes);
                let pub_key = PublicKey::from(&priv_key);
                (priv_key, pub_key)
            }
        };

        // 3. ShortId handling (up to 8 hex bytes)
        let short_id = match opts.short_id {
            Some(ref s) if !s.trim().is_empty() => hex::decode(s.trim())?,
            _ => {
                let mut rnd = [0u8; 8];
                for b in &mut rnd {
                    *b = fastrand::u8(..);
                }
                rnd.to_vec()
            }
        };

        let is_railway = std::env::var("RAILWAY_ENVIRONMENT").is_ok()
            || std::env::var("RAILWAY_PROJECT_ID").is_ok()
            || std::env::var("RAILWAY_SERVICE_ID").is_ok()
            || std::env::var("RAILWAY_TCP_PROXY_DOMAIN").is_ok()
            || std::env::var("RAILWAY_PUBLIC_DOMAIN").is_ok();

        let tcp_proxy_domain = std::env::var("RAILWAY_TCP_PROXY_DOMAIN")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let tcp_proxy_port = std::env::var("RAILWAY_TCP_PROXY_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok());

        let has_tcp_proxy = tcp_proxy_domain.is_some() && tcp_proxy_port.is_some();

        let public_address = opts
            .server_address
            .filter(|s| !s.trim().is_empty())
            .or(tcp_proxy_domain)
            .unwrap_or_else(|| "roundhouse.proxy.rlwy.net".to_string());

        let public_port = opts
            .server_port
            .or(tcp_proxy_port)
            .unwrap_or(opts.port);

        Ok(Self {
            listen_addr,
            user_uuid,
            private_key,
            public_key,
            short_id,
            dest_target: opts.dest,
            server_name: opts.sni,
            public_address,
            public_port,
            is_railway,
            has_tcp_proxy,
        })
    }

    /// Formats an RFC-compatible VLESS+REALITY link ready to import into Android Xray / v2rayNG / NekoBox.
    pub fn generate_vless_share_link(&self) -> String {
        let pub_key_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            self.public_key.as_bytes(),
        );
        let short_id_hex = hex::encode(&self.short_id);

        let formatted_addr = if self.public_address.contains(':') && !self.public_address.starts_with('[') {
            format!("[{}]", self.public_address)
        } else {
            self.public_address.clone()
        };

        format!(
            "vless://{}@{}:{}?security=reality&sni={}&fp=chrome&pbk={}&sid={}&type=tcp&flow=xtls-rprx-vision#vless-RS",
            self.user_uuid,
            formatted_addr,
            self.public_port,
            self.server_name,
            pub_key_b64,
            short_id_hex
        )
    }
}

