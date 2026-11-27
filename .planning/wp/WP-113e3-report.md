## WP-113e3 — DONE

Listener `127.0.0.1:P` chỉ sống theo slot ghép đôi `via:"usb"`; wizard gọi wrapper có kiểu thay cho bridge ép kiểu; QR trang cài lấy `ticket.page`.

### Đã đổi
Rust `apps/shell/src/remote/`:
- Mới `loopback.rs` (83 dòng): `supervise(ctx, stop)` nghe `pairing.usb_watch()`, slot usb sống thì bind `127.0.0.1:qr_port`, chạy `lan::accept_with(.., pairing_only=true)`; slot hết / huỷ / ghép xong / bị slot QR thay → đóng. Khi slot sống tick 1s để: bắt hết hạn, thử bind lại nếu cổng bận (eprintln 1 lần, chỉ in địa chỉ), đổi cổng nếu `config.port` / LAN đổi. Bỏ qua nếu LAN listener đã phủ đúng `127.0.0.1:P` (chỉ xảy ra ở rig test) hoặc port 0.
- Mới `loopback_tests.rs` (186 dòng, khai báo `#[path]` trong loopback.rs, không thêm dòng test mod vào mod.rs).
- `pair.rs`: `Pairing.usb` watch (exp khi slot là usb, None khi clear), `usb_until(now)` (dọn slot hết hạn), `usb_watch()`, `qr_port(ctx)` (QR và loopback dùng chung một cổng), `loopback_only(link)` = `origin.pairing && Peer::Lan`; `run` từ chối slot không phải usb trên link loopback: CLOSE_DEVICE + log mã `loopback`. Re-export `Via`.
- `lan.rs`: `accept_with(ctx, listener, stop, pairing_only)`; `accept_loop` giữ chữ ký cũ, gọi `accept_with(.., false)`; `handle` đặt `origin.pairing = pairing_only`. LAN listener không đổi hành vi.
- `rebind.rs`: `supervise` spawn `loopback::supervise(ctx, stop)` (dùng chung stop của Remote).
- `mod.rs`: +1 dòng `mod loopback;` (465 → 466).

Web `apps/web/src/features/remote/`:
- `lib/connectApi.ts`: bỏ `E1Bridge` / ép kiểu `desktop`; dùng `remotePairStart`, `remoteCheck`, `onInstallFetched(handler, since)` từ `remote.ts`, `installArm` từ `remoteInstall.ts`; thêm `installPagePath(ticket)` (ưu tiên `page` bắt đầu bằng `/`, fallback cắt `orrery.apk`). `ConnectApi.onInstallFetched` nhận thêm `since?`.
- `components/connect/useConnectWizard.ts`: QR dùng `installPagePath`, `since` = giây unix trước `installStart` trừ 1.
- `i18n/remote-v98.ts`: `remoteV98.guard.why.{pair_ip, held, install_pair, install_unarmed, install_missing, install_fetch, install_ip, install_used, loopback}` en + vi.
- Test: mới `lib/connectApi.test.ts` (mock `./remote`, `./remoteInstall`, devices); thêm 1 case vào `PhoneConnectWizard.test.ts` (page ưu tiên, since).
- Snapshot `apps/web/src/i18n/__snapshots__/keys.{en,vi}.txt` cập nhật bằng `-u`: chỉ +9 key `remoteV98.guard.why.*` ở trên (đã soát diff).

### Tiêu chí
- [x] Không slot usb → không bind (kể cả slot QR); `pair_start {via:"usb"}` → bind; ghép xong / cancel / slot QR thay / hết hạn (≤ exp + 2s, test đo) / tắt Remote → unbind.
- [x] Handshake loopback không đúng slot usb → 4401 + log `loopback`; session handshake (máy đã ghép) qua loopback → từ chối (mã `pairing`, do `session.rs` sẵn có chặn `origin.pairing`).
- [x] `/install` qua loopback → 403 (đi qua `install_pair::serve` như LAN, loopback không phải IP private).
- [x] LAN listener không đổi: `accept_loop` giữ nguyên, `bound()` / `addresses` không chứa loopback; test LAN cũ xanh.
- [x] Vitest: connectApi gọi wrapper thật (mock module), `page` ưu tiên, fallback cắt; wizard test cũ xanh + 1 case mới.
- [x] i18n en + vi; typecheck sạch; không comment; file mới ≤ 186 dòng, `pair.rs` 339, `lan.rs` 200, `mod.rs` chỉ +1 dòng được phép.

### Audit
- Diff chỉ trong phạm vi: `rebind.rs`, `lan.rs`, `pair.rs`, `mod.rs` (+1 dòng), `loopback*.rs`, `connectApi*.ts`, `useConnectWizard.ts`, `PhoneConnectWizard.test.ts`, `i18n/remote-v98.ts`, snapshot key. Không đụng `push*`, `webpush*`, `chat*`, `ctx.rs`, `session.rs`, `link.rs`, workspace, devices, settings, mobile, `apps/web/src/i18n/{en,vi}.ts`.
- Không thêm kênh IPC, không đổi `CHANNELS`.
- Không mở cổng nào khác ngoài `127.0.0.1:P` trong thời gian slot usb sống.

### Review (edge case)
- Cổng bận: bind lỗi → eprintln 1 lần (chỉ địa chỉ), thử lại mỗi 1s khi slot còn sống; test giữ cổng bằng listener khác rồi nhả → bind được.
- 2 lần `pair_start usb` liên tiếp: cùng cổng nên giữ listener, slot mới thay slot cũ (serial mới), mã cũ hết hiệu lực.
- Remote tắt khi đang bind: `stop()` gọi `pairing.cancel()` trước (watch None → đóng), rồi stop watch → supervisor thoát, runtime shutdown.
- Đổi port: target tính lại mỗi tick theo `qr_port` (LAN first port, không có LAN thì `config.port`); test đổi `config.port` → listener dời sang cổng mới, cổng cũ đóng.
- Race slot đổi giữa accept và msg1: check trong `pair::run` sau khi lấy ticket (serial cố định), nên link loopback đã accept trước khi slot QR thay vẫn bị từ chối (`loopback`), có test.
- Link đang dở khi listener đóng: không bị cắt, nhưng chỉ ghép được nếu slot usb còn đúng serial (`consume`).
- Strike: refuse trên loopback strike IP `127.0.0.1` như LAN (STRIKE_MAX 20 / 10 phút); probe TCP rỗng không strike.
- Bảo mật: tiến trình cục bộ bất kỳ có thể nối `127.0.0.1:P` trong 120s nhưng chỉ làm được handshake ghép với mã usb (giới hạn 5 lần / nguồn, 20 lần / slot) và vẫn phải qua hộp Cho phép.

### Test
- `cd apps/shell; cargo test --locked -j 4 remote` → `321 passed; 0 failed; 3 ignored` (trước gói 300 + test mới của gói khác). `remote::loopback` chạy riêng 3 lần → 5/5 mỗi lần (~4s).
- `cargo clippy --locked -j 4 --all-targets` → không cảnh báo mới ở `loopback*.rs`, `lan.rs`, `rebind.rs`; `pair.rs:62 method start is never used` có từ trước (không do gói).
- `npm test -w @ccm/web -- src/features/remote src/i18n` → 28 file, 210 test pass (sau `-u` snapshot key).
- `npm test -w @ccm/web -- src/importRules` → 4 pass.
- `npm run typecheck` → exit 0.
- `npm run qa -- --no-e2e` → 3/3 pass (cargo test shell, typecheck web, vitest related), exit 0.
- CHƯA KIỂM: luồng USB trên máy thật (adb reverse + app Orrery Remote), E2E (brief không cho chạy).

### Nhìn: không áp dụng

### Hot file cần thêm
1. `docs/remote-integration.md`: mục USB: listener `127.0.0.1:P` chỉ mở khi có slot `via:"usb"`, đóng ≤ 2s khi slot hết / huỷ / ghép xong / bị thay; link loopback chỉ được handshake ghép của slot usb (log `loopback`), `/install` qua loopback 403; cổng = cổng LAN đầu tiên hoặc `config.port`.
2. `.code_index` module remote: thêm `loopback.rs::supervise` / `target`, `pair.rs::{usb_until, usb_watch, qr_port, loopback_only}`, `lan.rs::accept_with`; `_meta.md` 1 dòng.
3. (Tuỳ chọn) `session.rs`: link `origin.pairing` trên `Peer::Lan` gửi msg1 không phải ghép → hiện log `pairing`, có thể đổi sang `loopback` cho rõ.

### Ngoài phạm vi
- Không có.

### Commit message đề xuất
```
feat(remote): loopback listener for USB pairing and typed wizard API wiring

- bind 127.0.0.1:<port> only while a via:"usb" pairing code is valid; close it within 2s when the code expires, is cancelled, replaced or used, or Remote stops
- loopback links may only run the pairing handshake of the usb code (refused with code loopback); /install over loopback stays 403
- retry the bind while the port is busy and follow port changes
- connect wizard uses remotePairStart, remoteCheck, onInstallFetched(since) and installArm wrappers and the ticket page path
- en/vi labels for the new access log codes
```
