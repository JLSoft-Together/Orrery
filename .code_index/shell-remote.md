# shell-remote: gateway Orrery Remote (điện thoại / PWA)

Đường vào: `apps/shell/src/remote/mod.rs`. Giao thức Noise: `apps/core/crates/core-remote-proto/src/lib.rs` (wasm cho PWA: `core-remote-wasm`).

## File chính

| File | Vai trò |
|---|---|
| `apps/shell/src/remote/host.rs` | cầu nối tới desktop (confirm trên điện thoại `ask_device`) |
| `apps/shell/src/remote/session.rs` | phiên Noise, request / event / keepalive |
| `apps/shell/src/remote/viewer.rs`, `viewer_map.rs`, `viewer_events.rs` | điện thoại làm viewer của pane (`Own` / `View`) |
| `apps/shell/src/remote/lan.rs`, `mdns.rs` | LAN cổng 4320 + mDNS |
| `apps/shell/src/remote/relay.rs`, `relay_setup.rs`, `cf_api.rs` | relay Cloudflare tự dựng (`apps/relay`) |
| `apps/shell/src/remote/pair.rs`, `keys.rs`, `link.rs` | pair QR, khoá, link mở pane |
| `apps/shell/src/remote/guard.rs`, `guard_ip.rs`, `alerts.rs` | chặn IP, nhật ký truy cập, cảnh báo bảo mật |
| `apps/shell/src/remote/push.rs`, `push_content.rs`, `push_seal.rs`, `push_setup.rs` | push FCM (nội dung niêm phong cho từng máy) |
| `apps/shell/src/remote/webpush.rs`, `push_web.rs` | Web Push RFC 8291 + VAPID |
| `apps/shell/src/remote/chat.rs`, `chat_src.rs` | chat điện thoại: đọc transcript Claude / Codex |
| `apps/shell/src/remote/pty_bridge.rs`, `phone_pty.rs`, `fit.rs` | terminal bytes + resize claim |
| `apps/shell/src/remote/workspace.rs`, `patch.rs`, `dashboard.rs` | state workspace / dashboard gửi điện thoại |
| `apps/shell/src/remote/install.rs`, `install_ops.rs` | cài app điện thoại (APK) |
| `apps/shell/src/remote/scope.rs`, `limits.rs`, `confirm.rs` | quyền theo nguồn, giới hạn, xác nhận |

## Bất biến

- Điều khiển từ xa tắt mặc định; env, secret, app key, PTY token không bao giờ rời máy.
- Hành động nguy hiểm xác nhận trên điện thoại (`remote/confirm.rs`); prompt từ điện thoại submit ngay (ngoại lệ docs §11au).
- Push mang nội dung đã mask secret, bỏ ký tự điều khiển / bidi, cắt 120 ký tự; cảnh báo bảo mật không bao giờ mang IP hay tên.
- Phone chỉ nhận FCM có nội dung khi đã chứng minh giải mã được trong 48 giờ gần nhất.

## Liên kết

- Docs: `docs/orrery-remote.md`, `docs/remote-integration.md`. Web: `web-remote.md`. Khách: `pwa.md`, `mobile.md`, `relay.md`. Plan: `.planning/PLAN-v97-mobile.md`, `PLAN-v98-ios-pwa.md`.
