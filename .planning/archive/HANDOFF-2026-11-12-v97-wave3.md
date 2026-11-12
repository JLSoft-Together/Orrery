# HANDOFF 2026-11-12 — v97 Orrery Remote, giữa lượt 3

## Đang ở đâu
- Branch `feat/v97-mobile` (đã push tới d3962232). Plan `PLAN-v97-mobile.md`, hợp đồng `docs/remote-integration.md`, quyết định `DECISIONS.md` v97, tiến độ + bảng WP `PROGRESS-2026-11-12-v97.md`, brief / report / review `.planning/wp/v97/`, pipeline `.planning/pipeline.json` (wave3 đang chạy).
- Chạy liên tục theo lệnh chủ nhân; commit từng gói ĐẠT; push cuối lượt; chỉ dừng ở Firebase và iOS MacBook.

## Đã xong (ĐẠT + commit)
WP-0 hợp đồng · WP-2 core-remote-proto · WP-3 hub PTY nhiều viewer · WP-4 StoreTap · WP-5 gateway LAN / mDNS / ghép QR / IK / relay client (opus security 2 vòng) · WP-7 renderer (phone source, ws.*, summary, claim, Settings "Điều khiển từ xa") · WP-9 relay `https://orrery-relay.jinvu.workers.dev` · WP-10 app: lớp kết nối (FRB, LAN / relay, ghép, cache).

## Đang chạy
- WP-6 (opus): gateway PTY bridge, ws.* qua `Shell::ask`, `ws.patch`, confirm trên điện thoại, keep awake, Web Lock chống throttle, 5 lỗi thấp WP-5.
- WP-11 (sonnet): UI app (Máy, Workspace, tab / pane, terminal + khung nhập, Mission Control, inbox, mở pane, confirm, offline). Ảnh từ Flutter Windows.

## Kế tiếp
1. Nghiệm thu WP-6 (opus review), WP-11 (flutter-reviewer + xem ảnh).
2. WP-8 viewer gateway (whitelist kênh desk + event sink theo thiết bị) → WP-12 viewer app → WP-13 deep link Telegram / ntfy (bounce relay `/o/…` đã có) → WP-15 QA (E2E Tauri + A21s qua LAN + relay) → WP-16 docs §11 + index.
3. WP-14 push: chờ chủ nhân tạo Firebase `orrery-remote`.

## Treo cho chủ nhân
- A21s bị project khác (com.wejoy.ragdollplayground) dùng suốt đêm → WP-1 chưa đo terminal trên máy; APK profile ở `apps/mobile/build/app/outputs/flutter-apk/app-profile.apk`.
- Relay deploy lên account Cloudflare Doanvu20122000 (wrangler có 3 account) — đúng không?
- Clippy toàn crate `apps/shell` / `core-bin` có 8 lỗi sẵn có ngoài v97 (host_extra, preview_*, vault, core-desk).
