## WP-2: crate `core-remote-proto` (Noise + frame + ghép)
- Mục tiêu: 1 crate Rust thuần (không tokio, không IO) dùng chung cho gateway desktop và app điện thoại (qua FRB), làm: bắt tay ghép, bắt tay phiên, mã hoá / giải mã, đóng gói frame, chia / ghép khúc, token relay. Có test đầy đủ.
- Spec item: PLAN-v97-mobile.md §1.1 T2, T3, T5, T11 (phần token), bất biến 4 + 8; review #4, #6, #7, #13, #16.
- Phạm vi file: tạo `apps/core/crates/core-remote-proto/**`; được sửa `apps/core/Cargo.toml` (thêm member + dep workspace) và `apps/core/Cargo.lock` (WP này là chủ 2 file đó trong lượt 1). | Cấm đụng: `apps/core/crates/core-pty/**` (WP-3 đang sửa), mọi crate khác, `apps/shell/**`, `apps/web/**`, `apps/mobile/**`.
- Phụ thuộc: — | Song song với: WP-1, WP-3 | Worktree: không.
- Model: opus (crypto, bảo mật).

### Thiết kế (bắt buộc theo)
- Dep: `snow` 0.10 với resolver thuần Rust (feature mặc định `default-resolver`; **không** bật `ring-resolver` để build Android không cần C), `serde`/`serde_json` (workspace), `hmac` + `sha2` (đã có trong lock), `getrandom`, `zeroize`, `hkdf` nếu cần (thêm nhỏ). Không thêm crate lớn khác; mỗi crate mới ghi lý do vào báo cáo.
- Hằng: `PROTO: u16 = 1`, `MIN_PROTO: u16 = 1`, prologue = `b"orrery-remote/" + PROTO (2 byte BE)`.
- Ghép (`pair` module): `Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s`. Mã ghép = 16 byte ngẫu nhiên (≥ 128 bit) trong QR; PSK = HKDF-SHA256(ikm = mã, salt = `"orrery-pair"`, info = machineId) 32 byte. QR payload (struct + encode/decode base64url JSON, max ~300 B): `{v:1, machine: machineId, name, host?, port?, relay?, mkey: desktop static pub 32 B, code: 16 B, exp: unix s}`. Phía điện thoại (initiator): sau khi đọc msg 2 phải so `remote_static` với `mkey` trong QR, khác → lỗi `KeyMismatch` và KHÔNG tạo msg 3. Sau msg 3: desktop có key tĩnh điện thoại + `deviceId` + tên (payload msg 3), điện thoại có xác nhận desktop.
- Phiên (`session` module): `Noise_IK_25519_ChaChaPoly_BLAKE2s`, điện thoại initiator biết key desktop; payload msg 1 = `{deviceId, proto}`; desktop (responder) sau msg 1 lấy `remote_static` + `deviceId`, gọi callback `authorize(deviceId, remote_static) -> bool` (so `devices.json` ở gateway), sai → `Unauthorized`. Lệch proto < MIN_PROTO → `ProtoTooOld {min}`.
- Transport: `Transport` bọc `snow::TransportState`, `seal(plaintext) -> Vec<u8>`, `open(ciphertext) -> Vec<u8>`; nonce tăng dần (snow tự quản) ⇒ frame lặp / đảo thứ tự / sửa 1 byte phải lỗi.
- Frame trong Noise (`frame` module): `[u8 kind][u8 flags][u32 stream BE][payload]`; kind: 0 json, 1 pty_out, 2 pty_in, 3 ping (kind lạ → lỗi `UnknownKind`, không panic); flags bit0 = `more`. `Chunker`: chia payload thành khúc sao cho mỗi Noise message ≤ 65 535 B (payload khúc ≤ 60 KiB); `Reassembler` ghép theo `(kind, stream)` tới max 1 MiB (vượt → lỗi, bỏ dở), khúc không `more` = xong.
- Ngoài Noise, đường relay (`relay` module): envelope `[u16 connId BE][bytes]`; control từ DO dạng JSON nhỏ `{t:"open"|"close", conn}` (kind riêng: connId = 0 dành cho control). Token điện thoại: `token = base64url(deviceId ‖ exp(u64 BE) ‖ HMAC-SHA256(relayKey, "orrery-relay/1" ‖ machineId ‖ deviceId ‖ exp))`, hàm `issue` / `verify(now)` (so sánh constant-time). `relayKey` 32 byte do máy sinh, dùng chung với DO lúc đăng ký (WP-9 lo đăng ký).
- Key: `StaticKeypair::generate()`, export / import 32 B (zeroize khi drop). Không log key / payload.
- API không panic trên input xấu (mọi hàm parse trả `Result`), `#![forbid(unsafe_code)]`.

### Tiêu chí nghiệm thu
- [ ] Crate build + `cargo test -p core-remote-proto` pass; `cargo clippy -p core-remote-proto --all-targets -- -D warnings` sạch.
- [ ] Test ghép đủ vòng (điện thoại ↔ desktop) ra cặp key + deviceId đúng.
- [ ] Test desktop giả (key khác `mkey`) → điện thoại dừng ở msg 2 (`KeyMismatch`), không gửi msg 3.
- [ ] Test sai mã ghép → bắt tay lỗi.
- [ ] Test IK: thiết bị đã ghép vào được; thiết bị lạ (`authorize` false) bị từ chối; proto cũ bị từ chối `ProtoTooOld`.
- [ ] Test transport: frame lặp lại, đảo thứ tự, sửa 1 byte → lỗi.
- [ ] Test frame / chunk: JSON 1 MiB chia + ghép đúng; > 1 MiB lỗi; kind lạ lỗi; input ngẫu nhiên (proptest hoặc vòng lặp random 10 000 lần) không panic.
- [ ] Test token: hợp lệ qua, hết hạn / sửa byte / sai machineId bị từ chối.
- [ ] Vector cố định (key + mã cố định → bytes msg 1 của IK và ghép) lưu `tests/vectors.json` để WP-10 (Dart qua FRB) kiểm chéo.
- [ ] Không có comment trong code; lý do thiết kế ghi vào `.code_index/core.md` mục mới `core-remote-proto` (≤ 25 dòng).

### Lệnh test (giới hạn tài nguyên: máy đang chạy song song WP-1 Flutter + WP-3 cargo)
`cd apps/core && cargo test --locked -j 3 -p core-remote-proto && cargo clippy --locked -j 3 -p core-remote-proto --all-targets -- -D warnings`
(Lần đầu thêm dep thì bỏ `--locked` một lần để cập nhật lock.)

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-2-report.md`

### Nhật ký
- 2026-11-12 WP-2: tạo crate core-remote-proto (pair / session / transport / frame / relay), thêm member + dep, sinh tests/vectors.json, viết mục core-remote-proto trong .code_index/core.md
