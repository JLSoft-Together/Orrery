# Spike v98 WP-50bc: CA riêng + wss LAN + Web Push trên iPhone

Chỉ trong LAN. Package riêng, không thuộc workspace root. Khoá CA, khoá VAPID, subscription, kết quả nằm trong `state/` (gitignored).

## Lệnh (PC, trong thư mục này)

- Cài: `npm install --workspaces=false --include-workspace-root=false`
- Chạy: `node run.mjs` — tạo CA + cert lá nếu chưa có (cert lá tự cấp lại khi IP đổi, CA giữ nguyên), mở `https :4320` (origin A: trang, `/ping`, `wss /echo`), `http :4321` (chỉ hồ sơ CA + trang hướng dẫn), `https :4322` (origin B), in URL + QR. `Ctrl+C` tắt.
- Kiểm trên PC (khi `run.mjs` đang chạy): `node check.mjs` (TLS với CA, CORS, CSP, manifest, sw, icon, wss 64 KiB + RTT).
- curl (Schannel trên Windows cần `--ssl-no-revoke` vì CA không có CRL): `curl --ssl-no-revoke --cacert state/ca.cert.pem https://192.168.0.104:4320/ping`
- Push từ CLI: `node push.mjs "tiêu đề" "nội dung" [--delay=15]` — in mã trả của dịch vụ push (201 ok, 404 / 410 thì xoá subscription).
- IP khác: `SPIKE_IP=192.168.x.y node run.mjs`. Tạo lại CA: `node gen-ca.mjs --force` (iPhone phải cài lại hồ sơ).

## iPhone (cùng Wi-Fi với PC)

1. Camera quét QR trong terminal → mở `http://<IP>:4321/` bằng **Safari** → "Tải hồ sơ CA" → Cho phép.
2. Cài đặt → **Đã tải về hồ sơ** → Orrery spike CA → Cài đặt (nhập mã máy).
3. Cài đặt → Cài đặt chung → Giới thiệu → **Cài đặt tin cậy chứng nhận** → bật Orrery spike CA.
4. Safari mở `https://<IP>:4320/`. Không được có cảnh báo chứng chỉ (nếu có: CA chưa tin, đừng bấm vẫn truy cập). Bước 2 trên trang tự chạy fetch + wss.
5. Bấm lần lượt các link "Biến thể": A wildcard, B, B wildcard (mỗi trang tự chạy lại fetch + wss).
6. Chia sẻ → **Thêm vào MH chính** → mở app "Orrery spike" từ màn hình chính (bước 2 chạy lại ở chế độ PWA). Trong PWA thử lại link A wildcard.
7. Trong PWA: **Bật thông báo** → Cho phép. Rồi **Gửi push ngay** (app đang mở), **Gửi push thử sau 15 s** + khoá màn hình, và lần nữa + vuốt đóng app. Chạm từng thông báo: app phải mở với dòng "Mở từ push".
8. Tuỳ chọn: lặp bước 6–7 cho origin B (`https://<IP>:4322/`, thêm vào MH chính riêng).

## Kết quả

- Terminal của `run.mjs` in mọi kết nối (IP, UA, origin), lỗi TLS, log từ trang và service worker (`sw-push`, `sw-click`).
- Toàn bộ ghi vào `state/results.jsonl` (mỗi dòng 1 JSON: `kind` = `http` / `client` / `ws-open` / `ws-close` / `tls-error` / `subscribe` / `push-sent`; với `client` xem `step`, `ok`, `data`, `page.standalone`, `page.csp`).
- Bảng cần điền: `.planning/wp/v98/WP-50bc-report.md`.

## Firewall Windows

Không tự đổi. Wi-Fi ở hồ sơ Public; máy này đã có rule Allow inbound cho `C:\program files\nodejs\node.exe` (Public). Nếu iPhone không vào được, mở bằng PowerShell admin:
`netsh advfirewall firewall add rule name="Orrery spike LAN" dir=in action=allow protocol=TCP localport=4320-4322 profile=any`
Gỡ sau spike: `netsh advfirewall firewall delete rule name="Orrery spike LAN"`.

## Gỡ sau spike

- iPhone: Cài đặt → Cài đặt chung → **VPN & Quản lý thiết bị** → Orrery spike CA → Xoá hồ sơ. Xoá app "Orrery spike" khỏi màn hình chính.
- PC: xoá `state/` (khoá CA + VAPID) và `node_modules/`.
