# v97 — Nghiên cứu thư viện cho app Flutter remote-control Orrery

Ngày: 2026-11-12. Phiên bản/ngày lấy từ pub.dev API (`https://pub.dev/api/packages/<tên>`), crates.io API, GitHub API, đọc ngày 2026-11-12. Mục nào không tự kiểm chứng được ghi **CHƯA XÁC MINH**. Chưa chạy thử code nào.

## Tóm tắt khuyến nghị

| Mục | Chọn | Lý do ngắn |
|---|---|---|
| Terminal | `xterm` 4.0.0 làm mặc định, spike sớm; fallback xterm.js trong WebView | xterm.dart không release từ 2024-02, repo push cuối 2025-06 |
| mDNS | `bonsoir` 7.1.6 (hoặc `nsd` 5.0.1) | cả hai còn bảo trì; `multicast_dns` chỉ là client Dart thuần |
| E2E | Noise XX: `snow` (Rust) + tự code trên `cryptography` (Dart) | không có package Dart Noise còn sống |
| TLS pin | `HttpClient` + `badCertificateCallback` so SHA-256 | khả thi, cần spike trên iOS thật |
| QR / key | `mobile_scanner` 7.5.0, `flutter_secure_storage` 11.2.0, `local_auth` 3.0.2 | đều release trong 2026 |
| Relay | DO + WebSocket Hibernation, 1 DO / machineId | Free plan có DO (SQLite-backed) |
| Push | FCM v1 từ Worker khả thi; APNs trực tiếp CHƯA XÁC MINH (HTTP/2) | iOS đi qua FCM là đường an toàn |
| State | Riverpod 3.5.0 | stream + snapshot + patch |

## 1. Terminal emulator Flutter

**`xterm` (TerminalStudio/xterm.dart)**
- Latest 4.0.0, published 2024-02-27 (https://pub.dev/api/packages/xterm). Tag mới nhất `v4.0.0` (https://api.github.com/repos/TerminalStudio/xterm.dart/tags). Repo push cuối 2025-06-19, `open_issues_count` 111 gồm cả PR (https://api.github.com/repos/TerminalStudio/xterm.dart). Trang repo ghi 78 issues + 33 PR (https://github.com/TerminalStudio/xterm.dart). Archived: false.
- README claim: 60fps, CJK + emoji, "Works with IMEs", mobile + desktop, MIT, Flutter >= 3.0 (https://github.com/TerminalStudio/xterm.dart). Truecolor / alt screen / mouse / scrollback: README không nêu, **CHƯA XÁC MINH** — cần test bằng output thật của Claude Code / Codex.
- Issues IME còn mở: #164 "IME (Korean) support is not working properly" (2023-02), #30 "Input handler gets triggered endlessly on macOS embedding" (https://github.com/TerminalStudio/xterm.dart/issues?q=is%3Aissue+is%3Aopen+IME). Chưa thấy issue Vietnamese cụ thể; Telex/VNI trên iOS dùng marked text nên rủi ro cao. Đề xuất: dùng input line riêng (TextField Flutter) thay vì gõ trực tiếp vào TerminalView, khớp thiết kế "Vietnamese input line".
- Hiệu năng scrollback lớn: **CHƯA XÁC MINH**. Đề xuất giới hạn scrollback phía phone (vd 2k-5k dòng) và benchmark với output TUI redraw liên tục.
- Phone chỉ nhận output (không có PTY cục bộ): dùng `Terminal.write` và không cần `flutter_pty` (0.4.2, 2025-01-06, https://pub.dev/api/packages/flutter_pty).

**Phương án khác**
- `terminal_view` 0.5.0 (2026-10-14, https://pub.dev/packages/terminal_view, repo Termphin/terminal_view): mới, chất lượng **CHƯA XÁC MINH**, chưa đọc source.

**Fallback xterm.js trong WebView**
- `webview_flutter` 4.14.1 (2026-07-07, https://pub.dev/api/packages/webview_flutter), Flutter team duy trì. `flutter_inappwebview` latest trên pub.dev 6.1.5 published 2024-10-08 (https://pub.dev/api/packages/flutter_inappwebview); bản mới hơn (nếu có) **CHƯA XÁC MINH**.
- Pros: xterm.js trưởng thành (truecolor, unicode addon, IME qua textarea của trình duyệt); tái dùng được renderer web của Orrery desktop.
- Cons: bridge JS channel cho mỗi chunk output, bàn phím ảo + focus WebView hay lỗi, RAM thêm cho mỗi WebView/pane, khó đồng bộ theme/font với Flutter. Đây là nhận định kinh nghiệm chung, không có nguồn trích dẫn.
- Khuyến nghị: spike 1-2 ngày với `xterm` trên máy thật (iPhone + Android tầm trung) bằng một ghi âm Claude Code TUI; nếu sai màu / wide char / lag thì chuyển WebView.

## 2. mDNS discovery

| Package | Version | Ngày | Ghi chú |
|---|---|---|---|
| `bonsoir` | 7.1.6 | 2026-10-25 | Android/iOS/macOS/Linux/Windows, dùng API native (https://pub.dev/packages/bonsoir) |
| `nsd` | 5.0.1 | 2026-04-04 | Android API 21+, iOS 13+, macOS, Windows (https://pub.dev/packages/nsd) |
| `multicast_dns` | 0.3.3+1 | 2026-06-09 | Flutter team, client Dart thuần (https://pub.dev/api/packages/multicast_dns) |

- Bonsoir: `BonsoirService.hostAddresses` là địa chỉ đã resolve; hostname `.local` không đảm bảo resolve được -> dùng `hostAddresses`, dựng URI bằng `Uri(...)` (IPv6 cần ngoặc) (https://pub.dev/packages/bonsoir).
- nsd iOS: `Info.plist` cần `NSLocalNetworkUsageDescription` + `NSBonjourServices` (vd `_orrery._tcp`). Android: `INTERNET` + `CHANGE_WIFI_MULTICAST_STATE` (https://pub.dev/packages/nsd). Service type phải đúng RFC 6335.
- iOS local network (iOS 14+): prompt hiện khi browse Bonjour hoặc kết nối tới IP private; service thiếu trong `NSBonjourServices` bị chặn; không có API hỏi trạng thái quyền, từ chối báo `kDNSServiceErr_PolicyDenied`; Simulator không bắt quyền. Trang TN3179 fetch chỉ trả tiêu đề nên phần này là kiến thức nền, **CHƯA XÁC MINH từ trang** (https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy).
- Android multicast lock: plugin dùng NsdManager hệ thống nên thường không cần tự giữ `MulticastLock`; `multicast_dns` thuần Dart thì cần tự quản. **CHƯA XÁC MINH từ doc**.
- Khuyến nghị: `bonsoir` + luôn có đường dự phòng nhập IP:port / QR chứa địa chỉ (mDNS hay bị chặn ở Wi-Fi công cộng / AP isolation). Phía desktop Rust: crate `mdns-sd` (phiên bản **CHƯA XÁC MINH**).

## 3. Noise Protocol

**Dart**
- `noise_protocol` 0.1.1, published 2020-05-11, nằm trong repo dint-dev/cryptography (https://pub.dev/api/packages/noise_protocol): không bảo trì, bỏ.
- `cryptography` 2.9.0 (2025-11-21) có X25519, `Chacha20.poly1305Aead`, BLAKE2s/BLAKE2b, HKDF, SHA-256; `cryptography_flutter` 2.3.4 dùng API native (https://pub.dev/packages/cryptography). Đủ primitive để tự code `Noise_XX_25519_ChaChaPoly_BLAKE2s` (ước ~200-300 dòng).
- Package Noise khác còn bảo trì cho Dart: không tìm thấy, **CHƯA XÁC MINH** là không tồn tại.

**Rust**
- `snow` 0.10.0, 2025-07-19, theo Noise spec rev 34, pure Rust, có `ring-resolver`, **chưa audit chính thức**; GitHub mcginty/snow 48 open issues, push cuối 2026-07-06 (https://docs.rs/crate/snow/latest , https://api.github.com/repos/mcginty/snow , https://crates.io/api/v1/crates/snow). ChaChaPoly + SHA256/BLAKE2s + Curve25519 được hỗ trợ; trang docs.rs không liệt kê pattern XX (XX là pattern chuẩn của spec) nên nên xác nhận bằng test.

**Rủi ro interop**: bản Dart tự viết phải khớp byte-for-byte với snow (prologue, thứ tự message, tag 16 byte, nonce counter 8 byte little-endian cho ChaChaPoly, rekey). Bắt buộc test vector chéo hai phía trong CI.

**So sánh**

| | Noise XX (tự code Dart) | TLS 1.3 pin (LAN) + E2E tự chế qua relay |
|---|---|---|
| Độ phức tạp | Một giao thức cho cả LAN và relay | Hai lớp, phải thiết kế handshake riêng cho relay |
| Rủi ro | Interop, tự code crypto phía client | Tự chế handshake rủi ro hơn code theo spec |
| Mutual auth, forward secrecy | Có (XX, static key hai phía) | TLS: server auth bằng pin; client auth cần thêm |

Khuyến nghị (ý kiến, không từ nguồn): một lớp Noise XX (pair lần đầu, sau đó IK hoặc XX lại) chạy trên mọi transport. TLS self-signed ở LAN chỉ là lớp phụ, có thể bỏ. Không tự chế handshake.

## 4. TLS pinning self-signed trong Dart

- `HttpClient.badCertificateCallback(cert, host, port)` chỉ được gọi khi cert không qua validate chuẩn (self-signed rơi vào đây) -> trả `true` khi SHA-256 của `cert.der` khớp fingerprint đã pair. Cách khác: `SecurityContext(withTrustedRoots: false)` + `setTrustedCertificatesBytes(der)` (https://www.zetcode.com/dart/securitycontext/ ; tổng hợp từ tìm kiếm).
- WebSocket: `WebSocketChannel.connect(uri, customClient: client)` (ví dụ trong kết quả tìm kiếm). Trang pub.dev của `web_socket_channel` 3.0.3 (2025-04-17) không mô tả tham số này; cần kiểm API reference của `IOWebSocketChannel`. Issue dart-lang/sdk #34284 hỏi `customClient` cho `WebSocket.connect` (https://github.com/dart-lang/sdk/issues/34284); trạng thái hiện tại **CHƯA XÁC MINH**.
- Android + iOS: `dart:io` dùng stack TLS của Dart nên logic giống nhau; chưa có nguồn chính thức -> **CHƯA XÁC MINH, cần spike trên máy thật**.
- Không so cert bằng tên host; khi đi bằng IP callback vẫn được gọi, so fingerprint là đủ.

## 5. QR, secure storage, biometrics

- `mobile_scanner` 7.5.0 (2026-11-01); repo push 2026-11-10, 72 open issues (https://pub.dev/packages/mobile_scanner , https://api.github.com/repos/juliansteenbakker/mobile_scanner). Android + iOS + macOS + Web. Android mặc định bundle ML Kit (+3-10 MB), tuỳ chọn unbundled `dev.steenbakker.mobile_scanner.useUnbundled=true` (+~600 KB, tải qua Play Services). iOS cần `NSCameraUsageDescription`; dùng AVFoundation + Vision. minSdk / iOS target: trang không nêu, **CHƯA XÁC MINH**.
- `flutter_secure_storage` 11.2.0 (2026-09-16): Android minSdk 23; EncryptedSharedPreferences đã bỏ, mặc định RSA-OAEP bọc AES-GCM; `AndroidOptions.biometric()` lưu key trong Keystore; iOS Keychain `accessibility` mặc định `unlocked` (nên dùng `first_unlock_this_device` nếu cần đọc khi app chạy nền). Lỗi hay gặp: Android auto-backup gây `InvalidKeyException` -> `android:allowBackup="false"` (https://pub.dev/packages/flutter_secure_storage).
- `local_auth` 3.0.2 (2026-07-09, Flutter team, https://pub.dev/api/packages/local_auth). iOS cần `NSFaceIDUsageDescription` (kiến thức nền, **CHƯA XÁC MINH** từ trang).
- Gợi ý: lưu static key Noise + fingerprint desktop trong secure storage, mở khoá app bằng `local_auth`.

## 6. Cloudflare Durable Objects (relay)

- WebSocket Hibernation: `ctx.acceptWebSocket(ws)` (khác `ws.accept()`) cho phép DO hibernate; handler `webSocketMessage/Close`; constructor chạy lại khi thức dậy; `serializeAttachment` max 16,384 byte, mất khi kết nối đóng; ping tự auto-response và không đánh thức DO; compat date 2026-04-07 trở đi runtime tự trả Close frame. Không hibernate nếu có alarm, `setTimeout/setInterval`, hoặc outbound WebSocket (giữ tới 15 phút) (https://developers.cloudflare.com/durable-objects/best-practices/websockets/). `setWebSocketAutoResponse` và tags không có trong trang này -> **CHƯA XÁC MINH từ trang**.
- Limits: message nhận max 32 MiB; CPU 30 s mỗi request/message (reset mỗi event; DO SQLite chỉnh tới 5 phút); soft limit 1,000 req/s mỗi object, vượt thì queue rồi lỗi "overloaded"; số connection tối đa mỗi DO: trang không nêu (https://developers.cloudflare.com/durable-objects/platform/limits/). Nên chia frame nhỏ (vài trăm KB) thay vì dựa vào 32 MiB.
- Giá (https://developers.cloudflare.com/durable-objects/platform/pricing/): Free: 100,000 req/ngày, 13,000 GB-s/ngày, chỉ DO SQLite-backed. Paid: 1M req/tháng gồm sẵn, sau đó $0.15/triệu; 400,000 GB-s/tháng, sau đó $12.50/triệu GB-s. Duration tính 128 MB/DO. **Message WebSocket đến tính 20:1** (100 message = 5 request); message đi ra và ping không tính; tạo connection = 1 request. DO idle đủ điều kiện hibernate không tính duration (pending I/O có thể giữ tới 15 phút). SQLite storage billing bật từ 2026-01-07: Free 5M rows read/ngày, 100k rows written/ngày, 5 GB.
- Free hay Paid: relay chỉ forward, không cần storage -> Free đủ để dev. 100k req/ngày ~ 2M message vào/ngày; stream terminal nhiều message nhỏ sẽ chạm nhanh khi dùng thật, nên gộp frame (batch 16-50 ms) và dự kiến Paid. Giá gói Workers Paid: **CHƯA XÁC MINH** từ nguồn đã đọc.
- Pattern đề xuất (thiết kế, không phải trích nguồn): `env.RELAY.idFromName(machineId)` -> 1 DO / machineId; endpoint `wss://.../ws/<machineId>?role=desktop|phone`; Worker xác thực token (HMAC/JWT do desktop cấp lúc pair) trước khi `stub.fetch`; DO lưu `role` trong attachment, forward frame sang phía còn lại, không đọc nội dung (E2E). Reconnect: exponential backoff + jitter, resume bằng sequence number cuối (xem mục 9).

## 7. Push (APNs / FCM)

- FCM HTTP v1 từ Worker: khả thi. Ký JWT RS256 bằng WebCrypto (`importKey` PKCS#8 + `RSASSA-PKCS1-v1_5`/SHA-256), đổi token tại `oauth2.googleapis.com/token` (grant jwt-bearer, scope `firebase.messaging`), gọi `fcm.googleapis.com/v1/projects/<id>/messages:send`; cache token tối đa ~55 phút, gặp 401 thì làm mới và thử lại một lần. Lib: `@sagi.io/workers-jwt`, crate `worker_jwt` (https://github.com/sagi/workers-jwt , https://docs.rs/crate/worker_jwt/0.2.0/source/src/google.rs). Endpoint legacy `/fcm/send` đã deprecated. Lưu service-account JSON bằng Worker secret.
- APNs trực tiếp: cần HTTP/2 + JWT ES256, host `api.push.apple.com:443`, POST `/3/device/<token>`; JWT làm mới không sớm hơn 20 phút và không muộn hơn 60 phút (https://developer.apple.com/documentation/usernotifications/establishing-a-token-based-connection-to-apns.md). Có fork `@fivesheepco/cloudflare-apns2` 13.0.0 cho Workers nhưng README không nói truyền tải bằng gì (https://unpkg.com/@fivesheepco/cloudflare-apns2@13.0.0/README.md). **CHƯA XÁC MINH** `fetch()` trong Workers tới `api.push.apple.com` có đi bằng HTTP/2 hay không (docs Workers không nêu); phải thử với sandbox host.
- Khuyến nghị: `firebase_messaging` 17.0.1 (2026-10-23, https://pub.dev/api/packages/firebase_messaging) cho cả hai nền tảng; iOS nhận qua FCM (FCM chuyển sang APNs) nên Worker chỉ cần một đường FCM v1. Cần upload APNs auth key (.p8) vào Firebase console.
- TestFlight push: cần Apple Developer Program (trả phí), capability Push Notifications + Background Modes (Remote notifications), APNs key. Đây là kiến thức nền, **CHƯA XÁC MINH từ nguồn đã đọc**.
- Nội dung push chỉ nói "có việc cần xem" kèm id, không chứa nội dung terminal (giữ E2E).

## 8. iOS background

- App đang chạy giữ network connection nhưng iOS thường suspend ngay sau khi vào nền; đã suspend thì socket mức thấp ngừng (https://developer.apple.com/forums/thread/716118 , https://developer.apple.com/forums/thread/799259).
- `beginBackgroundTask` cho ~30 giây, hệ thống có thể cho ít hơn khi bị áp lực; tạo nhiều task không kéo dài thêm (https://developer.apple.com/forums/thread/85066 , https://developer.apple.com/forums/thread/807588 , https://medium.com/@kkapilchoubisa/background-tasks-in-ios-d92d0a51dc25).
- WebSocket không phải background mode hợp lệ; Line / Slack / Telegram cũng dừng sau ~30 s (https://developer.apple.com/forums/thread/656778 ; https://docs.tryterra.co/help-center/help-topics/data-api-sdk/realtime-streaming-and-ble/realtime-keep-alive-backgrounded-ios, nguồn bên thứ ba).
- Tốt hơn reconnect-on-foreground: push FCM/APNs làm tín hiệu ("agent xong / cần duyệt"), mở app thì reconnect + resume từ sequence cuối (snapshot + replay). Không có đường chính thống giữ socket lâu hơn. Android foreground service: **chưa nghiên cứu**.

## 9. State management

- `flutter_riverpod` 3.5.0 (2026-11-06, https://pub.dev/api/packages/flutter_riverpod). Gợi ý: một `Notifier` giữ `ConnectionState` + `SessionsState` (snapshot ban đầu, rồi áp patch theo `seq`), mỗi terminal một notifier riêng cho buffer, UI `select` đúng field để tránh rebuild; mất seq -> xin snapshot lại. API 3.x khác 2.x, đọc migration guide trước.

## Việc spike tiếp theo
1. `xterm` trên iPhone + Android với ghi âm Claude Code TUI (màu, wide char, tốc độ, scrollback lớn).
2. `badCertificateCallback` + `customClient` trên iOS thật.
3. Cross-test vector Noise XX Dart <-> snow.
4. Worker `fetch()` -> APNs sandbox (HTTP/2) hoặc quyết định đi FCM.
5. Local network prompt trên iOS thật với `bonsoir`.
