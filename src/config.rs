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

    /// Transport security protocol: "reality" (default) or "shadowtls" (anti-SNI correlation)
    #[arg(long, env = "PROTOCOL", default_value = "reality")]
    pub protocol: String,

    /// Password for ShadowTLS v3 handshake (auto-generated if omitted)
    #[arg(long, env = "SHADOWTLS_PASSWORD")]
    pub shadowtls_password: Option<String>,

    /// Enable UDP multiplexing (XUDP / UDP-over-TCP) over the encrypted TLS stream
    #[arg(long, env = "ENABLE_UDP", default_value_t = true)]
    pub udp_enabled: bool,

    /// Upstream SOCKS5/HTTP proxy for outbound traffic (e.g. 127.0.0.1:8080 for greentunnelRS)
    #[arg(long, env = "UPSTREAM_PROXY")]
    pub upstream_proxy: Option<String>,

    /// Cloudflare Clean IP mapping to bypass TSPU/ISP Anycast 104.21.x / 172.67.x blocks
    #[arg(long, env = "CLOUDFLARE_CLEAN_IP", default_value = "104.16.132.229")]
    pub cf_clean_ip: Option<String>,

    /// Maximum concurrent connections for shoes engine (optional, unlimited if omitted)
    #[arg(long, env = "MAX_CONNECTIONS")]
    pub max_connections: Option<usize>,

    /// Maximum concurrent streams for shoes engine (optional, unlimited if omitted)
    #[arg(long, env = "MAX_STREAMS")]
    pub max_streams: Option<usize>,

    /// Reload grace period in seconds for TCP draining (default: 300)
    #[arg(long, env = "RELOAD_GRACE_SECS")]
    pub reload_grace_secs: Option<u64>,
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
    pub protocol: String,
    pub shadowtls_password: Option<String>,
    pub udp_enabled: bool,
    pub upstream_proxy: Option<String>,
    pub cf_clean_ip: Option<String>,
    pub max_connections: Option<usize>,
    pub max_streams: Option<usize>,
    pub reload_grace_secs: Option<u64>,
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

        let shadowtls_password = if opts.protocol.eq_ignore_ascii_case("shadowtls") {
            Some(opts.shadowtls_password.unwrap_or_else(|| {
                let mut rnd = [0u8; 16];
                for b in &mut rnd {
                    *b = fastrand::u8(..);
                }
                hex::encode(rnd)
            }))
        } else {
            None
        };

        let cf_clean_ip = opts
            .cf_clean_ip
            .filter(|s| !s.trim().is_empty() && s.trim() != "none" && s.trim() != "disable");
        let upstream_proxy = opts.upstream_proxy.filter(|s| !s.trim().is_empty());

        let max_connections = opts
            .max_connections
            .or_else(|| std::env::var("SHOES_MAX_CONNECTIONS").ok().and_then(|v| v.parse().ok()));
        let max_streams = opts
            .max_streams
            .or_else(|| std::env::var("SHOES_MAX_STREAMS").ok().and_then(|v| v.parse().ok()));
        let reload_grace_secs = opts
            .reload_grace_secs
            .or_else(|| std::env::var("SHOES_RELOAD_GRACE_SECS").ok().and_then(|v| v.parse().ok()));

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
            protocol: opts.protocol,
            shadowtls_password,
            udp_enabled: opts.udp_enabled,
            upstream_proxy,
            cf_clean_ip,
            max_connections,
            max_streams,
            reload_grace_secs,
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

    /// Formats the appropriate share link based on active protocol (REALITY or ShadowTLS v3)
    pub fn generate_share_link(&self) -> String {
        if self.protocol.eq_ignore_ascii_case("shadowtls") {
            let password = self.shadowtls_password.as_deref().unwrap_or("shadowtls-secret");
            let formatted_addr = if self.public_address.contains(':') && !self.public_address.starts_with('[') {
                format!("[{}]", self.public_address)
            } else {
                self.public_address.clone()
            };
            format!(
                "shadow-tls://{}@{}:{}?sni={}#vless-RS-ShadowTLS",
                password,
                formatted_addr,
                self.public_port,
                self.server_name
            )
        } else {
            self.generate_vless_share_link()
        }
    }
}

