## WP-113n — DONE (cần main nối 2 chỗ hot file)

### Đã đổi
- `apps/shell/src/remote/viewer_map.rs`: `Arg::Check(op)` (ép op `list`/`toggle`, chỉ nhận `dir`, `wt`, `id` ≤ 64, `done` bool), `Pre::CheckRun` (confirm trên phone), `check_run()` (probe `ccm:checklist` list qua guard register_repo rồi `host.ask("checklist.run")`), 3 spec `checklist.get|toggle|run` (SPECS 31 -> 34).
- `apps/shell/src/remote/viewer.rs`: nhánh `CHECK_RUN` gọi `check_run`.
- `apps/shell/src/remote/viewer_guard_tests.rs`: 3 test mới.
- `packages/shared/src/orreryApi.ts`: action page `checklist.run` (source phone). Ngoài phạm vi brief nhưng bắt buộc: kênh shell -> web chỉ có qua `host.ask` + ORRERY_ACTIONS.
- Web: `features/git/lib/checklist.ts` (`startCheckFromPhone`, chặn chạy trùng bằng `isCheckLive`, chưa bật shell integration thì báo lỗi), `features/git/composables/useChecklistRun.ts` (`installChecklistRun`), export ở `features/git/index.ts`, test thêm vào `checklist.test.ts`.
- Phone: `lib/ui/checklist/{checklist_model,checklist_parts,checklist_page}.dart`, lối vào `viewers/viewer_menu.dart` (chỉ hiện khi cap `checklist`), mã lỗi `running|script|integration|unavailable` trong `viewers/async_panel.dart`, ARB en + vi (đã `flutter gen-l10n`), `test/checklist_test.dart`.

### Tiêu chí
- [x] cargo: op map đúng, path qua guard sẵn có (ngoài root -> denied), run cần confirm, không ask khi huỷ / bị denied.
- [x] Vitest: gọi runCheck 1 lần, đang chạy thì bỏ qua (`running`).
- [x] Dart: hiện items + trạng thái kiểm, tick gửi toggle, chạy lại hỏi confirm (phone), ẩn khi thiếu cap.
- [x] Không comment, file < 400 dòng, chuỗi en+vi.
- Lưu ý: confirm của `run` hiện trên PHONE (luồng viewer sẵn có), không phải hộp thoại desktop.

### Audit
Phone chỉ gửi được op list/toggle (cmd `op` bị ép, `add/remove/setScript/setCheck` không đi qua). `wt` UNC bị từ chối. Web handler trả ngay `started`, phone poll 3s (max 200 lần) khi state `running`.

### Review / edge case
- Worktree không tồn tại hoặc ngoài project: desk trả denied (test) -> không ask web.
- Đang chạy kiểm: web trả `running`, phone hiện thông báo; phone không biết desktop có thật sự live hay state `running` bị kẹt nên nút Run luôn bấm được.
- App cũ không có cap: menu không hiện mục.
- Pane cwd là thư mục con của worktree: `wt` = cwd không khớp `git worktree list` -> báo `unavailable` (hạn chế, chưa xử lý).

### Test
- `cargo test --locked -j 4 remote` -> 300 passed, 0 failed, 3 ignored.
- `npm test -w @ccm/web -- src/features/git src/core/composables` -> 195 passed.
- `npm run typecheck` -> lỗi chỉ ở `core/lib/desktop.ts`, `PhoneConnectWizard.test.ts`, `remoteInstall.test.ts` (WP-113b/e2), không có lỗi ở file của WP này.
- `flutter analyze` -> No issues; `flutter test --concurrency=2 test/checklist_test.dart test/arb_no_dots_test.dart test/viewers_test.dart` -> 25 passed.
- `npm run qa -- --no-e2e`: Gate: bỏ qua, các bước tương đương (typecheck, vitest, cargo test) đã chạy riêng ở trên, qa tổng sẽ vấp lỗi typecheck của WP khác.

### Nhìn: CHƯA NHÌN
Trang checklist phone (list item, dòng trạng thái kiểm, nút Chạy kiểm, tag Sẵn sàng merge) chưa chụp, chỉ test widget.

### Hot file cần thêm
1. `apps/shell/src/remote/ctx.rs`: `DESKTOP_CAPS: [&str; 1] = ["devinfo"]` -> thêm `"checklist"` (nâng độ dài mảng).
2. `apps/web/src/App.vue`: import `installChecklistRun` từ `@features/git` và gọi cạnh `installRemotePhone()` (trong khối `hasTerminals && !ui.compact`). Chưa nối thì `checklist.run` trả `unavailable`.
3. `.code_index/*`, README feature git: ghi thêm file mới.

### Ngoài phạm vi
Sửa `packages/shared/src/orreryApi.ts` (cần để có action mới). Không sửa `checklist.rs`.

### Commit message đề xuất
feat(remote): pre-merge checklist on the phone with tick and confirmed check run
