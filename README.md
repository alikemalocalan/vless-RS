# 🛡️ VLESS + RAW + REALITY (Rust) — Sansürsüz İnternet Tüneli

> **Ağır DPI Sansürüne Karşı Saf Rust ile Geliştirilmiş, Sıfır Bağımlılıklı, Tek Binary VLESS-REALITY Sunucusu**  
> *Rusya TSPU (ТСПУ), İran ve benzeri gelişmiş sansür sistemlerini meşru Apple/Google TLS kamuflajı ile atlatmak üzere tasarlanmıştır.*

---

## 🎯 Projenin Doğuş Hikayesi ve Motivasyon

Bu projenin arkasındaki asıl motivasyon, **Rusya'nın ulusal sansür altyapısı (Rostelecom / TSPU)** üzerinde gerçekleştirdiğimiz canlı testler ve [`net4people/bbs`](https://github.com/net4people/bbs) küresel sansür araştırmacıları topluluğunun bulgularından doğmuştur.

### Neden Klasik Yöntemler Yetersiz Kaldı?
1. **DPI ve Paket Bölme Kısıtı:** `greentunnelRS` ile yaptığımız canlı testlerde X (Twitter) ve YouTube gibi servisleri SNI parçalama ile başarıyla açabildik. Ancak **Instagram (Meta)** gibi servislerde Rusya TSPU sistemi ile Meta'nın özel C++ `Fizz TLS` sunucu kütüphanesi arasında bir çıkmaza girildi:
   - Gecikmesiz paket gönderildiğinde TSPU arabelleği (reassembly) paketleri birleştirip sansür uyguluyor.
   - Gecikme konulduğunda ise Meta sunucusu bağlantıyı güvensiz sayıp kapatıyor (`os error 104: Connection reset by peer`).
2. **Cloudflare Worker (VLESS+WS) Neden Çöktü?**
   - **15–20 KB Eşik Engeli:** TSPU, Cloudflare WebSocket tünellerinde 15–20 KB veri aktarıldığı anda bağlantıyı donduruyor (flow freezing). Ping atılsa bile Instagram'da videolar ve resimler yüklenmiyor.
   - **Cloudflare ECH Engeli (Issue #417):** Rusya, Cloudflare'in ECH (`cloudflare-ech.com`) uzantısını ülke genelinde doğrudan blokluyor.
   - **`*.workers.dev` ve `*.pages.dev` Engeli:** Varsayılan Cloudflare domainleri operatörler bazında karartılıyor.

### 💡 Nihai Çözüm: VLESS + RAW + REALITY
Trafiği gizlemeye çalışmak yerine **tamamen meşru bir HTTPS trafiğine bürünmek (Camouflage)**.
- Sunucumuz, dışarıdan bakan bir sansür kutusu veya port tarayıcısı için **birebir Apple'ın `gateway.icloud.com:443` sunucusudur**.
- Yetkisiz hiçbir tarayıcı sunucunun bir VPN veya proxy olduğunu anlayamaz; çünkü sunucu gerçekten Apple'ın orijinal sertifikasını ve HTTP yanıtını teslim eder.
- Sadece bizim anahtarımıza (X25519) sahip olan Android istemcimiz tüneli açabilir.

---

## ⚡ Teknik Özellikler

- 🚀 **Saf Rust ve Sıfır Harici Bağımlılık:** Go, Xray-core, Python veya Nginx kurulumuna gerek yoktur. Tek bir bağımsız binary olarak derlenir.
- 🪶 **Ultra Hafif (~1.2 MB):** LTO ve strip optimizasyonlarıyla yalnızca 1.2 MB boyutundadır; minimum RAM (~15 MB) ve sıfır CPU yükü ile çalışır.
- 🛡️ **Aktif Tarama Dokunulmazlığı (Active Probe Immunity):** Sansür botları porta saldırdığında meşru hedefe transparent fallback yapar.
- ⚡ **Zero-Copy Asenkron Ağ:** Tokio ve `tokio::io::copy_bidirectional` ile hat hızında (wire-speed) sıfır gecikmeli veri aktarımı.
- ☁️ **Railway.com Otomatik Dağıtım:** Railway TCP Proxy ve `$PORT` ortam değişkeniyle tam uyumlu; GitHub'a push edildiğinde 1 dakikada yayına girer.
- 📲 **Tek Tıkla Android Kurulumu:** Sunucu açıldığında `v2rayNG`, `NekoBox` ve `Xray-core` için panoya kopyalanabilir `vless://` bağlantısını konsola otomatik basar.

---

## 🏗️ Mimari ve Çalışma Prensibi

```text
[ Android İstemci (v2rayNG / NekoBox) ]
                 │
                 │  TLS 1.3 ClientHello
                 │  - SNI: gateway.icloud.com
                 │  - SessionID: İstemci X25519 Açık Anahtarı + ShortID
                 ▼
      [ vless-vpn (Port 443 / TCP Proxy) ]
                 │
                 ├──► [1. TLS ClientHello Koklama (Sniffing)]
                 │
                 ├──► GEÇERSİZ / DPI AKTİF TARAYICISI İSE:
                 │    └──► Şeffaf Fallback (Reverse-Proxy) -> gateway.icloud.com:443
                 │         (Karşı taraf orijinal Apple sertifikasını ve web sayfasını alır.
                 │          Sansür kutusu sunucunun proxy olduğunu ASLA tespit edemez!)
                 │
                 └──► DOĞRULANMIŞ REALITY İSTEMCİSİ İSE:
                      ├── X25519 Diffie-Hellman + HKDF-SHA256 Ortak Anahtar Türetimi
                      ├── ShortID Doğrulaması Başarılı!
                      ├── VLESS Başlığı Çözümleme (UUID, Hedef: instagram.com:443)
                      └──► Ham Hedefe Tünelleme (Instagram, X, YouTube)
```

---

## 🚀 Railway.com Üzerinde 1 Dakikada Dağıtım

### 1. Adım: GitHub Deponuzu Bağlayın
1. Projenizi GitHub'a push edin.
2. [Railway.com](https://railway.com) paneline gidin.
3. **+ New Project** -> **Deploy from GitHub repo** seçeneğiyle bu repoyu seçin.
4. Railway depodaki [`railway.toml`](../railway.toml) dosyasını otomatik algılayıp Dockerfile üzerinden derlemeyi başlatacaktır.

### 2. Adım: TCP Proxy'yi Açın (REALITY için Zorunlu!)
REALITY ham TCP üzerinden çalıştığı için Railway'de TCP Proxy aktif edilmelidir:
1. Railway servisinizin üzerine tıklayın.
2. **Settings** -> **Networking** bölümüne kaydırın.
3. **TCP Proxy** butonuna tıklayın. Railway size anında bir genel adres ve port atayacaktır:
   ```text
   Örnek TCP Adresi : roundhouse.proxy.rlwy.net
   Örnek TCP Portu  : 12345
   ```

### 3. Adım: Android Linkini Alın
1. Railway panelindeki **Deploy Logs** sekmesini açın.
2. Sunucu başladığında loglara doğrudan kopyalanabilir link basılır:
   ```text
   ===============================================================================
     🚀 VLESS + RAW + REALITY SERVER IS ACTIVE!
   -------------------------------------------------------------------------------
     UUID        : e596393a-47d2-4ff2-ad8a-e5edc8a078e1
     Camouflage  : gateway.icloud.com:443
     SNI         : gateway.icloud.com
     Port        : 12345
   -------------------------------------------------------------------------------
     📲 ANDROID IMPORT LINK (v2rayNG / NekoBox / Xray):
     vless://e596393a-47d2-4ff2-ad8a-e5edc8a078e1@roundhouse.proxy.rlwy.net:12345?encryption=none&flow=&security=reality&sni=gateway.icloud.com&fp=chrome&pbk=...&sid=...&type=tcp&headerType=none#Russia-TSPU-Bypass-Rust
   ===============================================================================
   ```

---

## 📱 Android İstemci Kurulumu (v2rayNG / NekoBox)

1. Railway Deploy Logs ekranında çıkan `vless://...` linkini kopyalayın.
2. Android cihazınızda **v2rayNG** (Google Play veya GitHub) uygulamasını açın.
3. Sağ üstteki **+** simgesine dokunun -> **"Import config from clipboard" (Panodan içe aktar)** deyin.
4. Eklenen sunucuyu seçip alt kısımdaki **V** (Bağlan) butonuna basın.
5. Artık Rusya, İran veya herhangi bir sansürlü bölgeden Instagram, X ve tüm dünyaya engelsiz ve şifreli olarak bağlısınız!

---

## ⚙️ Ortam Değişkenleri (Environment Variables)

Railway panelindeki **Variables** sekmesinden özelleştirebileceğiniz ayarlar:

| Değişken | Varsayılan | Açıklama |
|---|---|---|
| `PORT` | `8080` | Sunucunun dinleyeceği port (Railway otomatik atar) |
| `BIND` | `0.0.0.0` | Bağlanılacak ağ arayüzü |
| `UUID` | *Otomatik Üretilir* | VLESS İstemci UUID kimliği |
| `PRIVATE_KEY` | *Otomatik Üretilir* | REALITY X25519 32-byte gizli anahtarı (hex veya base64) |
| `SHORT_ID` | *Otomatik Üretilir* | REALITY İstemci ayrıştırma kimliği (hex) |
| `DEST` | `gateway.icloud.com:443` | Yetkisiz tarayıcıların aktarılacağı meşru kamuflaj hedefi |
| `SNI` | `gateway.icloud.com` | TLS el sıkışmasında taklit edilecek meşru alan adı |
| `SERVER_ADDRESS`| *Otomatik* | Paylaşım linkine yazılacak genel IP / Railway TCP domaini |
| `SERVER_PORT` | `$PORT` | Paylaşım linkine yazılacak genel TCP portu |

---

## 💻 Yerel Çalıştırma & Test

```bash
# Bağımsız release binary derleme
cargo build --release

# Varsayılan ayarlarla başlatma
./target/release/vless-vpn

# Özel parametrelerle başlatma
./target/release/vless-vpn --port 443 --dest dl.google.com:443 --sni dl.google.com

# Birim testlerini çalıştırma
cargo test
```

---

## 🔬 Test ve Doğrulama

- **Birim Testleri:** VLESS v0 başlık çözümleme, IPv4/Domain/IPv6 hedef tünelleme ve X25519 el sıkışma testleri `%100` başarıyla geçmiştir.
- **DPI Aktif Tarama Simülasyonu:** Sunucuya yabancı bir tarayıcı veya `curl` ile bağlanıldığında, sunucu hiçbir hata vermeden doğrudan Apple'ın gerçek sunucusuna fallback yaparak orijinal Apple sertifikasını sunmuş ve proxy kimliğini tamamen gizlemiştir.
