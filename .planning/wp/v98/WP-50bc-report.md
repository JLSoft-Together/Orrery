# WP-50bc báo cáo: CA riêng + wss LAN + Web Push (harness, chờ iPhone thật)

Ngày 2026-11-12, nhánh `feat/v98-pwa`, chưa commit. Chỉ LAN, không tunnel. Server đã tắt sau tự kiểm (0 cổng 4320–4322 LISTEN).

## Đã làm (file)

Tất cả trong `scripts/spike-v98/lan/` (package riêng, cài bằng `npm install --workspaces=false --include-workspace-root=false`, root `package-lock.json` không đổi):

| File | Vai trò |
|---|---|
| `gen-ca.mjs` | CA ECDSA P-256 (basicConstraints CA critical pathlen 0, keyUsage keyCertSign+cRLSign critical, 825 ngày) + cert lá (SAN mọi IPv4 LAN + 127.0.0.1 + `localhost`, EKU serverAuth, keyUsage digitalSignature, basicConstraints CA:FALSE, SKI/AKI, đúng 397 ngày), `.cer` DER, `.mobileconfig` (`com.apple.security.root`, UUID mới, "Orrery spike CA"). Cert lá tự cấp lại khi IP đổi / sắp hết hạn, CA giữ nguyên |
| `server.mjs` | https A `0.0.0.0:4320` (trang, manifest, sw, icon, `/ping` CORS cho :4322, `wss /echo`, `/vapid`, `/log`, `/subscribe`, `/push` có trễ 0–120 s), https B `:4322` (cùng cert, cùng trang, không có WS), http `:4321` chỉ `/` hướng dẫn + `/ca.mobileconfig`. CSP chặt mỗi trang (`?csp=wild` → `wss://*:4320`). Log mọi request / WS / lỗi TLS ra console + `state/results.jsonl` |
| `push.mjs` | VAPID sinh 1 lần (`state/vapid.json`), lưu subscription, gửi `web-push` (TTL 3600, urgency high, payload JSON title/body/id/sentAt), in mã trả, xoá subscription khi 404 / 410; CLI `node push.mjs "t" "b" [--delay=15]` |
| `icons.mjs` | PNG 180 / 192 / 512 tự sinh (zlib, không thư viện) |
| `run.mjs` | 1 lệnh: CA nếu chưa có, VAPID, 3 listener, in URL + QR ASCII (`qrcode`) + hướng dẫn cài hồ sơ, `SIGINT`/`SIGTERM` tắt sạch |
| `check.mjs` | Tự kiểm từ PC (27 mục) |
| `public/index.html, app.js, style.css, sw.js, setup.html` | Trang test vi, không inline script; sw: `push` → `showNotification` + báo về `/log`, `notificationclick` → `/?from=push&id=…` |
| `README.md` (≈40 dòng), `.gitignore` (`node_modules/`, `state/`, `*.log`) | |

Bằng chứng: `docs/qa/v98/WP-50bc/pc-selftest.txt`, `docs/qa/v98/WP-50bc/pc-edge-push.jsonl`.

Chọn thư viện cert: `@peculiar/x509` 3.1 (+ `reflect-metadata`) trên WebCrypto của Node — thuần JS nên chạy cả Mac / máy không có Git Bash; API (extension từng cái) gần `rcgen` để chép sang Rust ở WP-51; ECDSA P-256 (node-forge chỉ RSA). openssl Git chỉ dùng để kiểm. Không comment trong code theo luật; lý do nằm ở đây.

## Tự kiểm thật trên PC

1. `node run.mjs` → in URL :4321 / :4320 / :4322 / `?csp=wild` + QR, nghe 3 cổng.
2. `node check.mjs` → **Tất cả PASS** (27 mục): cert lá ký bởi CA, SAN có 192.168.0.104, EKU serverAuth, 397 ngày, không CA thì TLS lỗi `UNABLE_TO_VERIFY_LEAF_SIGNATURE`, `/ping` 200 + CORS đúng origin B và không mở cho origin lạ, A và B trả trang + CSP IP cố định / wildcard + manifest (`application/manifest+json`, standalone, 2 icon) + sw.js + icon 192/512, :4321 trả `.mobileconfig` (`application/x-apple-aspen-config`) + trang hướng dẫn, mọi đường khác 404, `wss` echo 64 KiB khớp + 10 RTT (trung vị ~0.15–0.2 ms).
3. `curl --ssl-no-revoke --cacert state/ca.cert.pem https://192.168.0.104:4320/ping` → 200 + `access-control-allow-origin: https://192.168.0.104:4322`; trang / manifest / sw ở :4320 và :4322 đều 200. Không `--cacert` → `SEC_E_UNTRUSTED_ROOT`. Chú ý: curl Git và curl Windows đều Schannel, thiếu `--ssl-no-revoke` báo "revocation status is unknown" (CA không có CRL / OCSP) — iOS không đòi điều này với CA người dùng cài.
4. `openssl s_client -connect 192.168.0.104:4320 -CAfile state/ca.cert.pem -verify_ip 192.168.0.104` → TLSv1.3, `Verify return code: 0 (ok)`; `openssl verify -purpose sslserver` OK.
5. `.mobileconfig` parse XML được, payload DER mở ra đúng "CN=Orrery spike CA".
6. Edge 157 thật (profile tạm trong scratchpad, `--ignore-certificate-errors-spki-list=<SPKI lá>`, điều khiển qua CDP):
   - `https://localhost:4320`, `https://localhost:4322`, `https://localhost:4322/?csp=wild`, `https://192.168.0.104:4322/` → fetch `/ping` + wss echo 64 KiB + RTT OK, sw đăng ký OK.
   - Đối chứng CSP: từ trang strict mở `wss://127.0.0.1:4320` → bị chặn, `securitypolicyviolation connect-src` (CSP thật sự có hiệu lực).
   - Push: subscribe ra endpoint WNS; `node push.mjs` → **201**, sw nhận `sw-push` sau 114 ms, `getNotifications()` thấy đúng title/body/id; push hẹn 3 s qua `POST /push` → 201, sw nhận sau 434 ms; chạm toast Windows → `sw-click` → trang mở `/?from=push&id=76201646`. Subscription cũ → **410** và tự bị xoá.
   - Bài học: quyền cấp bằng CDP `Browser.grantPermissions` chỉ là override; push tới thì Edge kiểm quyền thật → "Permission denied" và tự unsubscribe. Phải đặt quyền vào Preferences của profile.
   - Chrome 157: `pushManager.subscribe` treo > 90 s (gcm-internals: GCM INITIALIZED, chưa check-in) dù mạng tới `mtalk.google.com:5228` thông → không dùng được để kiểm, Edge đủ chứng minh VAPID + mã hoá aes128gcm đúng.
7. Tắt sạch: `server.close()` với 1 WS đang mở xong trong 1 ms, process tự thoát, không còn LISTEN. Ctrl+C trong terminal thật chưa thử được từ agent (Windows không gửi SIGINT cho process nền), handler gọi đúng hàm đó.
8. `git status`: chỉ file nguồn untracked; `state/` (khoá CA, khoá lá, VAPID, subscription, results) và `node_modules/` bị `.gitignore` của thư mục bỏ qua (`git check-ignore` xác nhận). Kết quả PC chuyển thành `state/results-pc-selftest.jsonl`, subscriptions đã xoá để iPhone bắt đầu sạch; CA + VAPID giữ lại.
9. Firewall (chỉ đọc): Wi-Fi đang ở hồ sơ **Public**, đã có rule Allow inbound TCP/UDP cho `C:\program files\nodejs\node.exe` (Public) → iPhone nên vào được không cần đổi gì.

## Lệnh cho người thật

```
cd D:\Code\Tools\Claude-Code-Monitor\scripts\spike-v98\lan
node run.mjs
node push.mjs "Orrery spike" "Thử từ CLI" --delay=15
```

Bước iPhone: xem `scripts/spike-v98/lan/README.md` mục "iPhone" (8 bước). Kết quả tự về `state/results.jsonl` + terminal.

## Bảng kết quả cần điền (iPhone thật)

iOS: ____ · máy: ____ · Wi-Fi cùng mạng 192.168.0.x: ____

| # | Ca | Safari thường | PWA (MH chính) | Ghi chú (mã lỗi, RTT) |
|---|---|---|---|---|
| 1 | Tải + cài hồ sơ CA, bật tin cậy đầy đủ | | — | |
| 2 | Mở `https://IP:4320/` không cảnh báo | | | |
| 3 | A → fetch `https://IP:4320/ping` | | | |
| 4 | A → `wss://IP:4320/echo` 64 KiB + RTT, CSP IP cố định | | | |
| 5 | A → wss, CSP wildcard `wss://*:4320` | | | |
| 6 | B (:4322) → fetch A `/ping` (CORS) | | | |
| 7 | B → `wss://IP:4320`, CSP IP cố định | | | |
| 8 | B → wss, CSP wildcard | | | |
| 9 | Nút "Bật thông báo" bị khoá khi chưa standalone | | — | |
| 10 | Xin quyền thông báo → Cho phép, subscribe (`web.push.apple.com`) | — | | |
| 11 | `push.mjs` / nút → mã Apple (201 / 400 / 403 BadJwtToken / 410) | — | | |
| 12 | Push khi app đang mở (hiện banner?) | — | | |
| 13 | Push khi khoá màn (hiện đủ title + body trên màn khoá) | — | | |
| 14 | Push khi app đã vuốt đóng | — | | |
| 15 | Chạm thông báo → mở `/?from=push&id=…` (log `sw-click` + `opened-from-push`) | — | | |
| 16 | Độ trễ push (`sw-push.delayMs`) | — | | |

## Rủi ro / điều cần để ý

- iOS: nếu người dùng bấm "vẫn truy cập" qua cảnh báo cert, trang tải được nhưng wss / sw có thể hỏng → không tính là PASS; README dặn rõ.
- Tin cậy đầy đủ (Certificate Trust Settings) bắt buộc với CA từ hồ sơ; thiếu bước 3 thì Safari vẫn cảnh báo.
- WebKit có thể đối xử `wss://` tới IP riêng khác `fetch` (Private Network Access / ATS cho IP); bảng tách 2 dòng để thấy.
- Wildcard CSP `wss://*:4320` hợp lệ theo CSP3 nhưng WebKit có thể không khớp `*` với IP; nếu dòng 5 / 8 FAIL thì desktop phải ghi IP cụ thể vào CSP mỗi khi IP đổi (hoặc CSP do relay sinh theo IP báo về).
- Apple từ chối VAPID `sub` dạng `https://localhost` (BadJwtToken); harness dùng `mailto:orrery-spike@example.com` (đổi bằng `VAPID_SUBJECT`). Nếu Apple trả 403 thì thử `mailto:` thật.
- PWA mở link sang origin khác (B) sẽ bật trình xem Safari trong app, không phải PWA B; muốn thử PWA B phải thêm B vào MH chính riêng (bước 8, tuỳ chọn).
- iOS thu hồi subscription nếu push đến mà sw không `showNotification`; sw luôn hiện.
- Đổi IP LAN (DHCP) → cert lá cấp lại tự động, không cần cài lại CA; nhưng URL PWA đã thêm MH chính chứa IP cũ sẽ chết → bản thật cần IP ổn định hoặc mở qua relay rồi mới nối LAN (đúng quyết định #5).
- `/push`, `/subscribe` không xác thực — chỉ chấp nhận được cho spike trong LAN.

## Gợi ý cho WP-51 (CA trong keychain desktop, Rust)

- Rust: `rcgen` với `KeyPair` ECDSA P-256, cùng tập extension như `gen-ca.mjs` (CA: BasicConstraints critical pathlen 0, KeyUsage keyCertSign+cRLSign critical; lá: SAN IP mọi IPv4 LAN + 127.0.0.1 + localhost, EKU serverAuth, KeyUsage digitalSignature critical, SKI / AKI, ≤ 397 ngày, notBefore lùi 1 giờ). Khoá CA cất keychain / Credential Manager, cert lá + khoá lá có thể ở đĩa và cấp lại khi IP đổi mà không đổi CA.
- TLS: `rustls` + `tokio-tungstenite` cho listener `wss` song song `ws`; log lỗi TLS (iPhone chưa tin CA sẽ hiện ở đây — rất hữu ích để báo "chưa cài CA").
- `.mobileconfig`: plist như trong `gen-ca.mjs`, UUID cố định theo máy (để cài lại thay thế thay vì thêm hồ sơ mới), serve qua http với `application/x-apple-aspen-config`.
- Test chéo: dùng `check.mjs` của spike làm mẫu test tích hợp (CA → ping → wss 64 KiB).

## Gợi ý cho WP-56 (Web Push Rust, RFC 8291)

- Payload phải mã hoá `aes128gcm` (RFC 8188 / 8291) + VAPID JWT ES256 (RFC 8292): crate `web-push` (Rust) hoặc tự làm với `p256` + `hkdf` + `aes-gcm`; đối chiếu bằng `push.mjs` (Node `web-push` 4.0) với cùng subscription.
- Header: `TTL`, `Urgency: high` cho push có nội dung hiện trên màn khoá; xử lý 404 / 410 = xoá subscription (đã chứng minh trên WNS: subscription bị bỏ trả 410).
- VAPID `sub` không dùng `https://localhost`.
- Test desktop trước iPhone: Edge thật + profile tạm, đặt quyền notifications vào Preferences của profile (không dùng CDP grant), endpoint WNS trả 201 và sw nhận thật.
