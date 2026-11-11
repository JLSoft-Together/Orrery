## WP-8 — DONE (2 điểm lệch brief, xem "Tiêu chí" và "Review")

### Đã đổi
- Mới `apps/shell/src/remote/`: `viewer_map.rs` (bảng `SPECS` 31 lệnh → kênh, dạng args, timeout, cờ write / run / pre-confirm / watch; `build_args`, `outcome`, `pre_needs`), `viewer.rs` (`Phone::viewer_command|viewer_call|viewer_after|on_vevent`, `ViewerState`, `Guard` drop = nhả watcher), `viewer_events.rs` (`Hub` trong `Ctx.viewer`: đăng ký phiên, rate 120 / phút + `fs.write` 20 / phút, `push` / `push_run`, `merge` gom 100 ms), `scope.rs` (thread-local "lệnh từ điện thoại": confirm + tắt full-access), `viewer_tests.rs` (8 test).
- Sửa trong `remote/`: `ctx.rs` (trait `Host` thêm `viewer`, `viewer_release`; `Ctx.viewer`), `host.rs` (`AppHost::viewer` = `preview_ipc::dispatch(app, owner, …)` cho kênh preview, còn lại `desk::call_inside`; `viewer_event`, `run_event`, `is_viewer_owner`), `mod.rs` (module + `Remote::viewer_event|run_event`, re-export `VEvent`), `phone_ws.rs` (`Extra.v`, `Done::new`, nhánh `lookup` trong `extra`, `on_done` hỏi tiếp được với lệnh viewer + giới hạn kích thước reply, `confirm_answer` gọi lại lệnh viewer), `dashboard.rs` (nhánh select `on_vevent`, `Extra::new(.., &device)`), `test_util.rs` (TestHost: `viewer`, `on_release`, `released`, `views`).
- Nối dây ngoài `remote/`: `desk.rs` (`emit_app` chuyển `ccm:run-event` cho gateway), `hosts.rs` (`confirm` đầu hàm hỏi `scope::confirm`), `preview_host.rs` (`allowed_roots` bỏ full-access khi `scope::active()`), `preview_ipc.rs` (`notify_changes` / `on_root_change`: owner `remote:*` → `host::viewer_event`). `Subs` không cần sửa (owner là string, nhánh tách ở chỗ gọi).
- `.code_index/shell.md` (1 mục WP-8), `docs/remote-integration.md` §4.5 (bảng shape thật). Không đụng `apps/core`, `apps/web`, `apps/mobile`.

### Bảng lệnh → kênh thật + shape
Reply thành công `{re, ok:true, data}` (`data` = kết quả nguyên của desk; lỗi nghiệp vụ nằm trong `data.ok === false` + `data.error`); từ chối → `{ok:false, error:"denied"}`; sai field → `invalid`; reply > 900 000 byte → `limit`; `timeout` / `offline`; cần xác nhận → `{ok:true, confirm: cid}` + event `confirm.ask`.

| Lệnh | Args | Kênh | Ghi chú |
|---|---|---|---|
| `fs.list` | `{root, dir}` | `ccm:list-dir` | |
| `fs.files` | `{root}` | `ccm:list-files` | preview_ipc |
| `fs.read` | `{path}` | `ccm:read-file` | `data.kind` text / binary / image / too-large / missing |
| `fs.write` | `{path, text, base?}` | `ccm:file-write` | `base` → `expectedMtime`; 20 / phút |
| `fs.watch` / `fs.unwatch` | `{root}` | `ccm:watch` / `ccm:unwatch` | event `fs.changed {root, paths[]}` |
| `git.status` / `git.diff` / `git.file` | `{root}` / `{root, path, opts?}` / `{path}` | `ccm:changes-status` / `changes-diff` / `changes-file` | |
| `git.watch` / `git.unwatch` (thêm, brief thiếu) | `{root}` | `ccm:changes-watch` / `changes-unwatch` | event `git.changed {root}` |
| `git.stage` / `unstage` / `commit` | Obj | `ccm:git-stage` / `git-unstage` / `git-commit` | amend → confirm |
| `git.discard` | `{root, path, hunk?}` | `ccm:changes-discard` | gateway hỏi trước |
| `log.refs` / `page` / `commit` / `diff` | `{root, …}` | `ccm:log-refs` / `log-page` / `log-commit` / `log-diff` | |
| `branch.create` / `update` / `fetch` / `push-preview` | `{root, …}` | `ccm:git-branch-create` / `git-branch-update` / `git-fetch` / `git-push-preview` | update / fetch 60 s |
| `branch.delete` | `{root, ref}` | `ccm:git-branch-delete` | desk hỏi (kể cả force) → phone |
| `branch.push` | `{root, ref, remote?, branch?, setUpstream?}` | `ccm:git-push` | gateway hỏi trước, 60 s |
| `run.configs` / `run.devices` (thêm) / `run.sessions` | `{root}` / — / — | `ccm:run-configs` / `run-devices` / `run-sessions` | `run.sessions` đăng ký `run.event` |
| `run.start` | `{root, configId, deviceId?}` | `ccm:run-start` | đăng ký `run.event`; config lạ hỏi phone |
| `run.stop` / `run.reload` | `{sessionId}` / `{sessionId, hot?}` | `ccm:run-stop` / `run-reload` | |

Event: `run.event {event}` (payload `ccm:run-event`, bỏ > 256 KB). Cấm: `grant-folder`, `pick-*`, `open-path`, `reveal`, `request-access`, `full-access`, `forget-grants`, `create / rename / trash-entry`, `search`, mọi kênh khác.

### Tiêu chí: 2/2 (kèm lệch)
- [x] `cargo test --locked -j 3 remote` + toàn shell pass. Test mới: lệnh ngoài bảng → `denied`; path ngoài root → `denied` (read / write / list); `fs.read` trong root → text; `fs.write` + đọc lại + file ngoài root không đổi; watch → sửa file → đúng thiết bị nhận `fs.changed`, thiết bị khác không (1.5 s im), đóng phiên → watcher nhả + `viewer_release(owner)`; `branch.delete` → `confirm.ask` tới thiết bị gửi, đồng ý → nhánh mất, huỷ → còn; `git.discard` hỏi trước, huỷ → file giữ nguyên; `host.asked` (hộp native) rỗng; `run.configs` trả list; `run.event` chỉ tới thiết bị đã `run.sessions`; reply quá lớn → `limit`, phiên sống.
- [x] Không comment (rg rỗng); file dài nhất `viewer_tests.rs` 363; clippy `--all-targets -D warnings`: `remote/` sạch, còn đúng 8 lỗi có sẵn ngoài phạm vi; `.code_index/shell.md` + docs §4.5 cập nhật.
- Lệch brief: `fs.read` max thực tế ~900 KB (không phải 2 MiB): proto `MAX_MESSAGE` 1 MiB, vượt là `Secure::frame` fail và đóng phiên. Desk vẫn đọc tới 2 MiB; reply lớn hơn → `limit`. Muốn 2 MiB cần core chunk lớn hơn (ngoài phạm vi). Shape `fs.read` là `data.kind` của desk, không phải `{binary:true}`.

### Audit
Diff gồm đúng các file ở "Đã đổi". Không log path / nội dung / token. Gateway không bao giờ chuyển tên kênh từ điện thoại: chỉ `lookup(t)` trong bảng; `ccm:*` thô → `denied`. Args dựng lại từ field cho phép (`Str` ≤ 4096 ký tự, `Write` chỉ path / text / expectedMtime, `Obj` bỏ `id` `t` `confirmed` `_accept`).

### Review
- Whitelist: test duyệt `SPECS` không chứa kênh picker / grant / backup / agent / open; lệnh lạ, `ccm:read-file`, `fs.grant` → `denied`.
- Path traversal / symlink: toàn bộ qua `Gate` (`real_or_null` + `root_of`) của desktop; test path ngoài root. Full-access của desktop KHÔNG áp cho phone (`scope::active()` trong `allowed_roots`); grant do chủ nhân cấp vẫn áp.
- Confirm đúng thiết bị: `hosts::confirm` hỏi `scope` trước mọi thứ (kể cả `CCM_E2E_CONFIRM`, `show_main`); `accept` chỉ do gateway lưu trong `Confirms` (phone gửi `_accept` bị bỏ); key = hash title + detail nên confirm khác lần trước (force delete) hỏi thêm chứ không tự nhận. Confirm chạy cùng thread với dispatch (spawn_blocking) nên thread-local đủ; guard drop xoá scope.
- Watcher rò: owner theo phiên `remote:<device>:<nonce>` (2 phiên cùng thiết bị không nhả nhầm nhau); `Guard::drop` → `Hub::remove` + `preview_ipc::release`; max 8 watch / phiên, trùng thì không tăng ref; hàng event 64, tràn thì bỏ (event chỉ là gợi ý reload).
- Rủi ro còn lại:
  1. `reinstall` của run xin xác nhận ở thread runner (không phải thread dispatch) → vẫn hộp native trên desktop (chặn / chờ chủ nhân). Cần core đưa nguồn lệnh vào `RunManager` hoặc `run.start` từ phone bỏ `reinstall`.
  2. `run.start` config chưa trust: phone đồng ý → `trust.add` ghi trust như desktop (chủ ý).
  3. `branch.push` / `branch.fetch` / `branch.update` dùng GitHub account đã cấu hình trên desktop (`preview.ghEnv`); push có xác nhận trước, fetch / update thì không (giống desktop).
  4. Timeout gateway không huỷ lệnh đang chạy (git fetch / push 60 s có thể xong sau khi phone nhận `timeout`).
  5. Quy ước owner `remote:` giả định webview label không chứa `:`.
- CHƯA KIỂM: `AppHost::viewer` / `preview_ipc` thật qua AppHandle (không dựng được Tauri app trong test; test dùng engine cùng `Desk::standard` + `Gate` + `scope`, watcher `RootWatcher` thật); `run.start` thật (adb / flutter) và nhánh trust-config; `desk::emit_app` → `run_event` trên app chạy thật; macOS.

### Test
- `cd apps/shell && cargo test --locked -j 3 remote` → 51 pass / 0 fail; toàn shell → 245 pass / 0 fail.
- `cargo clippy --locked -j 3 --all-targets -- -D warnings` → chỉ 8 lỗi có sẵn (`host_extra.rs:102`, `preview_host.rs:86`, `preview_watch.rs:106,204`, `vault.rs:61`, `preview_serve_tests.rs:95,101,103`).
- Một số lệnh cargo phải chạy với `JIN_FORCE=1` vì máy báo CPU > 80% do tiến trình khác (WP-11b song song).

### Cho WP-12 (app viewer)
- Mọi lệnh: `{id, t:"fs.read", path}`; reply `{re, ok:true, data}` hoặc `{re, ok:false, error}`. Đọc `data.ok === false` / `data.kind` cho lỗi nghiệp vụ. Reply quá lớn → `limit`: hiện "file quá lớn cho điện thoại".
- Cần xác nhận: reply `{ok:true, confirm: cid}` + `confirm.ask`; trả lời `confirm.answer {cid, ok}`, kết quả thật nằm ở reply của answer; có thể có lượt `confirm.ask` thứ 2 (reply answer lại là `{ok:true, confirm: cid2}`); `{cancelled:true}` khi từ chối.
- Watch: gọi `fs.watch {root}` / `git.watch {root}` khi mở màn, `unwatch` khi rời; max 8; nhận `fs.changed` / `git.changed` rồi tự reload; event `run.event` tự có sau `run.sessions`.
- `fs.write {path, text, base}` với `base` = `mtimeMs` của `fs.read`; `data.error:"conflict"` → báo xung đột.
- Rate: 120 lệnh viewer / phút, `fs.write` 20 / phút; `rate` → đợi.

### Ngoài phạm vi
- `apps/core`: chunk > 1 MiB cho reply lớn (để `fs.read` đủ 2 MiB); đưa nguồn lệnh vào `RunManager` để confirm `reinstall` từ phone không mở hộp native; `ccm:run-start` trả riêng trạng thái trust.
- Bộ gom log run 200 dòng của `host_telegram` không dùng lại (phone nhận `run.event` thô theo mẻ 100 ms; app tự giữ log).
- Docs §5 giữ nguyên dòng giới hạn cũ (đã khớp `fs.write` 20 / phút).

### Commit message đề xuất
feat(shell): remote gateway viewer commands (fs, git, log, branch, run) through a fixed channel table and the desktop preview gate, per-session watch and run events, phone-side confirm without native dialogs

## Vòng 2 — DONE (review `WP-8-review.md`, 0 chặn / 0 cao)

### Đã sửa
- (vừa) UNC / device path: `viewer_map::local_path_ok` từ chối `\…`, `//…` (gồm `\?\UNC\`, `\.\pipe\`) và `\0` trước mọi canonicalize; áp cho `text()` (Str / Write) và mọi field `root` `dir` `path` `file` `origPath` của `Arg::Obj` (`paths_ok`, field không phải string → `invalid`). Test `unc_and_device_paths_are_refused…`: `invalid`, `host.views` không tăng.
- (vừa) Confirm `branch.push`: detail = `ref → remote/branch` + dòng "Set upstream" khi `setUpstream:true` + "Force push" khi `force:true` + cảnh báo khi đích là `main` / `master`; thiếu `branch` thì lấy tên ngắn của `ref`. Test `a_push_confirm_names…` (pure) + `a_push_asks_with_the_full_target…` (qua phiên: hỏi đủ, huỷ → không có `ccm:git-push` nào tới host).
- (thấp) Watch: `Hub::is_open`; lệnh watch chạy xong sau khi phiên đóng → closure gọi `viewer_release(owner)` lần nữa (test `a_watch_that_lands_after_the_session_closed…`); `fs.unwatch` / `git.unwatch` luôn xuống desk (bỏ early-return); max 8 đếm cả watch đang chạy (`pending`, settle ở đầu `on_done` kể cả timeout / lỗi) và key chuẩn hoá (bỏ separator cuối, Windows: lowercase + `\`) — test `watch_count_is_capped_and_keys_are_normalised`.
- (thấp) `fs.write` không có `base` / `expectedMtime` số → bắt buộc confirm "Overwrite file?" trên điện thoại (`Pre::Overwrite`, `pre_of`); test `a_write_without_a_base_asks…`. Test cũ cập nhật dùng `base`.
- (thấp) `confirm.answer` cho lệnh viewer đi qua `take_rate` (+ `take_write` nếu `spec.write`) → `rate`.
- (thấp) Timeout git: lệnh `SLOW` (fetch / update / push / delete / run.start) giữ `Busy` theo `device|root` trong `Hub`, nhả khi closure blocking THẬT SỰ xong (không phải lúc timeout) → lệnh thứ 2 cùng root / thiết bị → `busy`; root khác vẫn chạy. Test `one_slow_git_task_per_root_and_device…`.

### Để sau (xếp WP riêng)
- `run.start` + confirm `reinstall` (Android) chạy ở thread runner nên vẫn mở hộp native trên desktop: để sau, cần sửa core-desk run (`RunManager.start` nhận nguồn lệnh / trả `needsConfirm`).
- Chưa làm: `on_vevent` vẫn `sleep(100 ms)` trong `select!`, chưa cắt `paths` theo tổng byte (review thấp #6).
- `hosts.rs` chỉ thêm dòng đầu `confirm`; không đụng arm khác (WP-13 `remote.link` an toàn).

### Test vòng 2
- `cargo test --locked -j 3 remote` → 61 pass; toàn shell → 255 pass; clippy `--all-targets -D warnings`: `remote/` sạch, còn 8 lỗi có sẵn. Không comment; file dài nhất `viewer_tests.rs` 370. File mới: `remote/viewer_guard_tests.rs`.
