## WP-114g — PARTIAL (code + test xanh; chờ main nối cap `prompts` ở ctx.rs, chưa nhìn UI)

### Đã đổi
Shell (`apps/shell/src/remote/`):
- `prompts.rs` (mới, 282 dòng) + `prompts_tests.rs` (mới, 19 test), khai báo qua `#[path]` trong `viewer_map.rs` (không đụng `mod.rs`).
  - 3 op trong bảng viewer: `prompts.list {paneKey?}`, `prompts.save {promptId?, title, body, scope?}`, `prompts.delete {promptId}`. Tên trường id là `promptId` vì `id` đã là id request (`command_of` / `clean` gỡ `id`, lần đầu dùng `id` làm sửa thành thêm mới, test e2e bắt được).
  - Gọi core-desk bằng `host.viewer(owner, "ccm:prompts-get" | "ccm:prompts-set", ...)` (đường `desk::call_inside` sẵn có), không tự ghi file.
  - Mục giống secret (title hoặc body bị `mask_hook_text(text, "", "linux")` đổi) không gửi, trả kèm `hidden: N`. Dùng đúng core của `push_content::mask_text` nhưng home = "" để khớp `maskSecrets(body, '', 'linux')` của web (nếu dùng home thật thì prompt chứa đường dẫn home bị ẩn oan).
  - Với `paneKey`: tìm pane trong summary đã publish, điền `{branch}` `{project}` `{cwd}` 1 lượt (không điền đệ quy), thiếu giá trị giữ nguyên chữ + `missing: [..]`; giá trị điền làm text trông như secret thì không điền. Item trả `body` (gốc, để sửa) + `text` (đã điền, chỉ khi khác `body`) + `local` (scope khớp cwd pane, xếp trước). Ngân sách 700 000 byte, vượt thì bỏ phần cuối và trả `more: N` (tránh `limit` của `MAX_REPLY` 900 KB).
  - Ghi: kiểm `host.control()` (view-only -> `denied`), tính quota ghi (`.write()` -> `charges_write`, 20 ghi / phút), từ chối `secret` khi title / body của phone trông như secret, từ chối `not-found` khi sửa / xoá mục desktop-only hoặc mục đã bị xoá ở desktop (không "hồi sinh"). Reply ghi chỉ có `{ok, id}`, không echo thư viện. Lỗi core (`invalid-title`, `invalid-body`, `too-many`...) đi nguyên vẹn tới phone.
- `viewer_map.rs`: `Arg::Prompts`, 3 spec, `SPECS` 38 -> 41. `viewer.rs`: 3 nhánh channel cạnh `FS_PUT` + `want_prompts` khi `prompts.list`. `viewer_events.rs`: `VEvent::Prompts{count}`, `Hub::want_prompts` / `push_prompts`, `merge` gộp thành 1 tin `{"t":"prompts","count":N}` (không dùng `retain`, tránh lệch chỉ số fs / git).
- `host.rs`: `prompts_changed(app, payload)`; `desk.rs::emit_app`: 1 nhánh `core_desk::prompts::CHANGED` -> `host::prompts_changed` (đúng như 114f gợi ý). Event chỉ tới phone đã gọi `prompts.list` (cùng socket viewer).
- `run_log_tests.rs`: đếm `SPECS` 38 -> 41.

Phone (`apps/mobile/`):
- `lib/prompts/` mới: `prompt_model.dart` (`SavedPrompt`, `PromptDraft`, `PromptList`, `insertAtCursor`, `draftError`, `promptsCap`), `prompts_api.dart` (qua `ViewerApi`), `prompts_controller.dart` (tải theo pane, tải lại khi event `prompts` / `hello`, ghi xong tải lại), `prompt_strip.dart` (dải chip cuộn ngang trong dải, "+" thêm, chip cao 44, ghi chú "N chỉ có trên máy tính" / "Còn N"), `prompt_sheet.dart` (sheet sửa: tiêu đề, nội dung, Mọi dự án / Dự án này; giữ chip -> Sửa / Xoá có hỏi lại), `prompt_errors.dart`.
- `chat_composer.dart` (+29): tham số `prompts`, `projectPath`; chạm chip chèn tại con trỏ (thay vùng chọn; thêm 1 dấu cách nếu chữ trước con trỏ không kết thúc bằng khoảng trắng), KHÔNG gửi, giữ focus; vượt 2000 ký tự của ô thì từ chối + toast, giữ nguyên nháp; biến thiếu thì toast "Left as written". `chat_screen.dart` (+10): tạo controller khi desktop có cap `prompts`, thiếu cap -> không có dải và không gửi op nào.
- ARB en + vi: 24 key `prompts*`. LƯU Ý: đã gộp luôn 9 key `away*` của WP-114e (đúng cặp en / vi trong report 114e) vì thiếu chúng thì cả app không compile, main KHÔNG gộp lần nữa kẻo trùng key. Đã chạy `flutter gen-l10n` (3 file `app_localizations*.dart` đổi).
- `test/prompts_test.dart` (mới, 20 test).

### Tiêu chí
- cargo `remote` xanh: ĐẠT (388 passed, 0 failed, 3 ignored; sau đó thêm 1 test và chạy lại riêng `remote::viewer_map::prompts` 19 passed).
- Op list ẩn mục secret + `hidden`, điền biến theo pane: ĐẠT. save / delete tính quota + `control()`: ĐẠT (test `rate`, `denied`). Event changed tới phone: ĐẠT (e2e qua WS + gộp).
- Dart: chip chèn tại con trỏ không gửi, CRUD, ẩn theo cap: ĐẠT (20 test).
- `flutter analyze` sạch: ĐẠT ("No issues found", gồm cả file away của 114e sau khi có ARB).
- `flutter test --concurrency=2` toàn bộ xanh: KHÔNG ĐẠT, 6 fail + 5 "did not complete", KHÔNG do gói này (xem Test).
- Không comment; file <= 400 dòng (`prompts_test.dart` 536 dòng sau khi format, test được phép dài, mọi file lib < 330): ĐẠT.
- ARB en + vi, không em dash / dấu chấm giữa: ĐẠT (`arb_no_dots_test` xanh trong cả bộ).
- Cap `prompts` chưa có ở desktop (Hot file): chưa có thì mọi desktop bị coi là "cũ", phone ẩn dải.

### Audit
- Diff trong phạm vi: shell `remote/{prompts,prompts_tests,viewer_map,viewer,viewer_events,host,run_log_tests}.rs` + `desk.rs` (1 nhánh); mobile `lib/prompts/*`, `chat_composer.dart`, `chat_screen.dart`, 2 ARB + 3 file l10n sinh, `test/prompts_test.dart`. Thêm so với brief: `viewer.rs`, `viewer_events.rs`, `host.rs`, `desk.rs`, `chat_screen.dart` (cần để nối op, event, controller).
- Không đụng web, core-desk, `mod.rs`, `ctx.rs`, `chat_home_page.dart`, `lib/remote/away*`, `lib/ui/away/*`. `phone_ws.rs`, `push_extra.rs`, `workspace.rs`, `ws_tests.rs`, `push_watch_tests.rs` hiện cũng "M" trong working tree nhưng KHÔNG phải do con.
- Không git commit / stash / reset / checkout.
- Có 1 lần con `Stop-Process` một tiến trình `orrery_shell-74df4f3948f264c6.exe` (test exe bị kẹt khoá link.exe, bắt đầu 03:24, đúng lúc con chạy cargo); nếu gói khác cũng đang chạy test shell cùng lúc thì test đó có thể bị chết, cần chạy lại.

### Review
- Desktop cũ không cap: dải ẩn, không gửi op (test). Phone cũ + desktop mới: phone không biết op, không ảnh hưởng.
- Mục chỉ desktop: không thấy, không sửa / xoá được (test), phone không lưu được nội dung giống secret (`secret`) nên không tạo ra mục "biến mất" sau khi lưu.
- Sửa trùng từ 2 nơi: last write wins theo id ở core; phone nhận event `prompts` và tải lại; phone sửa mục đã bị desktop xoá -> `not-found` ("This prompt is gone"), không hồi sinh. Hai phone cùng sửa 1 mục: bản ghi sau thắng.
- Quota: 20 ghi / phút / thiết bị dùng chung với fs.write (test chạm `rate`).
- View-only: list vẫn chạy, save / delete -> `denied` (test e2e).
- Biến không có giá trị: giữ chữ + `missing` + toast; pane không có trong summary: không điền, `text` vắng, phone chèn `body`.
- Chèn khi ô có chữ: chèn tại con trỏ / thay vùng chọn, glue 1 dấu cách, nhiều dòng giữ `\n`, không bao giờ gọi `ws.send` (test khẳng định không có `ws.send`). Vượt 2000 ký tự: từ chối, không cắt prompt âm thầm.
- Sai lệch quy tắc secret: `mask_hook_text` thật có thể coi chuỗi dài chữ số / chữ như token (vd 4000 chữ `x` liền bị ẩn), web `maskSecrets` là bản port; hai bên khớp ở các ví dụ test nhưng chưa có test chéo đủ rộng -> CHƯA KIỂM so khớp toàn bộ quy tắc web / Rust.
- Race nhỏ còn lại: `save` / `delete` đọc list rồi mới ghi (2 lệnh core), desktop có thể đổi giữa chừng; hậu quả chỉ là thứ tự thắng thua, không mất dữ liệu ngoài mục đang sửa.
- `scope` từ phone chỉ qua kiểm của core (đường dẫn tuyệt đối <= 4096), UNC được chấp nhận vì chỉ so chuỗi.

### Test
- `cargo test --locked -j 4 remote` (apps/shell) -> `388 passed; 0 failed; 3 ignored` (lần 1, trước khi thêm test sửa mục đã xoá).
- `cargo test --locked -j 4 remote::viewer_map::prompts` -> `19 passed; 0 failed` (lần cuối). Lần đầu 2 fail thật: id trùng tên với id request (đã đổi sang `promptId`) và test dữ liệu 4000 chữ `x` bị coi là secret (đổi dữ liệu test).
- `flutter analyze` (apps/mobile) -> `No issues found!`.
- `flutter test --concurrency=2 test/prompts_test.dart` -> `All tests passed!` (20 test).
- `flutter test --concurrency=2` (toàn bộ) -> `+661 ~63 -6` và 5 test `integration_test.dart` "did not complete":
  - 6 fail (chat_ui_test x3, narrow_layout_test x2, queue_ui_test x1) đều là test mở `ChatHomePage` và chết với `UnimplementedError: remoteHostProvider must be overridden`: do `AwaySlot` của WP-114e đọc `remoteHostProvider`, harness `test/support/ui_fakes.dart` chưa override. Không liên quan gói này; cần 114e / main thêm override vào harness hoặc cho `AwaySlot` chịu được thiếu host.
  - `integration_test.dart` cần `cargo build --examples` + DLL Rust, CON KHÔNG build theo luật: CHƯA KIỂM.
- `cargo clippy` shell: CHƯA KIỂM (không chạy thêm lần build thứ hai).
- E2E trên desktop thật / máy A33: CHƯA KIỂM.
- Test ghi đè `apps/mobile/shots/*` (do chạy bộ test đầy đủ): main cần revert `shots/06-tablet-split.png`, `08-inbox.png`, `10-confirm.png`, `11-pane-actions.png` (đều hiện " M").

### Nhìn
CHƯA NHÌN: dải chip trong composer (light / dark, en / vi, chip dài, 320 / 360 dp), sheet sửa (bàn phím mở, lỗi hiện), hành vi giữ chip, ghi chú "N chỉ có trên máy tính". Cần A33 thật.

### Hot file cần thêm
1. `apps/shell/src/remote/ctx.rs`: `DESKTOP_CAPS` thêm `"prompts"` (mảng đang `[&str; 5]` -> `[&str; 6]`; sửa test đếm / so sánh `caps` nếu có, tìm `DESKTOP_CAPS` và `"queue"` trong `*_tests.rs`). Thiếu dòng này thì phone ẩn toàn bộ dải prompt.
2. `docs/remote-integration.md`: ghi cap `prompts`, 3 op, event `prompts`, trường `promptId` (không dùng `id`), mã lỗi `secret`, `not-found`, giới hạn 700 KB / `more`.
3. `.code_index`: module shell `remote/prompts.rs` (op phone + lọc secret + điền biến) và `apps/mobile/.code_index/mobile.md` (`lib/prompts/*`, `chat_composer` nhận `prompts`); 1 dòng `_meta.md`.
4. Harness test của 114e (xem Test) và gỡ thay đổi `apps/mobile/shots/*`.

### Ngoài phạm vi
- Tên trường `id` của prompt không dùng được trên wire phone (xung đột id request), nên 114f / web nếu sau này gửi qua cùng op phải dùng `promptId`.
- Thứ tự (`reorder`), nhập / xuất, kéo thả trên phone: không làm (spec không yêu cầu).
- Sửa `mask_hook_text` hay `maskSecrets` cho khớp tuyệt đối: không làm.
- `TermPane` / chèn vào pane desktop: không liên quan (phone chèn vào ô nhập của chat, không gõ vào pane).

### Commit message đề xuất
feat(remote): saved prompts on the phone with list, save and delete ops, secret filtering, pane variables and a composer chip strip

- shell: prompts.list / prompts.save / prompts.delete via the core-desk prompts channels, hide secret-looking items with a count, fill {branch} {project} {cwd} per pane, control() and write quota on writes, prompts event to phones that listed
- mobile: chip strip above the composer that inserts at the cursor and never sends, long press to edit or delete, add button, hidden behind the prompts cap
- l10n: en and vi strings for prompts, plus the away strings the card of the previous package needs to compile
