# REVIEW v113 Rust: `apps/shell/src/remote` (8f4ecce3~1..HEAD)

Reviewer độc lập, chỉ đọc. Không build / test. Tổng: **0 CAO, 3 VỪA, 9 THẤP**.

---

## VỪA

### V1. Lệnh kick (Revoked / Held) có thể bị rơi vì `Probe` chiếm hàng đợi `ctl` 4 chỗ
- `apps/shell/src/remote/session.rs:81` (`mpsc::channel(4)`), `ctx.rs:233-236` (`kick` dùng `try_send`, bỏ kết quả), `ctx.rs:239-241` (`probe` cũng `try_send` vào cùng kênh), `check.rs:83-94`.
- Lỗi: trước v113 kênh `ctl` chỉ chở lệnh kết thúc phiên nên đầy cũng không sao. Giờ `Control::Probe` (không kết thúc phiên) và `Control::Held` dùng chung kênh 4 chỗ. `remote_check` không có rate limit cho probe. Kênh đầy thì `kick(Revoked)` / `kick(Held)` thất bại im lặng.
- Kịch bản: phone trên relay chậm, vòng `run` của phiên đang chờ `sec.json(...)` (outbox đầy). Chủ nhân bấm "Kiểm kết nối" 4 lần (mỗi lần 3s), rồi bấm Thu hồi. 4 `Probe` còn trong kênh, `Revoked` bị bỏ. Thiết bị đã bị xoá khỏi `devices.json` nhưng phiên LAN vẫn sống, vẫn điều khiển máy tới khi tự rớt.
- Sửa: tách probe ra kênh riêng hoặc `AtomicBool` + `Notify` trong `Live`; hoặc `probe` chỉ gửi khi `ctl.capacity() > 1`; với `Revoked` / `Held` dùng `watch` / cờ huỷ không thể rơi (vd `CancellationToken` trong `Live`).
- CHẮC (cơ chế `try_send` rơi khi đầy); kịch bản cần phiên đang nghẽn gửi.

### V2. Push `apk` chở nội dung chưa ghi vào ngoại lệ 8
- `push/offer_apk.rs:83-84` (`url`, `urls` có IP LAN + token tải, `size`, `sha256`, `app`, `variant`), `push_seal.rs:94`, `push_web.rs:63`, `push_extra.rs:31,49` (`apk` là info kind, bỏ qua `wants`).
- Lỗi: `docs/core-principle.md` ngoại lệ 8 (đoạn "since v113") chỉ liệt kê nhãn menu, số pane chờ, 2 nhãn pane, kind `running` / `done` / `open` / `clear`. Không có kind `apk` và không có việc push mang IP LAN + token tải + applicationId. `docs/remote-integration.md` cũng không nhắc (`rg apk` trong `docs/` = 0). CLAUDE.md: "do not add one without recording it there first".
- Kịch bản: thực tế dữ liệu chỉ đi trong phần sealed / Web Push mã hoá (`offer_targets` lọc `web || seal_key`, `message()` plain không chèn `apk`), nên không lộ ra Google. Nhưng vẫn là nội dung push ngoài phạm vi đã duyệt.
- Sửa: bổ sung câu vào ngoại lệ 8 (kind `apk`, chỉ đi sealed / Web Push, chở URL LAN có token 10 phút, size, sha256, app, variant đã mask cắt 80) + `docs/remote-integration.md` §7.6.
- CHẮC.

### V3. `fs.put` không tính vào quota ghi
- `viewer_map.rs:87` (`s("fs.put", ..., Arg::Put)` thiếu `.write()`), `viewer.rs:97`.
- Lỗi: `fs.write` / `checklist.toggle` tốn `take_write` (`WRITE_PER_MINUTE = 20`), `fs.put` chỉ tốn `take_rate` (120 / phút). Mỗi chunk `last: true` seq 0 tạo 1 file mới (≤ 384 KB sau base64) trong `.orrery/inbox`.
- Kịch bản: app phone lỗi lặp gửi ảnh (retry loop) → tới 120 file / phút, ~46 MB / phút vào repo, không chặn.
- Sửa: tính quota ghi khi `seq == 0` (hoặc khi `last`), hoặc thêm `.write()` cho spec và cho phép N chunk tiếp theo của cùng `up` không tốn quota.
- CHẮC.

---

## THẤP

### T1. Ghép lại thất bại vẫn huỷ lệnh thu hồi đang chờ
- `pair.rs:328` `ctx.hold.forget(&paired.device_id)` chạy trước `if !added`.
- Kịch bản: thiết bị D đang trong grace thu hồi 30s; một lần ghép mang `device_id` D được duyệt nhưng `devices.add` trả false (lỗi ghi) → hold bị xoá, thread grace gọi `due()` = false, D (key cũ) không bao giờ bị thu hồi.
- Sửa: chuyển `forget` vào trong nhánh `added`.
- CHẮC.

### T2. Race grace thu hồi: phiên vào đúng lúc `revoke_later` không bị kick Held
- `ctx.rs:200` (kiểm `is_held` trước khi khoá `live`), `revoke.rs:89-90` (`hold` rồi `kick`).
- Kịch bản: `admit_live` đọc `is_held` = false → `revoke_later` chèn hold + kick (phiên chưa có trong `live`) → `admit_live` thêm phiên. Phiên sống suốt grace (≤ 30s), tới `revoke()` mới bị kick. Thêm: `still_wanted` (`push.rs` ~236) không xét `hold`, thiết bị đang chờ thu hồi vẫn nhận push (kể cả `apk` có token).
- Sửa: kiểm `is_held` lại sau khi đã chèn vào `live` (dưới cùng lock) hoặc giữ lock `live` khi chèn hold; `still_wanted` loại thiết bị đang held.
- CHẮC (cửa sổ hẹp).

### T3. Luật nguy hiểm khớp trên text thô, web bỏ ký tự điều khiển sau đó
- `danger.rs:41-44`; web `packages/shared/src/pty.ts:95-98` (`cleanPanePrompt` bỏ C0/C1).
- Kịch bản: phone gửi `r\u0001m -rf /` → regex `\brm\s+-...` không khớp, desktop không đòi `authed`; web xoá `\u0001` → pane nhận `rm -rf /`. Spec F8 nói lớp này chỉ chống bấm nhầm, nên mức thấp.
- Sửa: chạy cùng phép làm sạch (bỏ C0/C1 trừ `\t\n`, bỏ `is_hidden`) trước `matching`.
- CHẮC (cơ chế).

### T4. TOCTOU symlink khi phục vụ file APK
- `install_http.rs:92` mở `path` lúc phục vụ; `install.rs:26-30` `is_apk` dùng `metadata` (đi theo symlink); `offer_apk.rs:52-58` chỉ canonicalize lúc cấp ticket.
- Kịch bản: sau khi cấp ticket offer, tiến trình trong project (build script / agent) thay `build/outputs/.../app.apk` bằng symlink tới file khác ≤ 1 GB → desktop gửi file đó qua HTTP thường trên LAN (người nghe lén Wi-Fi đọc được). Phone sẽ từ chối vì sai SHA nhưng byte đã đi.
- Sửa: lúc phục vụ dùng `symlink_metadata` từ chối symlink, hoặc canonicalize lại và so với path đã lưu.
- NGHI (cần tiến trình local ác ý, vốn đã có quyền rộng).

### T5. `/install/<t>/pair` thay slot ghép đang chạy
- `install_pair.rs:36` → `pair_ops.rs:16` (`start_with` ghi đè slot duy nhất).
- Kịch bản: chủ nhân đang ghép USB / QR; một máy LAN đã tải APK bằng ticket còn hạn bấm "Mở và ghép đôi" trong lúc đang arm → mã USB / QR bị huỷ. Cần ticket + arm nên chỉ là phiền.
- Sửa: từ chối `install_pair` khi `pairing.active()` có slot `via != Lan`.
- CHẮC (tác động thấp).

### T6. `run.log` có thể kẹt lỗi `limit` với log nhiều byte
- `run_log.rs:11-12` (`TEXT_MAX` 1000 ký tự × `READ_MAX` 300), `viewer.rs` `MAX_REPLY = 900_000`.
- Kịch bản: 300 dòng 1000 ký tự tiếng Việt / CJK (2-3 byte) hoặc JSON escape nhiều → reply > 900 KB → `fits` false → phone nhận `limit` cho đúng cửa sổ đó mãi, không đọc tiếp được.
- Sửa: cắt theo byte tích luỹ (dừng khi ~800 KB, trả `more: true`).
- NGHI.

### T7. `run_log::record_event` mask mọi dòng log kể cả khi không có phone
- `viewer_events.rs` `push_run` gọi `record_event` trước khi xét `subs`; `run_log.rs:74-79`.
- Kịch bản: logcat vài nghìn dòng / s → regex mask chạy trên thread phát event dù Remote tắt; RAM tối đa ~24 × 2000 × 4 KB ≈ 190 MB (ký tự 4 byte).
- Sửa: chỉ ghi khi Remote đang chạy; giới hạn tổng byte toàn store.
- CHẮC (hiệu năng).

### T8. RAM `fs.put` tối đa 256 MB
- `fs_put.rs:18-22` (`MAX_TOTAL_OPEN` 32 × `MAX_FILE` 8 MB), owner theo phiên (`remote:{device}:{nonce}`).
- Kịch bản: vài phiên (LAN + relay, nhiều thiết bị) mở 4 upload dở mỗi phiên → 256 MB giữ 60s. Phone đã ghép là bên tin cậy nên thấp.
- Sửa: giới hạn tổng byte (vd 32 MB) thay vì số buffer.
- CHẮC.

### T9. Probe trên LAN đọc nhầm ping của phone làm echo
- `dashboard_ping.rs:49-59`.
- Kịch bản: LAN (`alive = None`) đang có probe ping chờ; phone gửi ping keepalive của nó → desktop coi là echo, ghi RTT sai và không trả lời ping đó.
- Sửa: chỉ ghi RTT khi `answer_is_echo` (hoặc payload ping có số thứ tự).
- NGHI.

---

## Đã soát, không thấy lỗi

- Token ticket: `install_tickets.rs:75-87` so sánh hằng thời gian, duyệt hết slot; token 16 byte `getrandom`; `valid_token` giới hạn 64 ký tự base64url.
- Ticket dùng lại: `claim_pair` kiểm arm, kind `ApkPage`, `fetched` cùng IP, `used` trong cùng 1 lock; HEAD `/pair` → 405; ticket `ApkOffer` không vào được `/` hay `/pair`.
- IP: `norm` (IPv4-mapped IPv6) dùng ở `allowed_peer`, `note_fetch`, `claim_pair`, `PairMeta::lan` / `admits`; `allowed_peer` chỉ nhận IPv4 riêng nên listener loopback không phục vụ `/install`.
- HTTP: header ≤ 8 KiB trong 5s, `Location` percent-encode (không chèn header), CSP / nosniff / no-store đủ.
- Loopback USB: chỉ nhận ticket `Usb`, chỉ bắt tay ghép; listener đóng khi slot hết hạn / consume / cancel / stop (`loopback.rs:66-78`); ghép LAN không cấp ticket relay.
- `fs.put`: tên do desktop đặt, `create_new`, từ chối `..`, `.orrery` / `inbox` là symlink / junction (`is_symlink` trên Windows gồm junction), root qua scope `ccm:list-dir`, kiểm thứ tự / kích thước / TTL chunk, chạy trong `spawn_blocking`. Trả path tuyệt đối nhưng phone đã tự gửi root tuyệt đối nên không lộ thêm.
- `offer_apk`: không nhận path từ web, canonicalize + nằm dưới `cwd` + `build/outputs` + `is_apk`; app / variant qua `mask_text` cắt 80.
- `clip`: ≤ 64 KB, chặn khi mask đổi nội dung, desktop → phone chỉ khi bấm, chỉ qua phiên Noise, không đi push.
- Push extra: nhãn menu / pane qua `clean` + mask, cắt 24; `sig` chỉ chữ số; FCM plain không chèn `wl` / `menu` / `apk`.
- `danger`: `authed` bị gỡ trước khi chuyển lệnh; đường `confirm.answer` cho `ws.send` dùng lại cmd đã kiểm; regex có `size_limit`.
- `checklist.run`: cần `needsConfirm`, probe scope `ccm:checklist` trước `host.ask`. Viewer bỏ `_accept` do phone gửi.
- `revoke`: `due` theo serial, `undo` theo hạn, `flush_revokes` khi shutdown / restart, session + `admit_live` đều kiểm `is_held`.
- Không thấy `std::sync::Mutex` giữ qua `.await`, không thấy `unwrap` / index có thể panic trong code mới (`in_build` dùng `windows(2)`, `stamp` an toàn).
- Hằng `host.control()` của app thật luôn `true` là hành vi đã ghi trong `docs/remote-integration.md` (hello `control`), không tính là lỗi.
