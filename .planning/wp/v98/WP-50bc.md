## WP-50bc: Spike iPhone — Safari tin CA tự cài + `wss://` LAN từ trang https; Web Push tới iPhone thật

- Mục tiêu: dựng harness để người cầm iPhone thật (cùng Wi-Fi với PC 192.168.0.104) trả lời 2 câu, kết quả tự gửi về PC:
  (b) Sau khi cài hồ sơ CA riêng (`.mobileconfig`) + bật tin cậy đầy đủ, trang tải từ origin https LAN (CA riêng, cùng origin và khác origin) có mở được `wss://192.168.0.104:4320` không — trong Safari thường VÀ trong PWA đã "Thêm vào MH chính"; có CSP `connect-src` chặt.
  (c) PWA (origin https LAN với CA riêng) đã thêm màn hình chính đăng ký Web Push (VAPID) và nhận được push có nội dung (kể cả khi khoá màn / app đóng), chạm mở đúng trang.
- Spec: `.planning/PLAN-v98-ios-pwa.md` quyết định #5, #15, #16, #25, #28, WP-50 (b)(c), rủi ro Safari / Web Push.
- Phạm vi file được đụng: chỉ tạo mới `scripts/spike-v98/lan/**` (package riêng, KHÔNG thêm vào workspace root; cài bằng `npm install --workspaces=false --include-workspace-root=false` trong thư mục đó), `.gitignore` của thư mục đó (khoá CA, khoá VAPID, subscription, node_modules, log), `docs/qa/v98/WP-50bc/**`, báo cáo. Cấm đụng mọi thứ khác.
- Model: opus. Song song với WP-50a (cargo) — bên này chỉ Node.

### Việc
1. `gen-ca`: CA riêng (ECDSA P-256 hoặc RSA 2048, basicConstraints CA critical, keyUsage keyCertSign) + chứng chỉ lá cho các IPv4 LAN của máy (SAN IP, gồm 192.168.0.104) + `localhost`, EKU serverAuth, hạn ≤ 397 ngày (luật iOS cho CA người dùng cài: SAN bắt buộc, ≤ 825 ngày). Sinh `.mobileconfig` (`com.apple.security.root`, UUID mới, tên "Orrery spike CA") + `.cer`. Chọn thư viện (`@peculiar/x509`, `node-forge` hoặc openssl Git `/mingw64/bin/openssl`) — ghi lý do.
2. Server LAN: `https` + `ws` trên `0.0.0.0:4320`, cert lá: `GET /ping` (CORS cho origin :4322), WS `/echo` (echo binary, đo RTT, nhận khung 64 KiB), log mọi kết nối (IP, UA, origin).
3. KHÔNG dùng tunnel ra internet (chủ nhân chưa duyệt). Mọi thứ chỉ trong LAN. Server LAN ở bước 2 phục vụ luôn trang app trên `https://<IP LAN>:4320/` (origin A), và 1 listener https thứ 2 `:4322` (origin B, cùng cert) để thử trường hợp khác origin: trang tải từ B nối `wss://<IP>:4320`. Thêm http thường `:4321` CHỈ để tải `/ca.mobileconfig` + trang hướng dẫn trước khi tin CA (không có gì khác). Phục vụ:
   - trang test (vi, nút to dễ bấm trên điện thoại) + `manifest.webmanifest` (`display: standalone`, icon PNG 192 / 512 tự sinh) + `sw.js` (nhận `push`, `showNotification` với title/body từ payload, `notificationclick` mở `/?from=push&id=…`);
   - CSP header chặt trên cả A và B: không inline script, `connect-src 'self' wss://<IP LAN>:4320 https://<IP LAN>:4320`; ghi thêm 1 biến thể CSP `wss://*:4320` để thử wildcard có chạy không;
   - `/ca.mobileconfig` (content-type `application/x-apple-aspen-config`);
   - `POST /log` (trang gửi kết quả từng bước về: UA, `navigator.standalone`, `display-mode`, lỗi WebSocket / fetch, RTT) → in console + ghi `results.jsonl`;
   - `POST /subscribe` lưu PushSubscription; `POST /push` gửi Web Push có nội dung (thư viện `web-push`, khoá VAPID sinh 1 lần lưu file gitignored), có trễ tuỳ chọn (vd. 15 s để kịp khoá màn), và gửi cả từ CLI (`node push.mjs "title" "body"`); in mã trả của Apple (201 / 404 / 410 …).
   - Trang test các bước: (1) tải hồ sơ CA, (2) thử `fetch https://IP:4320/ping` + `wss://IP:4320/echo` (binary 64 KiB, RTT 10 lần), (3) hướng dẫn "Thêm vào MH chính" khi chưa standalone, (4) nút "Bật thông báo" (xin quyền chỉ khi người dùng bấm, chỉ khi standalone), (5) nút "Gửi push thử sau 15 s".
4. Một lệnh `node run.mjs`: sinh CA nếu chưa có, chạy các server, in URL http :4321 (hồ sơ CA) + URL https :4320 / :4322 + mã QR ASCII trong terminal (để iPhone quét bằng Camera), in hướng dẫn cài hồ sơ (Cài đặt → Đã tải hồ sơ → Cài; Cài đặt → Cài đặt chung → Giới thiệu → Cài đặt tin cậy chứng nhận → bật). `Ctrl+C` tắt sạch.
5. Tự kiểm trên PC trước (không có iPhone): curl / node client với `--cacert` CA vào `https://192.168.0.104:4320/ping` + wss echo; https :4320 / :4322 trả trang + manifest + sw; Playwright không bắt buộc. Ghi chú: Windows Firewall có thể hiện hộp thoại cho Node lắng nghe 4320 — không tự đổi firewall, chỉ báo cách mở (lệnh `netsh` cần admin) trong README.
6. `scripts/spike-v98/lan/README.md` ≤ 50 dòng: lệnh, các bước người cầm iPhone làm, chỗ đọc kết quả, cách gỡ hồ sơ CA khỏi iPhone sau spike.

### Tiêu chí nghiệm thu
- [ ] `node run.mjs` chạy được trên Windows này, in URL + QR; trang + manifest + sw tải được qua https LAN với CA riêng (kiểm bằng curl --cacert).
- [ ] Từ PC: `https://192.168.0.104:4320/ping` và `wss://…/echo` qua CA riêng chạy (kèm lệnh kiểm), cert lá đúng SAN / EKU / hạn (in `openssl x509 -text` tóm tắt).
- [ ] `node push.mjs` gửi được tới 1 subscription (thử với Chromium / Edge desktop mở https://localhost:4320 với CA tin tạm bằng cờ dòng lệnh nếu được, để chắc phần VAPID đúng trước khi có iPhone).
- [ ] Không có khoá / subscription nào bị git theo dõi (`git status` sạch phần đó).
- [ ] Danh sách bước iPhone + bảng kết quả cần điền (Safari thường / PWA; wss LAN; CSP IP cố định / wildcard; quyền thông báo; push khi mở / khoá màn / app đóng; chạm mở trang).

### Luật (bắt buộc)
- Không viết comment trong code (kể cả `/** */`, `#`, TODO); lý do ghi vào báo cáo.
- Không `git commit` / `push`. File mới ≤ ~400 dòng. Text trên trang test tiếng Việt là đủ (spike, không qua vue-i18n).
- Không cài gì toàn cục. Không đổi cấu hình Windows (firewall, proxy). Không mở tunnel / port forward ra internet, không tải cloudflared. Push đi ra qua dịch vụ push của Apple / Google là đúng mục đích spike.
- Bash: `rtk` có thể rewrite `grep` → dùng tool Grep hoặc `rtk proxy grep`.
- Khi xong: tắt hết server đã chạy để tự kiểm.

### Báo cáo
`.planning/wp/v98/WP-50bc-report.md`: đã làm gì (file), tự kiểm thật (lệnh + kết quả), các bước iPhone, rủi ro, gợi ý cho WP-51 (CA trong keychain desktop, Rust) và WP-56 (Web Push Rust, RFC 8291).

### Nhật ký
- 12/11: vòng 1 bị chặn vì tunnel ra internet → brief đổi sang chỉ LAN (origin A :4320, origin B :4322, http :4321 cho hồ sơ CA). Phần "trang từ origin công khai → wss LAN" để sau khi chủ nhân duyệt (tunnel) hoặc ở WP-53 (relay /app/).
- 12/11 19:50: `gen-ca.mjs` (@peculiar/x509, ECDSA P-256, lá 397 ngày, `.mobileconfig`) xong, openssl verify sslserver OK.
- 12/11 20:00: server A/B/HTTP + trang + sw + push + `run.mjs` + `check.mjs` xong; `node check.mjs` 27/27 PASS; curl Schannel cần `--ssl-no-revoke`.
- 12/11 20:04: Edge 157 thật: wss A/B/wildcard OK, CSP chặn host lạ, `push.mjs` → WNS 201, sw nhận + chạm toast mở `/?from=push`; 410 tự xoá subscription. Chrome subscribe treo (GCM không check-in).
- 12/11 20:10: server đã tắt, state reset cho iPhone, README + báo cáo `WP-50bc-report.md`. Chờ người cầm iPhone điền bảng.
