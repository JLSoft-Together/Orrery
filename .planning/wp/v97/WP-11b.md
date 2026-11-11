## WP-11b: Mobile — claim thật, fake gateway khớp hợp đồng, font ký hiệu terminal, flutter-reviewer
- Mục tiêu: nối nốt các chỗ WP-11 để lại để app chạy đúng với gateway thật: cờ `claim` khi gõ, fake gateway khớp hợp đồng mới để chạy UI đầu-cuối, terminal hiện được ký hiệu Claude / Codex dùng (⏺ ✓ ✗ ● ⎿ ▸ … đang ra ô vuông trong ảnh `docs/qa/v97/WP-11/05-pane-asked.png`), sửa các lỗi review Flutter.
- Spec item: `.planning/wp/v97/WP-11-report.md` mục "Ngoài phạm vi" + CHƯA KIỂM; `docs/remote-integration.md` §3 (flags bit 1 = claim), §4.3, §4.4, §4.6.
- Phạm vi file: `apps/mobile/**` (cả `apps/mobile/rust/**`), `docs/qa/v97/WP-11b/**`. | Cấm đụng: ngoài `apps/mobile`.
- Phụ thuộc: WP-6 (gateway đã có claim strip, ws.patch base, confirm, attach reply), WP-11. | Song song với: review WP-6 | Worktree: không.
- Model: sonnet.

### Việc
1. Claim: FRB Rust `seal` (hoặc hàm đóng gói frame PTY) nhận `claim: bool`, đặt bit `0b10` vào byte flags của frame kind 2 sau `Frame::encode`, trước `seal`; nối qua `SecureLink`, `sendPtyInput`, `PtyStreams.input`, `MachineApi.input`; chỉ bật khi người dùng thật gõ / Gửi / phím phụ (`_TerminalPaneState._onOutput` từ bàn phím), không bật khi xterm tự trả lời query. Test Rust + Dart.
2. `examples/fake_gateway.rs`: `ws.patch {base, rev, ops}`, lệnh cần xác nhận trả `{confirm: cid}` + event `confirm.ask {cid, re, …}` + `{cancelled}`, `pty.attach` reply `{stream}`, `pty.exit {code:null}`, attach thứ 5 → `limit`, gỡ bit claim. Thêm 1 test integration chạy UI qua fake gateway (mở tab → attach → gõ → echo → confirm close).
3. Font terminal: thêm font fallback có các ký hiệu (bundle 1 font mã nguồn mở nhỏ, vd JetBrains Mono / Cascadia hoặc "Symbols Nerd Font" bản rút gọn chỉ phần cần; ghi licence vào `apps/mobile/assets/fonts/LICENSES.txt`), set `fontFamilyFallback` cho `TerminalView`; ảnh trước / sau.
4. Chạy agent `flutter-reviewer` (sonnet) trên `apps/mobile/lib/ui/**` + `lib/remote/**`, sửa mọi lỗi chặn / cao / vừa; tách `test/ui_screens_test.dart` (452 dòng) và `lib/remote/connection.dart` (461 dòng) xuống ≤ ~400 dòng.

### Tiêu chí nghiệm thu
- [ ] `flutter analyze` sạch; `flutter test --concurrency=4` pass; `cargo test -j 3` (apps/mobile/rust) pass.
- [ ] Test: frame kind 2 từ bàn phím có bit claim, từ query trả lời thì không.
- [ ] Test integration qua fake gateway đi hết mở tab → attach → gõ → echo → confirm.
- [ ] Ảnh `docs/qa/v97/WP-11b/` (headless như WP-11 hoặc máy A21s nếu foreground là launcher / app mình): ký hiệu ⏺ ✓ hiện đúng, không ô vuông.
- [ ] Review flutter-reviewer: không còn chặn / cao / vừa; report ghi lại.
- [ ] Không comment trong code; file ≤ ~400 dòng; `.code_index/mobile.md` cập nhật.

### Lệnh test
`cd apps/mobile && flutter analyze && flutter test --concurrency=4` và `cd apps/mobile/rust && cargo test -j 3`

- Nhìn thấy được: có.
- Báo cáo: `.planning/wp/v97/WP-11b-report.md`

### Nhật ký
- WP-11b: claim bit (Rust seal(claim) + Dart + terminal_pane) + fake gateway lệnh mới + 14 rust test + 133 flutter test pass
- WP-11b: fake gateway + test ui_integration (mở tab, attach, gõ, claim echo, confirm close) pass; sửa LinkChip overflow, protocol: event có re không bị nuốt, MachineScope retry sheet confirm
- WP-11b: font ký hiệu (Noto Symbols 2 + Symbols subset OFL, 178 KB) + terminal_fonts.dart; ảnh docs/qa/v97/WP-11b/05-pane-asked.png không còn ô vuông
- WP-11b: tách connection.dart (414 dòng, thêm connect_attempt.dart, openRelay, helloFailure, storeRelayToken) + ui_screens_test (276) / ui_actions_test (168) / support/ui_samples.dart
- WP-11b: cache.dart xếp hàng save (race rename Windows), code_index cập nhật; chờ flutter-reviewer
- WP-11b: flutter-reviewer xong (3 cao + 6 vừa đã sửa), flutter test 137 pass, analyze sạch
