# 🛡️ vless-RS — High-Performance VLESS + REALITY Proxy in Rust

> **Ultra-low footprint (~12 MB RAM), zero-dependency, standalone VLESS + REALITY (XTLS-Vision) server built for DPI and censorship circumvention.**  
> *Designed to bypass sophisticated national censorship systems (such as Russia TSPU, Iran, China GFW) using legitimate Apple/Google TLS 1.3 camouflage.*

---

## 🎯 Motivation & Background

Sophisticated censorship apparatuses (like Russia's national TSPU/Rostelecom infrastructure, Iran, and China GFW) deploy deep packet inspection (DPI) boxes that actively fingerprint, throttle, or block traditional VPN protocols and proxy tunnels.

### 💡 The Solution: VLESS + REALITY (XTLS-Vision)
Instead of trying to obfuscate encrypted data within identifiable tunnels, **camouflage completely as legitimate HTTPS traffic to top-tier CDN/tech services**:
- To any external DPI box or port scanner, the server appears **identical to Apple's `gateway.icloud.com:443`**.
- Unauthorized scanners receive Apple's genuine TLS certificate and HTTP responses via transparent fallback. The censorship box cannot detect the presence of a proxy.
- Only authorized clients presenting the correct X25519 public key and short ID can complete the handshake and establish the VLESS tunnel.

---

## ⚡ Key Features

- 🚀 **100% Rust Architecture:** Powered by async Tokio and lightweight Rust proxy engines. No heavy Go runtimes or complex Python/Node setups.
- 🪶 **Ultra-Low Memory Footprint (~12 MB RAM):** Uses up to 6x less memory than Go-based alternatives (which idle at 75–80 MB and spike to 400+ MB during speed tests).
- ⚡ **XTLS-Vision Direct Acceleration:** Bypasses double-encryption for TLS-in-TLS connections, delivering wire-speed throughput and minimal battery drain on mobile devices.
- 🛡️ **Active Probe Immunity:** Unauthenticated probes, bots, or censors are transparently redirected to the target domain (`gateway.icloud.com:443`).
- ☁️ **Cloud & Container Ready:** Fully compatible with Docker, Linux VPS, and cloud environments supporting TCP ports.
- 📲 **Instant Android Setup:** Automatically prints an importable `vless://` link and a compact, scannable ASCII QR code directly to deployment logs.

---

## 🏗️ Architecture & Protocol Flow

```text
[ Android Client (v2rayNG / NekoBox) ]
                 │
                 │  TLS 1.3 ClientHello
                 │  - SNI: gateway.icloud.com
                 │  - SessionID: Client Ephemeral Key + Encrypted ShortID
                 ▼
      [ vless-RS (Port 8080 / TCP Proxy) ]
                 │
                 ├──► [1. REALITY TLS 1.3 Handshake Inspection]
                 │
                 ├──► IF UNVERIFIED / DPI SCANNER / BOT:
                 │    └──► Transparent Fallback -> gateway.icloud.com:443
                 │         (The probe receives the genuine Apple certificate & response.
                 │          The middlebox cannot detect that a proxy exists.)
                 │
                 └──► IF AUTHENTICATED REALITY CLIENT:
                      ├── X25519 ECDH Key Exchange & AEAD Session Decryption
                      ├── Short ID Validation Verified!
                      ├── VLESS Header Parsing (UUID, Destination Target)
                      └──► Zero-Copy Async Outbound Tunneling (Instagram, YouTube, etc.)
```

---

## 📱 Client Setup (Android / iOS / Desktop)

### Android (v2rayNG / NekoBox)
1. Copy the `vless://...` URL or scan the QR code displayed in the server startup logs.
2. Open **v2rayNG** (available on Google Play or GitHub Releases).
3. Tap **+** -> **Scan QR Code** (or **Import config from clipboard**).
4. Select the created configuration and tap the **V** connect button.
5. All blocked services (Instagram, YouTube, X, Google) are now accessible.

---

## ⚙️ Environment Variables

Configure server runtime parameters via environment variables or CLI flags:

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | Local listen port |
| `BIND` | `0.0.0.0` | Network interface to bind (`0.0.0.0` for IPv4, `::` for dual-stack) |
| `UUID` | *Auto-generated* | VLESS user authentication UUID |
| `PRIVATE_KEY` | *Auto-generated* | REALITY X25519 32-byte private key (base64 URL-safe or hex) |
| `SHORT_ID` | *Auto-generated* | REALITY Short ID for client verification (hex) |
| `DEST` | `gateway.icloud.com:443` | Legitimate camouflage destination for fallback |
| `SNI` | `gateway.icloud.com` | Target server name indicated in the TLS ClientHello |
| `SERVER_ADDRESS`| *Auto-detected* | Public host/domain written to the generated link |
| `SERVER_PORT` | `$PORT` | Public port written to the generated link |

> [!TIP]
> **Persistent Configuration:** To keep your connection link and keys identical across future server restarts, set the `UUID`, `SHORT_ID`, and `PRIVATE_KEY` environment variables.

---

## 💻 Local Building & Testing

```bash
# Compile optimized release binary
cargo build --release

# Run with default parameters
./target/release/vless-RS

# Run with custom camouflage target and port
./target/release/vless-RS --port 8080 --dest dl.google.com:443 --sni dl.google.com

# Run test suite
cargo test
```

---

## 📄 License

MIT License. Designed for open communication, digital privacy, and anti-censorship research.
