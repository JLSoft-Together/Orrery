## WP-113l — DONE

### Đã đổi
Desktop (`apps/shell/src/remote/`):
- `danger.rs` (mới): 6 luật mặc định, `valid_rules` (max 32 luật, mỗi luật ≤ 200 ký tự, regex lỗi / trùng bị bỏ, size_limit regex), `matching` (so cả dòng trim và từng dòng, không phân biệt hoa thường), `needs_auth`, `strip_authed`.
- `danger_tests.rs` (mới), đăng ký trong `mod.rs` (`mod danger;` + `mod danger_tests;`).
- `config.rs`: field `danger_rules: Vec<String>` serde default = luật mặc định.
- `ctx.rs`: 1 dòng trong `welcome()`: `out["dangerRules"] = danger::rules_json(&cfg.danger_rules)` (vùng hàm welcome, không đụng trait Host).
- `phone_ws.rs`: `ws.send` khớp luật mà `authed != true` thì trả `{re, ok:false, error:"needsAuth", rule, message, detail}` và KHÔNG gọi host; có authed thì bỏ field `authed` rồi chuyển xuống renderer.
Phone (`apps/mobile/`):
- `lib/security/danger_rules.dart` (mới): `DangerRules` (parse cùng giới hạn như desktop), `guardDanger`, `authPassed`.
- `lib/ui/danger_guard.dart` (mới): `dangerRulesProvider` (đọc `hello['dangerRules']` từ store, không sửa store), `confirmDanger`, `authorizeNow` (dùng `AppLock.confirm()`).
- `chat_composer.dart`: khớp -> xác thực; huỷ giữ nguyên chữ; `onSend(text, authed)`; chip cũng kiểm.
- `chat_controller.dart`: `send(raw, {authed})`, gửi kèm `authed:true`; nhận `needsAuth` -> `reauthorize` đúng 1 lần -> gửi lại (resend reset cờ).
- `input_bar.dart` (chế độ gửi dòng vào PTY): khớp -> xác thực trước `paste` + Enter.
- `chat_screen.dart`, `terminal_pane.dart`: nối `danger` / `reauthorize` (vài dòng).
- `commands.dart`: `needsAuth` -> `errNeedsAuth`; ARB en + vi (`errNeedsAuth`), sinh lại `app_localizations*.dart` bằng `flutter gen-l10n`.
- `test/danger_rules_test.dart` (mới).

### Tiêu chí
- [x] cargo: khớp / không khớp từng luật, force-with-lease không khớp, regex lỗi bị bỏ, `ws.send` thiếu authed bị chặn và không gõ (host không nhận ask), có authed thì gõ.
- [x] Dart: composer gọi auth khi khớp, huỷ giữ chữ, needsAuth -> auth -> gửi lại 1 lần (fake), không vòng lặp.
- [x] Test cũ remote + mobile xanh; không comment; file ≤ 400 dòng; en+vi.

### Audit
Diff chỉ gồm các file trên; `install*.rs`, `host.rs`, `test_util.rs`, `viewer*.rs`, `core-desk`, `push/`, `app/` không đụng.

### Review
- Regex lỗi / quá dài / trùng / quá 32: bỏ ở cả desktop và phone (có test).
- Mặc định `git branch -d` cũng khớp (so không phân biệt hoa thường, Dart không hỗ trợ `(?-i:)`); chấp nhận.
- App cũ: gửi lệnh khớp -> desktop trả `needsAuth` + `detail`; app cũ hiện lỗi chung (chấp nhận theo brief).
- Huỷ xác thực / `failed`: không gửi, giữ chữ. `notAvailable` (máy không có khoá màn hình) và không có `AppLockScope`: cho gửi kèm `authed:true` (không có cách xác thực, tránh kẹt vĩnh viễn).
- needsAuth vòng lặp: `authRetried` chặn, chỉ hỏi 1 lần mỗi lượt gửi; bấm Gửi lại của user được hỏi lại.
- Luồng confirm (`needsConfirm`) không bị ảnh hưởng: lệnh đã qua kiểm khi vào, `authed` đã bị bỏ trước khi xuống renderer.
- Ghi access log: các từ chối trong `phone_ws.rs` hiện không có access log (Phone không giữ `Link`), nên chỉ trả lỗi, không ghi thêm.
- Locale message desktop: theo locale desktop (`welcome().locale`), không phải locale phone (hello không có locale).
- `authed` do phone tự khai: chỉ chống bấm nhầm / app cũ, không phải lớp bảo mật (đúng spec).

### Test
- `cd apps/shell; cargo test --locked -j 4 danger` -> 4 passed.
- `cargo test --locked -j 4 remote` -> 280 passed, 0 failed, 3 ignored (lần đầu lỗi compile ở `install.rs` của WP-113b, chờ rồi chạy lại là hết).
- `flutter analyze` -> No issues found.
- `flutter test --concurrency=2 test/danger_rules_test.dart test/chat_ui_test.dart test/chat_ui_model_test.dart test/arb_no_dots_test.dart test/app_lock_test.dart` -> 46 passed.
- `npm run typecheck`: không chạy (không sửa `desktop.ts`).

### Nhìn: CHƯA NHÌN
Hộp xác thực vân tay trên phone (A33 / emulator) và chữ giữ lại trong ô sau khi huỷ.

### Hot file cần thêm
- Docs (main): phím gõ thô PTY (`direct` mode) và `ws.keys` không kiểm; Telegram không áp; ghi `welcome.dangerRules`, `ws.send.authed`, lỗi `needsAuth` vào `docs/remote-integration.md`.
- `.code_index/` + `apps/mobile/.code_index/mobile.md`: `danger.rs`, `danger_rules.dart`, `danger_guard.dart`.
- Web i18n / Settings (WP Settings sau): UI sửa `danger_rules`, ghi rõ Telegram không áp.

### Ngoài phạm vi
- Kênh `ccm:remote-danger` get/set + type `desktop.ts`: chưa đăng ký (cần sửa `ccm.rs`, ngoài phạm vi); để WP Settings.
- Sửa ngoài danh sách trong phạm vi nhưng cần thiết để nối: `chat_screen.dart`, `terminal_pane.dart`, `ui/commands.dart`, `app_localizations*.dart` (sinh tự động).

### Commit message đề xuất
feat(remote): require device authentication on the phone for dangerous commands, and recheck on the desktop
