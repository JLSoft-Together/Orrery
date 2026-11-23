## WP-112g — DONE
- Đã đổi:
  - `apps/core/crates/core-desk/src/preview/stash.rs` (mới) + `stash_tests.rs` (mới): `list` (hash, index, time, untracked, branch, message; max 200), `push` (tên, `--include-untracked` tuỳ chọn, chặn "nothing" / "no-commit"), `apply` / `pop` / `drop` tìm entry theo hash (list đổi giữa chừng không xoá nhầm), lỗi `conflict` / `dirty` / `gone`, `undo_commit` (`reset --soft HEAD~1`; chặn `stale` HEAD lệch, `root-commit`, `merge`, `pushed` qua `head_unpushed`; trả message đầy đủ để Commit lại). Stash chạy với `GIT_LITERAL_PATHSPECS=0` vì bật `1` làm `stash -u` lưu file untracked nhưng không xoá khỏi thư mục.
  - `preview/mod.rs` (`pub mod stash`), `preview/ipc_git.rs`: kênh `ccm:git-stash {root, op, name?, untracked?, hash?}` (khoá một thao tác mỗi repo, Drop hỏi confirm native kind `stashdrop`), `ccm:git-undo-commit {root, hash}`.
  - `apps/shell/src/preview_text.rs`: tiêu đề + chi tiết confirm `stashdrop` en / vi, 1 assert test.
  - `apps/shell/shim/preload.ts` + `apps/shell/src/shim.js` (đã gen): `gitStash`, `gitUndoCommit`.
  - `apps/web/src/core/lib/preview.ts`: type `StashEntry` / `StashList` / `StashOpResult` / `StashRequest` / `UndoCommitResult`, `hasStash`, `stashList`, `stashRun`, `hasUndoCommit`, `gitUndoCommit`.
  - `features/git/lib/stash.ts` + `stash.test.ts`, `components/log/StashMenu.vue` + `StashMenu.test.ts`, `components/StashDialog.vue`, `i18n/stash-v112.ts`.
  - `components/log/LogPane.vue`: nút stash trong header, mục "Hoàn tác commit cuối (giữ file)" ở menu chuột phải của HEAD (1 parent, chưa có trên remote), thanh thông báo có nút "Commit lại" (redo = `gitCommit` cùng message).
  - `components/ChangesPane.vue`: mục "Stash thay đổi..." trong menu `...`, mở StashDialog (tên mặc định `<nhánh> <ngày giờ>`, checkbox untracked chỉ hiện khi có file untracked), thông báo + nạp lại status.
  - `i18n/en.ts`, `vi.ts` (1 import + 1 spread), `i18n/__snapshots__/keys.{en,vi}.txt` (+41 key `v112stash.*` thêm tay).
  - `apps/shell/e2e/v112-stash.cjs` (mới), `.code_index/{web-git,core-desk-preview,_meta}.md`.
- Tiêu chí: 4/4 đạt.
  1. Rust test repo tạm (9): stash push / apply / pop / drop, untracked, tìm theo hash khi list lệch, conflict, dirty, undo giữ staged + trả message, undo bị chặn khi đã push / commit đầu / merge / HEAD lệch.
  2. Vitest UI: StashMenu 5 test (đếm, list, pop gọi đúng hash + emit changed, conflict không báo thành công, drop huỷ không báo lỗi) + stash.ts 4 test.
  3. E2E: classic + next 56 PASS, 0 FAIL (stash qua dialog Changes, list ở Log, Apply giữ stash, Pop, Drop huỷ giữ / Drop ok xoá, undo ẩn ở commit cũ / commit đầu, undo + Commit lại đúng message, sau khi push backend vẫn từ chối).
  4. cargo test + typecheck pass; ảnh.
- Audit: không comment trong code, không em dash / chấm giữa trong chuỗi mới. Không polling (đếm stash chỉ nạp khi root / pane active đổi và khi mở dropdown). Mọi git chạy qua `exec_git` có timeout, không giữ lock khi chờ confirm ngoài khoá một-thao-tác-mỗi-repo. Không đụng file của 112d / 112h.
- Review: (a) Redo = commit lại cùng message trên index hiện tại; nếu user đã đổi staging thì commit gồm cả phần đó (không chặn). (b) Apply với thư mục đang bẩn trả `dirty`, không tự stash giúp. (c) Pop xung đột: git giữ stash, UI báo `conflict`, người dùng giải ở Changes. (d) Undo trong lúc merge / rebase dở chưa chặn riêng (reset --soft vẫn chạy). (e) Menu undo dựa `ahead` / `upstream` đã nạp, có thể cũ vài giây sau push ngoài; backend kiểm lại và trả `pushed` (E2E phủ). (f) Remote viewer (phone) chưa map 2 kênh mới trong `remote/viewer_map.rs`. (g) Nút OK của confirm Drop là "Continue" chung kiểu `run`.
- Test:
  - `cargo test --locked -j 4 -p core-desk stash` (apps/core) -> 9 passed; `-p core-desk preview` -> 92 passed.
  - `cargo test --locked -j 4 preview_text` (apps/shell) -> 4 passed; `cargo build -j 4` shell ok.
  - `cargo clippy -p core-desk`: không cảnh báo ở stash.
  - `npm run typecheck -w @ccm/web` -> sạch.
  - `npm test -w @ccm/web -- src/features/git src/i18n` -> git 26 file pass; i18n snapshot chỉ còn lệch key của 112h (`cmdHist.*`, `termMarks.*`), không có key `v112stash`.
  - `node apps/shell/e2e/v112-stash.cjs` (web build thư mục tạm qua `CCM_WEB_DIST`, cổng 4612 / 9612) -> 56 PASS, 0 FAIL.
  - `node scripts/index-check.mjs` -> ok 39 file.
- Nhìn: `docs/qa/v111/WP-112g/stash-dialog-{classic,next}.png`, `stash-list-{classic,next}.png`, `undo-toast-{classic,next}.png` (chụp sau lần sửa code cuối). Không ghi đè ảnh cũ.
- Hot file cần thêm: không (đã sửa `LogPane.vue`, `core/lib/preview.ts` là chủ).
- Ngoài phạm vi cần main xử lý: (1) `apps/shell/src/preview_text.rs` (không thuộc danh sách, thêm kind `stashdrop`). (2) Thêm `v112-stash` vào `E2E_MAP` của `scripts/qa-fast.mjs` (map `preview/stash.rs`, `StashMenu.vue`, `StashDialog.vue`, `LogPane.vue`). (3) Snapshot i18n đang đỏ vì key của 112h chưa được thêm. (4) TermPane.vue của 112h từng lỗi cú pháp giữa chừng khi typecheck, sau đó sạch.
- Commit message đề xuất: feat(git): stash list / apply / pop / drop and undo last unpushed commit with redo
- Dòng changelog (en / vi, ≤ 100 ký tự): en: Git Stash from Changes, stash list in Git Log, undo the last unpushed commit / vi: Stash từ Changes, danh sách stash trong Git Log, hoàn tác commit cuối chưa push
