## WP-113b — DONE

### Đã đổi
- `scripts/dist-tauri.mjs`: `stageApk(j)` (ORRERY_APK > `flutter build apk --release --target-platform android-arm64`, GRADLE_OPTS workers.max=j; apksigner tìm trong ANDROID_HOME/ANDROID_SDK_ROOT build-tools bản mới nhất; so SHA-256 với `apps/mobile/android/cert.sha256`; chép `orrery-remote.apk` + `version.txt` từ pubspec), cờ `--skip-apk`, `SETUP_MAX_MB` 70. Helper export: `pubspecVersionName`, `normCert`, `parseSignerCerts`, `certMatches`.
- `apps/shell/tauri.release.json`: resource apk + version.txt. `.gitignore`: `apps/shell/resources/apk/*.apk`, `version.txt`. `resources/apk/.gitkeep`.
- `remote/ctx.rs` (chỉ trait Host): `bundled_apk()`, `save_apk(src, name)` (default None / Ok(None)).
- `remote/host.rs`: AppHost resolve Resource `apk/...`, hộp thoại lưu mặc định Downloads, copy.
- `remote/install.rs`: `TicketKind {ApkPage, ApkOffer}` + `ttl()`, `Tickets` Vec ≤ `MAX_SLOTS`=4 (đầy thì bỏ cũ nhất cùng kind, không có thì cũ nhất), `lookup` trả `(kind, path)` so hằng thời gian qua mọi slot; `Source`, `ApkSource`, `pick_source`, `save_name`; `Route` enum + `route()` tách khỏi `resolve` cho F3.
- `remote/install_ops.rs`: `apk_source()` dùng chung `get`/`start`; reply thêm `source`, `version`; op `save`; `start` cấp ticket `ApkPage`.
- `remote/test_util.rs`: TestHost `bundled`, `save_to` (copy thật nên lỗi ghi là lỗi thật).
- `remote/install_tests.rs`: test mới/đổi.
- `apps/web/src/features/remote/lib/remoteInstall.ts` + test: `source`, `version`, `parseSaved`, `installSave`, error `save|cancel`.

### Tiêu chí
- [x] Thứ tự 3 nguồn cho get và start (test pure `pick_source` + `get_and_start_share_the_same_apk_source`).
- [x] save thành công / huỷ / lỗi ghi / không có apk.
- [x] 2 ticket sống song song hết hạn riêng, slot 5 đẩy cũ nhất cùng kind; route file cũ giữ nguyên (test cũ xanh).
- [x] `node --check scripts/dist-tauri.mjs` sạch; helper thử bằng node.
- [x] Vitest parse field mới.
- [x] Không comment, file ≤ 400 dòng.

### Audit
- Hành vi đổi có chủ ý: ticket mới KHÔNG còn thay ticket cũ (test cũ `a_new_ticket_replaces_the_old_one` đổi thành `tickets_live_side_by_side...`). `stop`/`clear`/`set_apk` vẫn `revoke()` toàn bộ.
- `install_state.missing` chỉ tính custom `apk_path` hỏng. `apk` nay là nguồn hiện hành (có thể bundled/dev); `suggested` loại trừ nguồn hiện hành.
- File cũ là CRLF, đã giữ CRLF.

### Review
- Không unwrap ngoài test; không lock giữ qua await; lookup so qua mọi slot không thoát sớm.
- `--skip-apk` mà chưa có APK staged thì `tauri build` lỗi do thiếu resource (chấp nhận, dev tự chịu).
- `save_name` lọc ký tự, không cho `/`.

### Test
- `cd apps/shell; cargo test --locked -j 4 install` → 21 passed, 0 failed.
- `npm test -w @ccm/web -- src/features/remote/lib/remoteInstall` → 7 passed.
- `npm run typecheck` → chỉ lỗi `connectCheck.test.ts` (25,35,36,50,51 TS2532, file của WP khác); không lỗi ở file WP này.
- `node --check scripts/dist-tauri.mjs` → ok.
- CHƯA KIỂM: `stageApk` chạy thật (cần Flutter + Android SDK + dist, brief cấm); AppHost dialog lưu/resource resolve chỉ compile, chưa chạy trong bản cài.

### Nhìn: không áp dụng

### Hot file cần thêm
- `.code_index/*` (remote/install: Tickets nhiều slot, pick_source, op save, stageApk) và `docs`: hợp đồng `ccm:remote-install` reply thêm `source`, `version`, op `save`.
- `QA-mac-checklist`: bước chuẩn bị Mac cho dist (Flutter, Android SDK, keystore hoặc `ORRERY_APK`).
- i18n cho lỗi `save`/`cancel` (F3 UI).

### Ngoài phạm vi
- Trang HTML (F3), UI `RemoteInstallCard.vue`, `apps/mobile`.

### Commit message đề xuất
feat(remote): bundle the phone APK in the installer with one shared apk source, save op and multi-slot tickets
