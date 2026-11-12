# WP-24 RESULT: service account Firebase vượt giới hạn Credential Manager + QA push trên A21s

Ngày 2026-11-12. Dev Tauri riêng (collector 4431, remote 4341, ctrl 4566, CDP 9355, data dir tạm), A21s 192.168.1.217:34351.

| Mục | Kết quả | Ảnh |
|---|---|---|
| `cargo test --locked -j 3 remote` | ĐẠT, 139 passed | |
| `remotePushSetup` với 2 file thật | ĐẠT `{ok:true, projectId:"orrery-remote"}`; file `fcm-sa.sealed` 2.4 KB, bắt đầu `o1:`, không chứa `PRIVATE` / `BEGIN` | |
| Ghép lại điện thoại (mã nhập tay, debug) | ĐẠT, LAN | 01-paired-switch-off.png |
| Bật switch Thông báo, token đăng ký | ĐẠT (`tokens:1`) | 02-switch-on.png |
| `remotePushTest` khi app ở Home | ĐẠT `{ok:true,sent:1}`; thanh thông báo "Orrery: Thông báo thử từ máy tính. Push đã hoạt động." | 03-push-test-shade.png |
| Chạm thông báo mở app | ĐẠT (foreground `dev.ccm.orrery.remote`) | 04-tap-opens-app.png |
| Push `done` | CHƯA KIỂM: `done` chỉ bắn khi terminal của pane Workspace đổi `working → idle`; pane shell trong dev app không có trạng thái này, cần pane Claude / Codex (gọi LLM thật, ngoài phạm vi). Đường gửi FCM đã chứng minh bằng push thử; chuyển trạng thái có test Rust (`push_watch_tests`). | |
| `remotePushRemove` | ĐẠT: `configured:false`, file `fcm-sa.sealed` bị xoá | |
| Thu hồi (`remoteRevokeAll`) | ĐẠT: điện thoại hiện "Đã thu hồi", "Chưa thiết lập trên máy tính" | 05-revoked-push-removed.png |

## Sửa
- `apps/shell/src/remote/fcm_files.rs`: `save_account` / `load_account` / `remove_account`. Niêm phong AES-GCM (`o1:`) vào `<remote dir>/fcm-sa.sealed` (ghi tmp + rename, 0600 trên unix); master 32 byte trong keychain `orrery-fcm-master` (sinh khi chưa có, đọc lại kiểm khớp). Keychain hỏng → `keyring`.
- `apps/shell/src/remote/push_setup.rs`: setup / đọc / gỡ dùng file niêm phong; entry cũ `orrery-fcm-sa` vẫn đọc được (migrate) và bị xoá khi cài mới / gỡ.
- Lệch brief: không gọi `vault::Vault` trực tiếp vì `Remote` chỉ giữ `KeyStore` (Vault nằm ở Tauri state, không đụng `mod.rs` / `host.rs`); dùng cùng định dạng `o1:` + AES-256-GCM, master nằm trong cùng keyring service qua `KeyStore`.
- Token Cloudflare `orrery-cf-token` ~40 ký tự, dưới max 2560 byte: giữ, ghi chú vào `.code_index/shell.md`.
- Test mới (`push_setup_tests.rs`): file niêm phong không chứa private key + không còn mục `orrery-fcm-sa`; account > 4 KB lưu / đọc lại; keychain hỏng → `keyring` và không có file; mục cũ không còn sau cài lại; gỡ xoá file; restart đọc lại.
- Docs: `docs/remote-integration.md` (1 câu), `.code_index/shell.md`.

## Dọn
Dev app tắt (cổng 4431 / 4341 / 9355 đóng); điện thoại về Home, `show_touches` = 0, quyền camera chỉ cấp "Chỉ lần này"; app Remote còn cài, máy dev ở trạng thái "Đã thu hồi". Ảnh camera chụp lúc mở màn ghép đã xoá, không lưu. Không đụng Orrery thật.
