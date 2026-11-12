# HANDOFF 2026-11-12 — v97 Orrery Remote: xong phần làm được không cần chủ nhân

## Đang ở đâu
- Branch `feat/v97-mobile` (đã push). Plan `PLAN-v97-mobile.md`, hợp đồng `docs/remote-integration.md`, hướng dẫn `docs/orrery-remote.md`, ngoại lệ `docs/claude-code-integration.md` §11as, quyết định `DECISIONS.md` v97, bảng WP `PROGRESS-2026-11-12-v97.md`, brief / report / review `.planning/wp/v97/`.
- E2E trên app Tauri thật + điện thoại giả Rust qua LAN + relay thật: `node apps/shell/e2e/v97-remote.cjs` → 12/12 ĐẠT (`docs/qa/v97/WP-18/RESULT.md`).

## Đã xong (commit trên branch)
WP-0 hợp đồng · WP-2 proto Noise · WP-3 hub PTY nhiều viewer · WP-4 StoreTap · WP-5 gateway LAN / mDNS / ghép / relay client · WP-6 PTY bridge + ws.* + confirm + keep awake · WP-7 renderer + Settings · WP-8 viewer gateway · WP-9 relay CF · WP-10 app kết nối · WP-11 / 11b UI app · WP-12 viewer app + Monitor · WP-13 link Telegram / ntfy · WP-15 E2E · WP-16 docs · WP-17 dọn tồn · WP-18 sửa lỗi QA. Mọi gói bảo mật / concurrency có review opus độc lập.

## Còn lại (cần chủ nhân)
1. A21s: máy bị project khác (com.wejoy.ragdollplayground) chiếm suốt đêm → chưa đo terminal (WP-1), chưa chạy app thật với gateway thật trên điện thoại. Khi máy rảnh: cài `apps/mobile/build/app/outputs/flutter-apk/app-profile.apk` (hoặc build debug), bật Settings → Điều khiển từ xa, quét QR.
2. Relay: chạy `cd apps/relay && npm run deploy` (bản có cert Android debug + close control 1 phone; hệ thống chặn con deploy lần 2). Xác nhận account Cloudflare (đang Doanvu20122000).
3. WP-14 push native: tạo Firebase project `orrery-remote` (+ APNs key khi làm iOS).
4. iOS: build trên MacBook khi chủ nhân báo (chưa có `scripts/ios.command` cho apps/mobile — làm khi tới bước đó).
5. 2–3 pane Codex do hook quota-fallback mở lúc 4:40 — đừng Enter, đóng tay.

## Việc tồn nhỏ (không chặn)
- Browser args WebView2 chọn 1 lần lúc mở app (bật / tắt remote cần mở lại app); Mac chống throttle chỉ Web Lock.
- 3 file Flutter 415–424 dòng; `ws.send` log chưa có trong E2E (cần pane agent).
- Clippy toàn crate shell / core-bin: 8 lỗi sẵn có ngoài v97.
