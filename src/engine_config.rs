use std::path::PathBuf;
use crate::config::ServerConfig;

const DEFAULT_SHOES_REALITY_YAML: &str = include_str!("../config/shoes_reality.yaml");
const DEFAULT_SHOES_SHADOWTLS_YAML: &str = include_str!("../config/shoes_shadowtls.yaml");

fn load_template(filename: &str, embedded_default: &'static str) -> String {
    // 1. Check custom directory from environment variable (e.g. CONFIG_DIR)
    if let Ok(dir) = std::env::var("CONFIG_DIR") {
        let p = PathBuf::from(dir).join(filename);
        if let Ok(content) = std::fs::read_to_string(&p) {
            return content;
        }
    }
    // 2. Check candidate standard directories on the filesystem
    for candidate_dir in &["config", "/app/config", "/etc/vless-rs", "."] {
        let p = PathBuf::from(candidate_dir).join(filename);
        if let Ok(content) = std::fs::read_to_string(&p) {
            return content;
        }
    }
    // 3. Fallback to compile-time embedded template
    embedded_default.to_string()
}

pub fn generate_shoes_config(config: &ServerConfig) -> String {
    let priv_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        config.private_key.as_bytes(),
    );
    let short_id_hex = hex::encode(&config.short_id);

    // Build anti-DPI outbound rules (FEATURES.md)
    let mut rules_yaml = String::new();
    let mut has_rules = false;

    // 1. Cloudflare Clean IP Mapping rule (FEATURES.md Tehdit N / Madde 25)
    // Remap blocked Anycast prefixes (104.21.0.0/16, 172.67.0.0/16) to clean reachable IP
    if let Some(ref clean_ip) = config.cf_clean_ip {
        rules_yaml.push_str(&format!(
r#"  rules:
    # Defeat TSPU/ISP Cloudflare Anycast IP prefix blocking (FEATURES.md #662)
    - masks: ["104.21.0.0/16", "172.67.0.0/16"]
      action: allow
      override_address: "{}"
"#,
            clean_ip
        ));
        has_rules = true;
    }

    // 2. Upstream Proxy Chaining (e.g. greentunnelRS on 127.0.0.1:8080)
    // Chains with greentunnelRS for SNI midpoint splitting, XDNS EDNS0, and TCP OOB desync
    if let Some(ref upstream) = config.upstream_proxy {
        if !has_rules {
            rules_yaml.push_str("  rules:\n");
        }
        rules_yaml.push_str(&format!(
r#"    # Route outbound traffic through upstream DPI evasion proxy (greentunnelRS)
    - masks: "0.0.0.0/0"
      action: allow
      client_chain:
        address: "{}"
        protocol:
          type: socks
"#,
            upstream
        ));
    }

    // Build global_limits (shoes v0.3.1+ process limits migration)
    let mut global_limits_yaml = String::new();
    if config.max_connections.is_some() || config.max_streams.is_some() || config.reload_grace_secs.is_some() {
        global_limits_yaml.push_str("- global_limits:\n");
        if let Some(mc) = config.max_connections {
            global_limits_yaml.push_str(&format!("    max_connections: {}\n", mc));
        }
        if let Some(ms) = config.max_streams {
            global_limits_yaml.push_str(&format!("    max_streams: {}\n", ms));
        }
        if let Some(rg) = config.reload_grace_secs {
            global_limits_yaml.push_str(&format!("    reload_grace_secs: {}\n", rg));
        }
    }

    let rendered = if config.protocol.eq_ignore_ascii_case("shadowtls") {
        let password = config.shadowtls_password.as_deref().unwrap_or("shadowtls-secret");
        let template = load_template("shoes_shadowtls.yaml", DEFAULT_SHOES_SHADOWTLS_YAML);
        template
            .replace("{{LISTEN_ADDR}}", &config.listen_addr.to_string())
            .replace("{{SERVER_NAME}}", &config.server_name)
            .replace("{{PASSWORD}}", password)
            .replace("{{DEST}}", &config.dest_target)
            .replace("{{UUID}}", &config.user_uuid.to_string())
            .replace("{{UDP_ENABLED}}", &config.udp_enabled.to_string())
            .replace("{{RULES}}", &rules_yaml)
    } else {
        let template = load_template("shoes_reality.yaml", DEFAULT_SHOES_REALITY_YAML);
        template
            .replace("{{LISTEN_ADDR}}", &config.listen_addr.to_string())
            .replace("{{SERVER_NAME}}", &config.server_name)
            .replace("{{PRIVATE_KEY}}", &priv_key_b64)
            .replace("{{SHORT_ID}}", &short_id_hex)
            .replace("{{DEST}}", &config.dest_target)
            .replace("{{UUID}}", &config.user_uuid.to_string())
            .replace("{{UDP_ENABLED}}", &config.udp_enabled.to_string())
            .replace("{{RULES}}", &rules_yaml)
    };

    if rendered.contains("{{GLOBAL_LIMITS}}") {
        rendered.replace("{{GLOBAL_LIMITS}}", &global_limits_yaml)
    } else if !global_limits_yaml.is_empty() {
        format!("{}{}", global_limits_yaml, rendered)
    } else {
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Opts;

    #[test]
    fn test_generate_shoes_config_reality_default() {
        let opts = Opts {
            port: 8080,
            bind: "0.0.0.0".to_string(),
            uuid: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            private_key: None,
            short_id: Some("0123456789abcdef".to_string()),
            dest: "gateway.icloud.com:443".to_string(),
            sni: "gateway.icloud.com".to_string(),
            server_address: Some("example.com".to_string()),
            server_port: Some(8080),
            protocol: "reality".to_string(),
            shadowtls_password: None,
            udp_enabled: true,
            upstream_proxy: None,
            cf_clean_ip: Some("104.16.132.229".to_string()),
            max_connections: None,
            max_streams: None,
            reload_grace_secs: None,
        };

        let cfg = ServerConfig::from_opts(opts).unwrap();
        let yaml = generate_shoes_config(&cfg);

        assert!(yaml.contains("reality_targets:"));
        assert!(yaml.contains("vision: true"));
        assert!(yaml.contains("udp_enabled: true"));
        assert!(yaml.contains("override_address: \"104.16.132.229\""));
        assert!(yaml.contains("104.21.0.0/16"));
        assert!(!yaml.contains("global_limits:"));
    }

    #[test]
    fn test_generate_shoes_config_with_global_limits() {
        let opts = Opts {
            port: 8080,
            bind: "0.0.0.0".to_string(),
            uuid: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            private_key: None,
            short_id: Some("0123456789abcdef".to_string()),
            dest: "gateway.icloud.com:443".to_string(),
            sni: "gateway.icloud.com".to_string(),
            server_address: Some("example.com".to_string()),
            server_port: Some(8080),
            protocol: "reality".to_string(),
            shadowtls_password: None,
            udp_enabled: true,
            upstream_proxy: None,
            cf_clean_ip: None,
            max_connections: Some(1024),
            max_streams: Some(2048),
            reload_grace_secs: Some(60),
        };

        let cfg = ServerConfig::from_opts(opts).unwrap();
        let yaml = generate_shoes_config(&cfg);

        assert!(yaml.contains("global_limits:"));
        assert!(yaml.contains("max_connections: 1024"));
        assert!(yaml.contains("max_streams: 2048"));
        assert!(yaml.contains("reload_grace_secs: 60"));
        assert!(yaml.contains("reality_targets:"));
    }

    #[test]
    fn test_generate_shoes_config_shadowtls_and_upstream() {
        let opts = Opts {
            port: 8443,
            bind: "0.0.0.0".to_string(),
            uuid: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            private_key: None,
            short_id: None,
            dest: "gateway.icloud.com:443".to_string(),
            sni: "gateway.icloud.com".to_string(),
            server_address: Some("example.com".to_string()),
            server_port: Some(8443),
            protocol: "shadowtls".to_string(),
            shadowtls_password: Some("my-secret-pass".to_string()),
            udp_enabled: true,
            upstream_proxy: Some("127.0.0.1:8080".to_string()),
            cf_clean_ip: None,
            max_connections: None,
            max_streams: None,
            reload_grace_secs: None,
        };

        let cfg = ServerConfig::from_opts(opts).unwrap();
        let yaml = generate_shoes_config(&cfg);

        assert!(yaml.contains("shadowtls_targets:"));
        assert!(yaml.contains("password: \"my-secret-pass\""));
        assert!(yaml.contains("udp_enabled: true"));
        assert!(yaml.contains("address: \"127.0.0.1:8080\""));
        assert!(yaml.contains("type: socks"));
    }
}
