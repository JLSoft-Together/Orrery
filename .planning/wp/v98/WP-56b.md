## WP-56b: FCM có nội dung cho Android — mã hoá bằng khoá thiết bị, data message, app giải mã rồi tự hiện; lỗi thì hiện bản cố định

- Mục tiêu: Android nhận thông báo có nội dung như web app (WP-56a) mà Google chỉ thấy gói mã hoá; app đang chạy nền / bị kill vẫn hiện được; giải mã lỗi → bản không nội dung.
- Spec: `.planning/PLAN-v98-ios-pwa.md` #16, #17, #18, #20, rủi ro "Samsung chặn khi app bị kill → giữ fallback cố định". Hợp đồng từ WP-56a: `.planning/wp/v98/WP-56a-report.md` mục "Hợp đồng cho WP-56b" (`Ctx::notice(&Job)` cho title / body; `Job.content`, `terminal`, `worked`; `push.register` đã lưu `content` cho FCM; thay `push::message(job)` bằng data message mã hoá, chỉ chuyển sang `notice()` khi payload đã mã hoá; giới hạn, mute, `push_off`, `still_wanted`, xoá token 404 / UNREGISTERED giữ nguyên). Docs: `docs/remote-integration.md` §7.2, §7.4, §7.6; `CLAUDE.md` ngoại lệ thứ 7 (câu "FCM to Android stays fixed text until … WP-56b" phải sửa cho đúng sau gói này).
- Code: desktop `apps/shell/src/remote/{push,fcm,push_content,push_web,devices}.rs` + test; Android `apps/mobile/lib/push/**` (hoặc nơi `PushService` / `FirebasePushGateway` nằm — tìm theo `firebase_messaging`), `apps/mobile/lib/remote/{keys,crypto,crypto_rust}.dart` (khoá tĩnh thiết bị `PhoneIdentity.privateKey`, `SecureKv` khoá `remote.static_key`), `apps/mobile/android/app/src/main/**` (channel `attention`, màu cam, `MainActivity`), `apps/mobile/rust/**` (crate FRB), `apps/mobile/.code_index/mobile.md`.
- Phạm vi file được đụng: những chỗ trên + test; `CLAUDE.md` (chỉ câu FCM trong ngoại lệ thứ 7); docs §7.2 / §7.4 / §7.6 + `docs/orrery-remote.md` mục thông báo; `.code_index/`; `docs/qa/v98/WP-56b/**`; báo cáo. Cấm đụng: `apps/pwa/**`, `apps/relay/**`, `apps/web/**` (trừ khi cần nhãn UI mới — ghi lại), `scripts/spike-v98/**`.
- Model: opus. Chạy một mình (không song song) — review opus độc lập khi về.

### Việc (thiết kế do bạn chốt, ghi docs + báo cáo)
1. Sơ đồ mã hoá: desktop đã giữ khoá tĩnh X25519 (Noise) của từng thiết bị trong `devices.json`. Dùng kiểu sealed box: khoá tạm X25519 mỗi tin, ECDH với khoá tĩnh thiết bị, HKDF-SHA256 (salt / info có domain `orrery-push/1` + cả 2 khoá công khai + machineId), AEAD ChaCha20-Poly1305 (có sẵn trong cây phụ thuộc) với AAD gồm machineId + kind; nonce duy nhất. Kiểm lại có cần tách khoá khác Noise không (dùng chung khoá tĩnh cho 2 mục đích: phân tích ngắn, chọn, ghi lý do). Test vector cố định (khoá tạm cố định) dùng chung Rust ↔ Dart.
2. Payload FCM: data-only message, `android.priority: high`, `data` = `{v, m, e}` (`e` = base64url(khoá tạm ‖ ciphertext)), không có khối `notification` khi gửi nội dung; ≤ 4 KB. Thiết bị tắt "Kèm nội dung" / thiếu khoá / proto cũ → giữ đúng message cố định hiện tại (`push::message`). Chọn và ghi rõ cách chống "app bị kill trên Samsung không chạy handler": vd. vẫn gửi bản cố định nếu thiết bị chưa từng báo đã giải mã thành công; hoặc phone báo `push.ack` capability lúc `push.register`. Đừng làm mất thông báo.
3. Android: `FirebaseMessaging.onBackgroundMessage` (top-level, `@pragma('vm:entry-point')`) + foreground handler: giải mã → hiện thông báo local (channel `attention`, màu cam, tag theo pane để gộp, chạm mở deep link `orrery-remote://m/<m>/p/<p>` như hiện có); giải mã lỗi → hiện bản cố định theo kind; app đang mở + phiên máy đó đang mở → không hiện (#20). Giải mã: ưu tiên Rust qua FRB nếu thêm được API (máy này KHÔNG có `flutter_rust_bridge_codegen` — được phép `cargo install flutter_rust_bridge_codegen --version <khớp pubspec 2.13.0> -j 4` nếu cần, ghi lại); không thì Dart thuần (package có sẵn hoặc `cryptography`), ghi lý do. Khoá riêng đọc từ `SecureKv` trong isolate nền — kiểm `flutter_secure_storage` chạy được trong background isolate.
4. Quyền / hiển thị: dùng plugin thông báo local nếu chưa có (`flutter_local_notifications`) — kiểm Android 13+ `POST_NOTIFICATIONS` đã xin; màn khoá hiện đủ nội dung (#16, `visibility: public`).
5. Sửa `CLAUDE.md` câu FCM của ngoại lệ thứ 7 cho đúng thực tế mới; docs §7.2 (data mới), §7.4, §7.6.
6. Test: Rust (mã hoá / vector / kích thước ≤ 4 KB / fallback khi content tắt, thiếu khoá, thiết bị cũ / mute / `still_wanted`), Dart qua `node apps/mobile/scripts/check.mjs` (giải mã vector, giải mã lỗi → bản cố định, chọn hiện / không hiện khi app mở, deep link), shell `cargo test --bin orrery-shell remote::`, mobile Rust `cargo test` nếu đụng. Không có điện thoại trên máy này: ghi rõ phần phải thử trên A21s (app nền, app bị kill, màn khoá, Samsung).

### Tiêu chí nghiệm thu
- [ ] Vector Rust ↔ Dart khớp; shell remote pass (số); `check.mjs` analyze sạch + test pass (số, so 278); clippy file đụng không warning mới.
- [ ] `CLAUDE.md` + docs đúng thực tế; không key / token / nội dung trong log.
- [ ] Danh sách bước QA máy thật cho chủ nhân (A21s).

### Luật (bắt buộc)
- Không viết comment trong code (kể cả `///`, `/** */`, `//`, `#`, TODO); lý do ghi báo cáo hoặc `.code_index/`.
- Không `git commit` / `push`. File mới ≤ ~400 dòng. Không build APK / release. Mobile chỉ kiểm qua `check.mjs`. Đừng chạy `apps/mobile/tool/gen_arb.mjs` (đã cũ, xoá khoá ARB) — sửa ARB tay.
- Tool nặng ≤ 80% CPU / RAM (`cargo -j 4`). Không dùng cổng 4320–4322.
- Bash: `rtk` có thể rewrite `grep` → dùng tool Grep hoặc `rtk proxy grep`.
- Tắt mọi tiến trình nền khi xong.

### Báo cáo
`.planning/wp/v98/WP-56b-report.md`: sơ đồ mã hoá + lý do, cách chống mất thông báo, file + symbol, test chạy thật (số, lệnh), bước QA A21s, rủi ro (để review độc lập).
