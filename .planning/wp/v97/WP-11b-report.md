## WP-11b — DONE (còn CHƯA KIỂM: máy A21s / gateway shell thật)

### Đã đổi
- Claim: `SecureChannel.seal(kind, stream, payload, claim)` (Rust, bit 1 chỉ kind 2) -> `SecureLink.seal({claim})` -> `RemoteSession.sendPtyInput` -> `PtyStreams.input` (gộp 30 ms: claim nếu có chunk claim) -> `MachineApi.input`. `terminal_pane`: phím người dùng / dán / phím phụ claim; câu trả lời query của xterm (xảy ra trong `_write`) bị bỏ hẳn (desktop đã trả lời, tránh trả đúp). FRB regen (`lib/src/rust/**`, `frb_generated.rs`).
- Fake gateway (`rust/tests/common/{mod,commands}.rs`): `ws.patch {base,rev,ops}` (`ws.rename`), `ws.close` -> `{confirm}` + `confirm.ask {re}` + `confirm.answer` (patch / `cancelled` / `notFound`), `pty.attach` reply `{stream}`, stream 0 / trùng -> `invalid`, thứ 5 -> `limit`, `pty.exit {code:null}`, gỡ bit claim rồi echo `claim+echo:`.
- Font: `assets/fonts/NotoSansSymbols{,2}-Terminal.ttf` (subset OFL, 178 KB) + `LICENSES.txt`, pubspec fonts, `ui/terminal/terminal_fonts.dart` (fallback đầu danh sách).
- Lỗi thật tìm được khi chạy UI qua fake gateway / review: `protocol.dart` nuốt event có cả `re` và `t` (`confirm.ask` thật có `re` -> sheet xác nhận không bao giờ hiện); sheet confirm không hiện khi dialog còn đóng (retry + PopScope + hiện lại); `LinkChip` tràn trong AppBar; `MachineCache.save` đua nhau (rename lỗi trên Windows) -> xếp hàng; hello bị đóng 4401/4426 -> trước đây thành retry vô hạn, nay phân loại đúng; race attach khi đổi session; `_syncDesk` trong build; `TextPainter` không dispose; nút confirm bấm đúp / quá 60 s; `load` cache hỏng không văng.
- Tách file: `connection.dart` 460 -> 424 (+ `connect_attempt.dart`, `openRelay`, `helloFailure`, `storeRelayToken`), `ui_screens_test` 452 -> 276 (+ `ui_actions_test`, `support/ui_samples.dart`), `support/gateway.dart`, `support/fonts.dart`.
- Test mới: Rust 4 (claim, attach rule, confirm, huỷ + exit null), Dart: claim coalesce, claim từ bàn phím, event có `re`, cache save chồng, `ui_integration_test` (tag integration, UI thật qua `fake_gateway`: mở máy -> project -> pane -> attach -> gõ -> `claim+echo:ls` -> đóng pane -> confirm sheet -> patch).
- `.code_index/mobile.md` cập nhật.

### Tiêu chí: 6/6 đạt (một phần phụ CHƯA KIỂM)
1. `flutter analyze` sạch; `flutter test --concurrency=4` 137 pass (chạy 3 lần ổn định); `cargo test -j 3` (apps/mobile/rust) 14 pass.
2. Claim: test Rust (bit chỉ khi caller xin, chỉ kind 2) + Dart (phím người dùng true; xterm trả lời không gửi).
3. Integration UI qua fake gateway đi hết mở tab -> attach -> gõ -> echo -> confirm.
4. Ảnh `docs/qa/v97/WP-11b/` (19 file, headless): ⏺ ✓ ✗ ✔ ⎿ ▸ … hiện đúng, không ô vuông.
5. Review flutter-reviewer: 0 chặn; 3 cao + 6 vừa đã sửa; chưa sửa (thấp): `_Notice`/`OfflineBanner` live region + màu hard-code, `minProto` lấy từ client, double attach khi `readySessions` trùng lúc attach, input rơi im lặng khi link chưa ready.
6. Không comment; file ≤ ~400 dòng trừ `connection.dart` 424 (hơi vượt), `connection_test.dart` 569, `fakes.dart` 449 (có sẵn); `.code_index` cập nhật.

### Audit
Chỉ sửa `apps/mobile/**`, `docs/qa/v97/WP-11b/**`, brief / report. Không đụng `apps/shell`, `apps/core`. Font tải từ github.com/google/fonts (Noto, OFL 1.1), cắt subset bằng fontTools.

### Review
flutter-reviewer (sonnet) trên `lib/ui/**` + `lib/remote/**`: xem mục tiêu chí 5.

### Test
`cd apps/mobile && flutter analyze` sạch; `flutter test --concurrency=4` 137 pass; `cd rust && cargo test -j 3` 14 pass.

### Nhìn
Đã mở `05-pane-asked.png`, `05b-pane-working-dark.png`, `10-confirm.png` (sau lần sửa cuối): không ô vuông, không tràn. Trước: `docs/qa/v97/WP-11/05-pane-asked.png` (ô vuông ở ✔). Các ảnh còn lại chỉ chụp, chưa mở từng cái.

### CHƯA KIỂM
- Chạy với gateway shell thật (WP-6) và máy A21s (không dùng; headless như WP-11). Claim chỉ kiểm với fake gateway mô phỏng gỡ bit.
- Font trên máy Android thật (mới thấy ở test headless với font Consolas + Roboto).
- Kiểm `FLAG_CLAIM` có nên nằm trong crate `core-remote-proto` (hiện đặt bit trong FRB của app).

### Ngoài phạm vi
Chip "Desktop đang gõ" (gateway chưa có event); 3 mục thấp ở mục 5.

### Commit message đề xuất
feat(mobile): send the claim flag only for user typed terminal input, bundle terminal symbol fonts, make the fake gateway match the real contract with a UI integration test; fix confirm.ask events dropped as replies, confirm sheet timing, hello close codes, cache save races
