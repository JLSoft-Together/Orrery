# WP-30 report

## Nguyên nhân
Phone bị kill / mất mạng không đóng WS sạch thì relay DO không gọi `webSocketClose`/`webSocketError` (hibernation; text `"ping"` được auto-response, không tới desktop). Desktop gateway (`dashboard.rs` `Phone::run`) chỉ chờ `rx.recv()` không timeout cho phiên relay, nên `live` kẹt `connected via relay`, `push_watch` bỏ qua máy này. Tái hiện: `liveness_tests` fail trước khi sửa (pings 0, phiên không bị bỏ). LAN: `lan.rs` `pump` có IDLE 45 s + phone ping 30 s, không dính, không đổi.

## Đã đổi
- `apps/shell/src/remote/liveness.rs` (mới): `Liveness` (ping 15 s, dead 45 s).
- `dashboard.rs`: field `alive`, `Phone::watch`, arm `due(&mut self.alive)` -> `keepalive()` (hết hạn thì close 4400 "idle", else gửi Noise ping), `pong()` (ping đang chờ thì nhận là trả lời, không echo, tránh lặp), `heard()` mỗi message vào.
- `session.rs`: gọi `phone.watch` khi `Peer::Relay`. `ctx.rs`: `keepalive: Mutex<(Duration, Duration)>` (test đặt ms). `mod.rs`: khai báo module.
- `liveness_tests.rs` (mới, 2 test): phone im lặng -> bị ping, bị bỏ, relay nhận control close; phone echo -> sống qua deadline, không ping lặp.
- `docs/remote-integration.md` (mục keepalive), `.code_index/shell.md`.
- Không đổi relay, mobile, bundle relay (không cần).

## Tiêu chí 3/3
- Nguyên nhân có bằng chứng + test fail trước: đạt.
- Phiên chết relay bỏ ≤ ~45-60 s (tick 15 s, hết hạn ở tick thứ 3); LAN có sẵn IDLE 45 s (chưa có test riêng 45 s, xác nhận qua code): đạt.
- Không comment, file ≤ 400 dòng (dashboard.rs 395), index + doc: đạt.

## Test thật
`cargo test --locked -j 3 remote` (apps/shell): 141 passed, gồm 2 test mới. Chưa chạy flutter/relay test (không đổi).

## Edge
Mạng chập chờn: mọi frame từ phone reset timer, cần 3 ping liên tiếp mất (45 s) mới bỏ. Nhiều phone: mỗi phiên timer riêng. LAN + relay cùng lúc: ping chỉ trên phiên relay. Phone cũ không cần cập nhật (đã echo ping). Chi phí relay: ~8 gói/phút/phiên (20 message = 1 request). Còn hở (ngoài phạm vi): phía desktop không kiểm pong của text "ping" tới DO (desktop <-> relay half-open).

## Hot file cần thêm (apps/mobile/.code_index/mobile.md)
"Relay: desktop gửi Noise ping (kind 3) mỗi 15 s; `protocol.dart` echo khi `_pingOutstanding` false; không cần app gửi ping Noise trên relay (WP-30)."

## Commit message đề xuất
fix(remote): drop a relay phone session after 45 s of silence with a desktop-initiated Noise ping every 15 s, so remoteStatus and push no longer see a dead phone as connected
