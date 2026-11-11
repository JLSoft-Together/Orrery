## WP-12 — PARTIAL (tiêu chí 4/4 đạt, còn mục CHƯA KIỂM bên dưới)

### Đã đổi
- `apps/mobile/lib/ui/viewers/**` (mới): Explorer, File (highlight, số dòng, tìm, sửa + lưu `base`, conflict), Changes (stage / unstage / discard / commit), Diff (unified, gộp context dài), Git Log (graph 1 cột, phân trang), Commit, Branches (tạo / fetch / update / push có preview đích / xoá), Run (config / device / start / stop / reload / log 200 dòng), `viewer_api`, `viewer_actions`, `async_panel`, models.
- `lib/ui/monitor/**` (mới): tab Monitor "Quỹ đạo" dạng list (project, session, cây subagent, context %, token chỉ khi có `totalTokens`), chạm mở pane.
- Sửa: `ui/machine_api.dart` (`events`, `timeout`), `ui/workspace/workspace_page.dart` (bottom nav Workspace · Monitor · Inbox, bỏ nút Inbox trên AppBar), `ui/workspace/tab_page.dart` (nút menu viewer), `pubspec.yaml`, ARB vi + en (`tool/arb_viewers.mjs`, 100+ khoá `v*`), `.code_index/mobile.md`.
- Test: `test/viewers_test.dart` (13), `test/ui_viewer_integration_test.dart` (fake gateway thật: Explorer -> đọc -> sửa + lưu base -> `fs.changed`), `test/viewers_shots_test.dart`, `test/support/ui_fakes.dart`; Rust `rust/tests/common/viewer.rs` + `commands.rs` / `mod.rs` (fake gateway trả lời `fs.*`, `git.status`, watch).
- Ảnh: `docs/qa/v97/WP-12/*.png` (17 ảnh, sáng / tối, vi / en).

### Package thêm + lý do
- `highlight ^0.7.0` (MIT): chỉ cần parser (không kèm widget như `flutter_highlight`), để tự cắt token theo dòng cho list ảo hoá; > 24k ký tự chạy trong isolate, > 160k ký tự hiện plain.

### Tiêu chí 4/4
- [x] `flutter analyze` sạch; `flutter test --concurrency=4` 161 pass (gồm integration; 17 skip = ảnh).
- [x] Integration qua fake gateway: Explorer -> đọc -> sửa + lưu (base) -> `fs.changed` (không hiện banner giả cho chính lần lưu của mình).
- [x] Ảnh từng viewer, chụp sau lần sửa cuối, đã tự mở xem (explorer, file, conflict, changes, diff, push dialog, run, monitor dark vi).
- [x] vi + en đủ; không comment (rg rỗng); `.code_index` cập nhật. File > 400 dòng: `run_page` 421, `explorer_page` 419, `changes_page` 415 (nhỉnh nhẹ, chưa tách).

### Audit
Chỉ đụng `apps/mobile/**`, `docs/qa/v97/WP-12/**`, brief / report. Không commit.

### Review (flutter-reviewer, sonnet): 0 chặn, 2 cao, 11 vừa
- Đã sửa: cao 1 (dispose controller dialog sớm), cao 2 (`ViewerApi` ref.read + onDispose, không rò listener), vừa 3 (Explorer reload khi đang tìm), vừa 6 (đang sửa không tự reload, lưu `pending` tính là lần ghi của mình), vừa 7 (size theo byte), vừa 9 (Run merge session thay vì xoá).
- Chưa sửa (ghi nhận): vừa 4/5/12/13 (Explorer `_loadDir` không guard thế hệ, fuzzy chạy trong build, `ListView(children)` không lazy ở Changes / Branches, CodeView tính lại độ rộng), vừa 8 (`reloadAfterConfirm` nếu confirm xong trước khi gắn listener), vừa 10 (state Run hiện chuỗi gốc), 11 (load thêm khi trang đầu ngắn hơn màn / `IntrinsicHeight`); thấp: chặn bấm đúp, a11y nhãn mark, touch target 36–40 dp, chuỗi `+N -N` / thời gian chưa vào ARB.

### Test
- `cd apps/mobile && flutter analyze && flutter test --concurrency=4` -> sạch / 161 pass.

### Nhìn
- Đã xem 01, 02, 02c, 03, 04, 06b, 07, 08b. Ký tự mũi tên ahead / behind hiện ô vuông trong ảnh do font test Roboto thiếu glyph (Changes đã đổi sang icon; Branches vẫn dùng chữ ↑↓).
- CHƯA NHÌN: ảnh dark của file / diff / changes chỉ xem nhanh; ảnh trên máy A21s không chụp (headless như WP-11).

### CHƯA KIỂM
- Chạy với gateway / desktop thật (shape `data` suy ra từ code desktop; `run.start`, branch ops, confirm 2 lượt chưa thử thật); A21s; iOS; tablet.
- Watch không tự gắn lại nếu phiên bị gateway đóng mà không có `hello` mới (đã gắn lại khi `hello`).

### Ngoài phạm vi
- Xoá remote branch, rename / create entry trong Explorer (gateway cấm).

### Commit message đề xuất
feat(mobile): viewer screens (explorer, file view and edit with base, changes, diff, git log, branches, run) and Monitor orbit list over the remote gateway, bottom navigation, fake gateway viewer commands and tests
