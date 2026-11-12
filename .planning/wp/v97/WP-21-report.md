## WP-21 — DONE

### Đã đổi
- Mới: `apps/web/src/components/settings/remote/RemoteRelaySetup.vue` (191 dòng), `RemotePushSetup.vue` (148); `composables/useRemoteSetupIpc.ts` (wrapper + kiểm kiểu kết quả IPC, `hasRelaySetup` / `hasPushSetup`), `useRemoteRelay.ts`, `useRemotePush.ts`, `useRemotePushFiles.ts` (`classifyPushFile`); `i18n/remote-setup-v97.ts` (vi + en, tách khỏi `remote-v97.ts` cho < 400 dòng); test `useRemoteRelay.test.ts`, `useRemotePush.test.ts`, `useRemotePushFiles.test.ts`, `remote-setup-v97.test.ts`; `docs/qa/v97/WP-21/` (30 ảnh + `shots.cjs`).
- Sửa: `SettingsRemote.vue` (2 khối mới, ẩn khi bridge thiếu kênh), `i18n/remote-v97.ts` (ghép `setup.en/vi`), `settings/index.ts` (+2 mục tìm kiếm `remoteRelay`, `remotePush`), `docs/orrery-remote.md` §13, `.code_index/web.md` + `_meta.md`.
- Không sửa shell / preload / shared / store (store `remote.ts` không đụng: trạng thái relay / push nằm trong composable, token không vào store).
- Kéo thả: Tauri WebView này đưa `File` của HTML5 drop như TermPane / DeviceFilesDialog đang dùng, nên đọc bằng `file.text()` ở trang, KHÔNG cần IPC đọc file mới. Chọn file bằng `<input type=file>`. Giới hạn 64 KB ở trang; nhận diện theo nội dung (`type: service_account` / `project_info.project_id`).
- Seam test: `useRemoteSetupIpc.ts` đọc `window.__ccmE2eSetupBridge` nếu có (vì `ccmDesktop` và `__TAURI_INTERNALS__` bị đóng băng, không mock từ ngoài được). Mặc định không có thì dùng `window.ccmDesktop`.

### Tiêu chí 3/3
- [x] `npm run typecheck -w @ccm/web` sạch; Vitest 72 pass (11 file: composables `useRemote*`, i18n, settings): nhận diện file (đúng / sai loại / JSON hỏng / quá 64 KB / 1 file / 3 file / lệch project), lỗi theo mã, tiến độ deploy, rời trang giữa lúc deploy rồi mở lại, huỷ confirm xoá, token xoá khỏi trang ngay sau khi gửi + khi rời trang, nội dung file không vào web storage.
- [x] 30 ảnh light + dark (`relay-1..6`, `push-1..6`, mỗi cái 2 theme), đã tự mở xem từng nhóm: chưa dựng, token OK + chọn account, đang dựng (bước 3/5), lỗi `token_scope`, sẵn sàng (cảnh báo health + cập nhật), hỏi gỡ, push chưa cài / 1 file / lệch project / đủ 2 file / lỗi `no_android_client` / đã cài.
- [x] vi + en (test đủ mã lỗi + bước), không comment, file ≤ 400 (lớn nhất 281 i18n, 191 vue), docs + index cập nhật.

### Audit
Diff chỉ trong `apps/web/src/{components/settings,composables,i18n}`, `docs/`, `.code_index/`, `.planning/wp/v97/WP-21*`. Không đụng `apps/shell`, `apps/mobile`, `apps/relay`, preload, shared.

### Review (edge case)
- File thả sai loại / JSON hỏng / quá lớn: báo mã `unknown_file` / `bad_json` / `too_big`, file tốt cùng lần thả vẫn nhận. 1 file: ô còn lại "Thiếu", nút Cài thông báo đẩy tắt. 3 file: cùng loại thì file sau thay file trước. Lệch project: chặn ở trang (`mismatch_project`), nút tắt.
- Token sai / thiếu quyền: hiện câu theo mã (`token_invalid`, `token_scope`...), ô token đã xoá. Token không bao giờ vào store, status, ảnh chụp; xoá ngay sau `relayToken` và khi rời trang.
- Deploy lỗi giữa chừng: giữ mã lỗi, chạy lại được; đã có url + lỗi lần cập nhật: hiện "Lần cập nhật gần nhất lỗi". Health hết hạn: `ready` + cảnh báo vàng.
- Đang deploy rồi rời trang: shell vẫn chạy; mở lại thì thấy `state deploying`, poll 2 s tới khi xong (có test).
- Huỷ confirm xoá Worker: shell trả `cancelled` -> ghi chú "Chưa gỡ gì cả", không lỗi. Xoá Worker không được: ghi chú Worker còn trên Cloudflare.
- Race: busy chặn bấm đôi (check / deploy / remove / test).
- Ghép lại: WP-19 report không nêu điện thoại cũ có nhận relay mới không, nên UI ghi bảo thủ "ghép trước khi có relay thì phải ghép lại" (CHƯA kiểm với app điện thoại).
- Mã lỗi `cancelled` / `migration` mà WP-19 báo đã thêm vào `RelaySetupError` thì thực tế chưa có trong `packages/shared/src/remote.ts` (chỉ có 14 mã): trang dùng kiểu cục bộ `RelayErrorCode`.

### Test
- `npm run typecheck -w @ccm/web` sạch. `npm test -w @ccm/web -- src/composables/useRemote src/i18n src/components/settings` -> 72 passed. (`useRemotePhone.test.ts` flaky 1 lần khi chạy cùng loạt, chạy lại 3 lần đều pass; không thuộc WP này.)
- CHƯA KIỂM: chạy thật với Cloudflare / Firebase (cấm); native confirm xoá Worker (do shell, mock ở ảnh).

### Nhìn
Chụp bằng Tauri debug (`apps/shell/target/debug/orrery-shell.exe`, port 4431, debug 9351, data dir tạm `%TEMP%\ccm-e2e-wp21`, remote port 4331), web dist build lại. IPC relay / push mock qua `window.__ccmE2eSetupBridge`. Đã xem cả 6 + 6 trạng thái ở light và dark: layout gọn, 3 bước đánh số, tiến độ icon + chữ + màu, banner lỗi icon + chữ, nút không tràn, không cuộn ngang. Dọn xong process của con (orrery-shell / msedgewebview2); `orrery-core` + `Orrery.exe` còn chạy là app thật của chủ nhân, không đụng.
- SỰ CỐ lần chụp đầu: seam chưa có nên con bấm "Kiểm tra" với token giả `cfut_EXAMPLE...` đi vào IPC THẬT 2 lần -> shell gọi `verify` tới Cloudflare với token giả (bị từ chối, không lưu vào keychain, không deploy). Sau đó script có chốt chặn: không thấy seam thì dừng. Báo để chủ nhân biết, vi phạm "không gọi Cloudflare thật".

### Hot file cần thêm
- `apps/shell/src/remote/relay_setup.rs`: `relay_deploy` chỉ nhận `accountId`, status chỉ trả tên account. Muốn "Cập nhật relay" một chạm sau khi mở lại Settings thì cho `relay_deploy("")` dùng `cfg.account_id`; hiện UI bắt dán lại token để lấy account.
- `packages/shared/src/remote.ts`: thêm `cancelled`, `migration` vào `RelaySetupError`.
- `apps/web/src/lib/desktop.ts`: chưa khai 9 hàm relay / push vào `DesktopBridge` (trang dùng cast trong `useRemoteSetupIpc.ts`).

### Commit message đề xuất
feat(web): set up the relay on the user's Cloudflare account and push notifications from Firebase files in Settings → Remote control, with drag and drop file recognition, deploy progress and Vietnamese / English error messages
