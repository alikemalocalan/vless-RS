# 🛡️ vless-RS — High-Performance VLESS + REALITY Proxy in Rust

> **Ultra-low footprint (~12 MB RAM), pure Rust architecture, zero-dependency VLESS + REALITY (XTLS-Vision) server built for DPI and censorship circumvention.**  
> *Designed to bypass sophisticated national censorship systems (such as Russia TSPU, Iran, China GFW) using legitimate Apple/Google TLS 1.3 camouflage.*

---

## 🎯 Motivation & Background

Advanced national censorship systems (like Russia's TSPU/Rostelecom infrastructure, Iran's national firewall, and the GFW) deploy deep packet inspection (DPI) boxes that actively fingerprint, throttle, or block traditional VPN protocols and plain proxy tunnels.

### 💡 The Solution: VLESS + REALITY (XTLS-Vision)
Instead of attempting to obfuscate encrypted traffic within identifiable tunnels, **camouflage completely as legitimate HTTPS traffic to top-tier CDN/tech services**:
- To any external DPI box or port scanner, the server appears **identical to Apple's `gateway.icloud.com:443`**.
- Unauthorized scanners receive Apple's genuine TLS certificate and HTTP responses via transparent fallback. The censorship middlebox cannot detect that a proxy exists.
- Only authorized clients presenting the correct X25519 public key and short ID can complete the handshake and establish the VLESS tunnel.
- **Strict REALITY Enforcement:** Unencrypted VLESS is deliberately disabled; full TLS 1.3 encryption is mandatory to prevent instant packet inspection and DPI blocking.

---

## ⚡ Key Features

- 🚀 **100% Rust Architecture:** Powered by Tokio and high-performance Rust proxy engines. No heavy Go runtimes or complex Python/Node setups.
- 🪶 **Ultra-Low Memory Footprint (~12 MB RAM):** Uses up to 6x less memory than Go-based alternatives (which idle at 75–80 MB and spike to 400+ MB during speed tests).
- ⚡ **XTLS-Vision Direct Acceleration:** Bypasses double-encryption for TLS-in-TLS connections, delivering wire-speed throughput, reduced latency, and minimal mobile battery drain.
- 🛡️ **Active Probe Immunity:** Unauthenticated probes, bots, or censors are transparently redirected to the target domain (`gateway.icloud.com:443`).
- ☁️ **Cloud & Container Ready:** Fully compatible with Docker, Linux VPS, Kubernetes, and cloud platforms supporting TCP networking.
- 📲 **Instant Client Setup:** Automatically prints an importable `vless://` link and a compact, scannable ASCII QR code directly to the startup console logs.

---

## 🏗️ Architecture & Protocol Flow

```text
[ Client (v2rayNG / NekoBox / FoXray) ]
                 │
                 │  TLS 1.3 ClientHello (uTLS Chrome Fingerprint)
                 │  - SNI: gateway.icloud.com
                 │  - SessionID: Client Ephemeral Key + Encrypted ShortID
                 ▼
          [ vless-RS Server ]
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
                      └──► Zero-Copy Async Outbound Tunneling (Instagram, YouTube, X)
```

---

## 🚀 Quick Start with Docker

### 1. Build and Run

```bash
# Clone repository
git clone https://github.com/alikemalocalan/vless-RS.git
cd vless-RS

# Build Docker image
docker build -t vless-rs .

# Run container
docker run -d --name vless-rs -p 8080:8080 --restart always vless-rs
```

### 2. View Connection Link and QR Code

```bash
docker logs vless-rs
```

The server displays a clean, compact startup screen with your ready-to-import configuration:

```text
===============================================================================
  🚀 VLESS + REALITY (XTLS-VISION) SERVER ACTIVE
  🌐 ADDRESS   : your-server-ip:8080
-------------------------------------------------------------------------------
  📲 ANDROID LINK:
  vless://uuid@your-server-ip:8080?security=reality&sni=gateway.icloud.com&fp=chrome&pbk=...&sid=...&type=tcp&flow=xtls-rprx-vision#vless-RS
-------------------------------------------------------------------------------
  📷 QR CODE:
  [Compact Scannable ASCII QR Code]
===============================================================================
```

---

## 📱 Supported Clients

| Platform | Recommended Client | Import Method |
|---|---|---|
| **Android** | [v2rayNG](https://github.com/2dust/v2rayNG) / [NekoBox](https://github.com/MatsuriDayo/NekoBoxForAndroid) | Scan QR code or import `vless://` from clipboard |
| **iOS** | [FoXray](https://apps.apple.com/app/foxray/id6448898396) / [Shadowrocket](https://apps.apple.com/app/shadowrocket/id932747118) | Scan QR code or import URL |
| **Windows** | [v2rayN](https://github.com/2dust/v2rayN) / [Nekoray](https://github.com/MatsuriDayo/nekoray) | Paste `vless://` URL |
| **macOS / Linux** | [sing-box](https://github.com/SagerNet/sing-box) / [Clash Verge Rev](https://github.com/clash-verge-rev/clash-verge-rev) | Import configuration URL |

---

## ⚙️ Configuration & Environment Variables

Configure server runtime parameters via environment variables or CLI flags:

| Variable | CLI Flag | Default | Description |
|---|---|---|---|
| `PORT` | `--port` | `8080` | Local TCP listen port |
| `BIND` | `--bind` | `0.0.0.0` | Network interface to bind (`0.0.0.0` for IPv4, `::` for dual-stack) |
| `PROTOCOL` | `--protocol` | `reality` | Proxy protocol (`reality` or `shadowtls`) |
| `ENABLE_UDP` | `--enable-udp` | `true` | SagerNet UDP-over-TCP (XUDP) multiplexing to bypass UDP 443/53 blocking |
| `CLOUDFLARE_CLEAN_IP`| `--cloudflare-clean-ip` | *(optional)* | Clean Cloudflare IP (e.g. `104.16.132.229`) to bypass TSPU Anycast blocks |
| `UPSTREAM_PROXY` | `--upstream-proxy` | *(optional)* | Upstream proxy (e.g. `127.0.0.1:8080` or `socks5://...`) to chain with `greentunnelRS` |
| `SHADOWTLS_PASSWORD` | `--shadowtls-password` | *Auto-generated* | Shared secret password when running in `shadowtls` protocol mode |
| `UUID` | `--uuid` | *Auto-generated* | VLESS user authentication UUID |
| `PRIVATE_KEY` | `--private-key` | *Auto-generated* | REALITY X25519 32-byte private key (base64 URL-safe or hex) |
| `SHORT_ID` | `--short-id` | *Auto-generated* | REALITY Short ID for client verification (hex) |
| `DEST` | `--dest` | `gateway.icloud.com:443` | Legitimate camouflage destination for fallback |
| `SNI` | `--sni` | `gateway.icloud.com` | Target server name indicated in the TLS ClientHello |
| `SERVER_ADDRESS`| `--server-address`| *Auto-detected* | Public host/domain written to the generated link |
| `SERVER_PORT` | `--server-port` | `$PORT` | Public port written to the generated link |
| `MAX_CONNECTIONS`| `--max-connections`| *(unlimited)* | Process-wide connection limit for shoes engine |
| `MAX_STREAMS` | `--max-streams` | *(unlimited)* | Process-wide stream limit for multiplexed protocols |
| `RELOAD_GRACE_SECS`| `--reload-grace-secs`| `300` | Grace period in seconds for TCP connection draining |

> [!TIP]
> **Persistent Configuration:** To keep your client connection link and keys identical across future server restarts, set the `UUID`, `SHORT_ID`, and `PRIVATE_KEY` environment variables.

---

## 🛡️ Advanced Anti-Censorship & DPI Evasion

`vless-RS` includes built-in mitigations for aggressive TSPU and state-level DPI engines:

1. **UDP-over-TCP Multiplexing (XUDP):** Enabled by default (`--enable-udp true`). Censors frequently drop or throttle UDP (HTTP/3 QUIC, WireGuard, DNS on UDP 53). XUDP encapsulates all client UDP traffic inside the established TLS 1.3 TCP stream.
2. **Cloudflare Clean Anycast Remapping:** State DPI systems frequently block entire Cloudflare Anycast subnets (`104.21.0.0/16`, `172.67.0.0/16`). Set `--cloudflare-clean-ip 104.16.132.229` (or any unblocked CF IP) to transparently remap outbound traffic to unblocked ranges.
3. **Upstream Chaining with `greentunnelRS`:** Run [greentunnelRS](https://github.com/alikemalocalan/greentunnelRS) locally on port 8080 or socks5 port 1080 and pass `--upstream-proxy 127.0.0.1:8080`. Outbound connections automatically benefit from packet-level desynchronization (TCP OOB desync, SNI midpoint splitting, and TLS Record fragmentation).
4. **ShadowTLS v3 Dual Protocol Mode:** When middleboxes perform active SNI-to-IP correlation checks on REALITY camouflage hosts, switch to `--protocol shadowtls` to establish genuine Dual-Handshake camouflage with any remote host.

---

## 💻 Local Building & Testing

```bash
# Compile optimized release binary
cargo build --release

# Run with default parameters
./target/release/vless-RS

# Run with custom camouflage target and port
./target/release/vless-RS --port 8080 --dest dl.google.com:443 --sni dl.google.com

# Run with upstream DPI-evasion proxy and Cloudflare clean IP
./target/release/vless-RS --upstream-proxy 127.0.0.1:8080 --cloudflare-clean-ip 104.16.132.229

# Run unit tests
cargo test
```

---

## 📄 License

MIT License. Designed for open communication, digital privacy, and anti-censorship research.
