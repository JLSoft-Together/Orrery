# WP-55a báo cáo: Home, ghép, Monitor, khoá, Cài đặt (apps/pwa)

## Màn đã làm (file + symbol)
- Router `src/router.ts` (`parsePath/toPath/navigate/back`, history thật, `idx` trong history.state, hướng push/pop cho motion); deep link `/m/<id>/p/<pane>` -> `workspace` route (stub `views/WorkspaceStub.vue`, "Sắp có").
- Home A `views/HomeView.vue` + `components/MachineCard.vue` (chip đường icon+chữ+màu, đếm đang chạy / chờ / rảnh, select Kết nối, chuông lưu IndexedDB), gộp bản trùng `lib/machineGroups.ts` + "Dọn bản cũ" (`lib/confirm.ts`, `ConfirmHost.vue`), empty `OrbitArt.vue`, FAB.
- Ghép `views/PairView.vue`: tab Quét QR (`components/QrScanner.vue`, getUserMedia + jsQR 1.4.0, không eval / worker) và Nhập mã (dán chuỗi base64url, `lib/qrInput.ts`); hiện LAN -> relay đang thử (`PairInput.onRoute`), tên thiết bị, lỗi denied / expired / badCode / noRoute / failed. `pairing.ts` + `noise.ts` giờ gửi `platform: "web"` qua `Pairing.fromQrOn` (wasm đã có, không đổi shell).
- Monitor `views/MonitorView.vue` (A thẻ + lọc, B nhóm theo trạng thái, nút đổi mode nhớ `prefs.monitorMode`, banner mất kết nối + Thử lại, dữ liệu cũ mờ), `lib/monitorModel.ts` (cùng luật với Android `monitor_model/filter/session_activity`), chi tiết `views/SessionView.vue` (sub-agent, 12 hoạt động, nút Mở terminal -> route Workspace).
- Khoá `security/webauthn.ts` (platform, `userVerification: required`, `residentKey: preferred`), `stores/lock.ts`, `views/LockView.vue`, `lib/lockPolicy.ts` (1 phút / 15 phút / 1 giờ mặc định / 4 giờ). `main.ts` chỉ `hub.start` sau khi mở khoá nên cache snapshot IndexedDB không được đọc khi đang khoá.
- Cài đặt `views/SettingsView.vue`, `SettingsMachines.vue`: khoá, thông báo (3 nhóm + Kèm nội dung, lưu cục bộ, `prefs.events` phát `bell` / `notify` cho WP-56), ngôn ngữ, chủ đề, đổi tên / quên máy, xoá dữ liệu, phiên bản + proto, vào Gỡ lỗi (DebugView chỉ còn log, form ghép QR v1 đã bỏ).
- iPad >= 768: master 380 px + detail (`SessionView embedded`), safe-area, rem + `-apple-system-body`, chạm >= 44 px, motion push 300 ms / pop 210 ms, `prefers-reduced-motion` có sẵn.
- Token: radius 10 / 8 / 4, dark = Galaxy (#000), thêm agent-codex / shell, scanner.

## Test thật
- `npm run typecheck -w @ccm/pwa`: sạch. `npm test -w @ccm/pwa`: 13 file, 129 pass / 0 fail (mới: `lib/logic.test.ts`: gộp máy trùng, lọc / nhóm / sắp Monitor, hoạt động, parse + giải mã QR thật bằng jsQR, hẹn giờ khoá; `app.test.ts`: router, WebAuthn giả, prefs). `npm run build -w @ccm/pwa`: sạch (dist.test pass: không inline script / style).
- `npm run e2e -w @ccm/pwa`: WebKit 19/19 + Chromium 19/19 = 38 pass / 0 fail. Chromium: quét QR từ camera giả (file y4m do `e2e/lib/y4m.mjs` vẽ) rồi ghép qua relay; khoá bằng virtual authenticator CDP (bật -> reload -> màn khoá không render app -> mở khoá -> tắt). WebKit: ghép bằng dán mã; khoá chỉ kiểm giao diện "không hỗ trợ" (headless WebKit không có platform authenticator). Không vi phạm CSP, không cuộn ngang.
- Peer E2E (`e2e/peer/src/conn.rs`) thêm dữ liệu giả: provider / branch / cwd, agents, activity.

## Ảnh `docs/qa/v98/WP-55a/` (390x844, 820x1180, sáng / tối, webkit + chromium)
home, monitor-a, monitor-b, detail, settings, pair-code, empty, pair-scan, pair-error, home-stale, monitor-offline, revoked, lock-phone-{light,dark}, settings-lock-unsupported.

## Khác mockup
- Mã ghép: dán chuỗi QR (textarea) thay ô 8 ký tự + mDNS (web không có mDNS, QR v2 không có mã ngắn).
- Thẻ Home thêm select Kết nối + chuông (brief); Quên máy chuyển sang Cài đặt.
- Chi tiết: chỉ nút Mở terminal; nút gửi / dừng là việc WP-55b.
- Khoá mặc định TẮT: WebAuthn cần thao tác người dùng để đăng ký, nên bật trong Cài đặt (Android mặc định bật).
- Ảnh fake có "315d" vì statusSince giả cố định.

## Đánh đổi bảo mật (cho reviewer)
WebAuthn chỉ là cổng giao diện; khoá giải mã dữ liệu vẫn là CryptoKey không trích xuất trong IndexedDB (`keyStore.ts`), không dẫn từ PRF. Ai đọc được profile trình duyệt vẫn không có khoá thô nhưng có thể dùng lại CryptoKey trong cùng origin. Chưa thử PRF (cần iOS 18+, chưa kiểm trên máy thật). Khoá chỉ chặn đọc cache khi khởi động nguội / sau thời gian nền; dữ liệu đã nạp trong RAM không bị xoá khi khoá (ẩn bằng màn khoá + `hub.background()`).

## Việc dở / cần main
- Shell / desktop: không cần gì (platform `web` đã được hợp đồng cho phép). WP-56 nối `prefs.events` + `notifyPrefs` vào `push.register`.
- Chưa thử trên Safari iOS thật: WebAuthn tự mở khoá lúc vào màn khoá có thể bị từ chối (cần chạm) nên có nút chạm; camera trong PWA đã cài.
- Mở khoá không tự khoá khi đang dùng liên tục (chỉ tính thời gian ẩn app).
- `docs/qa/v98/WP-54` giữ nguyên ảnh cũ.

## Hợp đồng cho WP-55b
- Router: thêm route trong `src/router.ts` (`workspace` đã có `machine` + `pane?`), render trong `App.vue` thay `WorkspaceStub` (cắm tại nhánh `master.name === 'workspace'`). `lib/useWide` cho layout iPad.
- Layout: `TabBar` Workspace tab đã trỏ `workspace` route; trang dùng `PageBar` (`backless` cho tab gốc).
- Dữ liệu: `hub.viewOf(machine).workspace` (tabs / panes), `SessionView` tìm pane theo `terminalId` / `sessionId`; lệnh qua `rt.hub.session(id)?.request`.
- Cần xterm: CSP `style-src 'self'` (xem WP-54 báo cáo).
