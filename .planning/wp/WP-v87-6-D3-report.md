# WP-v87-6-D3 — báo cáo (2026-11-10)

Port `apps/desktop/src/preview/` sang `core-desk`: xong 57 / 86 kênh, chia 3 nhóm cờ, mặc định `rust`. 29 kênh còn TS (danh sách cuối).

## Đã làm

1. **Nhóm `previewGit`** (`CCM_DESK_PREVIEW_GIT`, 40 kênh): `changes-status|diff|file|discard`, `git-show-image`, `log-refs|page|commit|contains|diff`, `git-branch-create|delete|update`, `git-fetch`, `git-push-preview`, `git-push`, `gh-repo-account(-set)`, `git-stage|unstage|can-amend|commit|commit-cancel|blame|merge-state|merge-abort`, `wt-compare|diff|file-diff|take|merge`, `overlap-files|dryrun`, `cp-state|set|take|diff|file-diff|revert-hunk|restore`.
2. **Nhóm `previewFiles`** (`CCM_DESK_PREVIEW_FILES`, 13 kênh): `read-file`, `log-read`, `file-write` (+ Local History), `lh-list|read`, `list-dir`, `create|rename|trash-entry`, `search` (git grep), `commands-list`, `notes-read|write`.
3. **Nhóm `previewTools`** (`CCM_DESK_PREVIEW_TOOLS`, 4 kênh): `env-check`, `env-reveal`, `i18n-scan`, `scripts`.
4. **Cổng (gate) vẫn ở vỏ trong 6a**: map repo / grants / full access / roots của pane là trạng thái dùng chung với kênh còn TS và với `PreviewIpc.resolveDir|resolveFile|grant` mà module khác gọi. Rust hỏi vỏ qua `Ctx::host` (`preview/gate.rs`), vỏ trả bằng `previewHosts` (`apps/desktop/src/preview/deskHosts.ts`). Vì vậy cờ từng nhóm bật / tắt độc lập mà không lệch trạng thái.
5. Huỷ lượt cũ (search / blame / overlap-dryrun theo sender, commit theo repo) và `busy` (wt-merge, checkpoint) làm lại trong Rust (`gate::Latest`, `gate::Busy`, `CheckpointStore` busy set).
6. Helper riêng của gói: `preview/exec.rs` (= `gitExec.ts`: `-c` config + `GIT_LITERAL_PATHSPECS`, cap stdout → `truncated` + code 0, timeout + kill tree (`taskkill /T` / process group), stdin, cờ huỷ, binary), `preview/paths.rs` (`safe_repo_path`, `is_within`, `root_key`, realpath bỏ tiền tố `\\?\`). Dùng lại `git::git_path` (đổi thành `pub`), `js::{len,slice,entries,number,stringify,parse}`, `util::{resolve,basename,mtime_ms,now_ms,iso_string}`, `fs_tree::read_names`.

## File

- Mới (Rust): `apps/core/crates/core-desk/src/preview/{mod,exec,paths,gate,changes,log,branches,commit,gitmisc,worktrees,checkpoints,files,history,explorer,search,envcheck,scripts,i18n,i18n_parse,ipc_git,ipc_wt,ipc_files,ipc_tools}.rs` (mọi file ≤ ~420 dòng).
- Sửa (Rust, sửa nhỏ): `core-desk/src/lib.rs` (`pub mod preview` + `preview::register`), `core-desk/src/git.rs` (`git_path` → `pub`), `core-desk/Cargo.toml` (`sha1 = "0.11"`, đã có sẵn trong `Cargo.lock` qua tungstenite).
- Mới (TS): `apps/desktop/src/preview/deskHosts.ts`, test `apps/desktop/test/desk-preview-{git,files,tools}.twin.test.ts`.
- Sửa (TS): `apps/desktop/src/preview/ipc.ts` (deps `desk`, object `gate`, 3 handle `git` / `files` / `tools` thay `ipcMain.handle` cho 57 kênh; logic TS giữ nguyên làm fallback), `src/desk/forward.ts` (`DESK_MODULES` += `previewGit`, `previewFiles`, `previewTools`), `src/main.ts` (`registerPreviewIpc({… desk: deskRouter ?? undefined})`).
- `.code_index/{core,desktop,_meta}.md`.
- Không đụng file của D1 / T1.

## Số test thật

- `cargo test -j 4 -p core-desk` (lúc chốt, gồm cả module D1): 37 passed, 0 failed; riêng preview 13 unit test.
- `cargo clippy -j 4 -p core-desk --all-targets -- -D warnings`: sạch (lần chạy sau khi sửa 4 lỗi của gói).
- Twin TS↔Rust (`orrery-core desk` thật, 2 instance `registerPreviewIpc`, cùng input trên 2 bản sao fixture, `swapRoot` + so cả file trên đĩa, confirm, trash, `twin.logs` / `fellBack` rỗng):
  - `desk-preview-git.twin.test.ts` 5/5 (repo thật + bare remote, ngày commit cố định nên hash trùng: status, diff full / whitespace / untracked / binary, ảnh HEAD, file gutter, refs, log page (hash, author, path, regex), commit detail / contains / diff (rename, full), blame, merge state / abort, stage / hunk / unstage, commit / amend / nothing / empty, discard file / hunk / untracked (trash), branch create / push preview / push / fetch / update / delete (local + remote, có confirm), gh account; worktree compare / diff / file diff / take / merge squash / ff / keep, overlap files / dry-run, checkpoint on / take / diff / file diff / revert hunk / restore / off).
  - `desk-preview-files.twin.test.ts` 4/4 (read-file text / CRLF / BOM / UTF-8 hỏng / binary / ảnh / Dockerfile / missing / denied; log-read lùi / tiến / reset; file-write conflict / CRLF / BOM / encoding / denied / missing + so cây file; Local History list / read; notes; list-dir / create / rename (đổi hoa thường) / trash (cả lỗi EBUSY); search 4 kiểu + bad-query + not-repo; commands-list project + user).
  - `desk-preview-tools.twin.test.ts` 3/3 (env check / reveal, i18n Android + Flutter ARB + JSON locales, scripts 5 nguồn).
- Tổng twin preview: 3 file, 12/12 pass. Không còn process `orrery-core` nào tự spawn.
- Typecheck desktop: 0 lỗi.

## E2E (instance cô lập: `CCM_E2E_PORT=4501`, `CCM_E2E_DBG=9501`, `CCM_E2E_OUT=%TEMP%\ccm-e2e-d3`, `CCM_CORE_BIN` = bản release copy riêng; build `node apps/desktop/build.mjs` + `cargo build --release -j 4 -p core-bin`; bản build gồm nhóm git + files)

| Bộ | PASS | FAIL |
|---|---|---|
| diff | 30 | 0 |
| log | 27 | 0 |
| split-diff | 29 | 0 |
| push | 14 | 0 |
| explorer | 33 | 0 |
| explorer-menu | 11 | 0 |
| file-editor | 22 | 0 |
| v67-ide | 10 | 0 |
| v69-ide | 22 | 0 |
| v70-tools | 40 | 1 |
| v72 | 41 | 0 |
| v73 | 45 | 0 |

`desk.log`: `orrery-core desk ready`, không dòng fallback. Lỗi v70 `json tree — timeout: menu tree` là bước UI mở menu More (máy lúc đó CPU ~90 %), chưa kịp chạy lại với `CCM_DESK_PREVIEW_*=node` để so. Nhóm `previewTools` chưa qua E2E (chỉ twin).

## Callback host mới (vỏ Tauri phải làm theo)

`preview.repoOf {root}` · `preview.registerRepo {dir}` → path | `{error}` · `preview.addRepo {root, from}` · `preview.allowed {path, fresh}` → `{real, roots}` | null · `preview.roots` · `preview.writeRoots` · `preview.editableFile {file}` → `{real, root}` | null · `preview.explorerRoot {root}` · `preview.checkDir {dir}` · `preview.caps` → `{trash, confirmGit, confirmRun, confirmReplace, confirmPermRules, confirmDenyRules, github, history, claudeConfigDir}` · `preview.sender` (id trang; Tauri 1 cửa sổ → hằng số) · `preview.invalidate {root}` (cache list-files) · `preview.trash {path}` → true | `{error}` · `preview.confirm {kind: git|run|replace|perm|deny, req}` (req giữ nguyên object TS, vỏ dựng câu i18n) · `preview.ghEnv {repo, url}` · `preview.ghResolve {repo, url}` · `preview.ghSetChoice {repo, choice}`.

Ở Tauri nên chuyển phần cổng (`paths.ts` + `grants.ts` + map repo trong `ipc.ts`, ~250 dòng) vào Rust cùng lúc với các kênh cổng còn lại, khi đó các callback trên thành gọi hàm trong process.

## Còn TS (29 kênh) + lý do

- Việc của vỏ (giữ TS / Tauri tự làm): `<webview>` hardening (`will-attach-webview`, `did-attach-webview`, `hardenGuest`, `lockPreviewSession`), `clipboard-image`, `clipboard-image-file`, `capture-guest`, `pick-file`, `pick-files`, `open-external`, `reveal`, `open-path`.
- Trạng thái cổng + confirm native: `request-access`, `full-access`, `forget-grants`, `project-root` (ghi map repo).
- Theo dõi file / server, gửi event tới đúng webContents: `watch`, `unwatch`, `changes-watch`, `changes-unwatch` (`RootWatcher` + `GitDirWatcher`), `preview-url` (`PreviewServer` HTTP), `list-files` (cache bị watcher xoá).
- Chưa port (nhóm tools, chia sau): `replace-files` (regex JS do trang gửi: lookbehind, cờ `u` / `i` — `regex` crate không khớp; cần engine tương thích JS như `regress`), `format` (spawn formatter), `check-run` / `check-cancel` (spawn script), `perm-read` / `perm-allow`, `sensitive-deny`, `ctx-sources`, `memory-doctor` (dùng `extensions/status` của D1), `http-send` / `http-cancel` (`httpSend.ts`, mạng + confirm theo host).

## Bước tiếp theo

1. Chạy lại `v70-tools` 2 lần (rust / `CCM_DESK_PREVIEW_GIT=node CCM_DESK_PREVIEW_FILES=node CCM_DESK_PREVIEW_TOOLS=node`) để xác nhận lỗi `json tree` không do desk; build release mới (có nhóm tools) rồi chạy bộ E2E có env-check / i18n / scripts (rg `ccm:env-check|ccm:i18n-scan|ccm:scripts` trong `apps/desktop/e2e`).
2. D3b: port `format`, `check-run/cancel`, `perm-read/allow`, `sensitive-deny`, `ctx-sources`, `memory-doctor` (sau khi D1 xong `extensions/status`), `http-send/cancel` vào `previewTools`; `replace-files` với crate `regress`.
3. Trước Tauri: chuyển cổng (repos / grants / full access / roots) + `watch`/`changes-watch`/`list-files`/`preview-url`/`project-root`/`request-access` sang Rust, bỏ các callback `preview.*` tương ứng.

## Khác biệt còn lại

- Regex: mọi `\d` / `\w` port từ JS viết thành `[0-9]` / `[A-Za-z0-9_]`; `\s`, `\b`, `(?i)` của Rust theo Unicode (JS: `\b` ASCII, `i` không `u`) — chỉ lệch với tên / text không phải ASCII ở `isWordy`, recipe `just`.
- `localeCompare` (sort nhóm remote, file `.env`) dùng so chữ thường rồi so byte, không phải ICU.
- `file-write` ghi Local History đồng bộ trước khi trả lời (TS ghi nền, không chờ).
- Rename đổi hoa thường: TS so `ino` / `dev` (bigint), Rust so realpath của 2 đường dẫn.
- `exec.rs` khi `truncated`: đọc tiếp tới EOF sau khi kill; nếu git con giữ pipe (ví dụ `git-remote-https`) thì join có thể chờ tới khi nó thoát.
- `JSON.parse` chuỗi có surrogate lẻ (i18n JSON / ARB) vẫn khác như D0.
- macOS: code theo `cfg` (`process_group`, `libc::kill`, mã lỗi errno), chưa chạy trên Mac.

STATUS: PARTIAL
