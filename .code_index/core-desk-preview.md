# core-desk-preview: files, Git, search, gate

Đường vào: `apps/core/crates/core-desk/src/preview/mod.rs` (`register`). Ba nhóm kênh: Git (`ipc_git.rs`, `ipc_wt.rs`), file (`ipc_files.rs`), công cụ (`ipc_tools.rs`).

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-desk/src/preview/gate.rs` | quyền đường dẫn: root đã grant, realpath, chặn `..` / symlink ra ngoài |
| `apps/core/crates/core-desk/src/preview/changes.rs` | trạng thái Changes (stage / discard) |
| `apps/core/crates/core-desk/src/preview/commit.rs` | commit, amend, hook |
| `apps/core/crates/core-desk/src/preview/stash.rs` | `ccm:git-stash` (list/push/apply/pop/drop, tìm theo hash, `GIT_LITERAL_PATHSPECS=0` vì stash -u hỏng khi bật) + `ccm:git-undo-commit` (`reset --soft HEAD~1`, chặn root / merge / đã push / HEAD lệch) |
| `apps/core/crates/core-desk/src/preview/log.rs` | Git Log (đồ thị, refs, filter) |
| `apps/core/crates/core-desk/src/preview/branches.rs` | tạo / fast-forward / fetch / xoá nhánh, push range |
| `apps/core/crates/core-desk/src/preview/worktrees.rs` | worktree |
| `apps/core/crates/core-desk/src/preview/checkpoints.rs` | checkpoint theo turn |
| `apps/core/crates/core-desk/src/preview/history.rs` | Local History |
| `apps/core/crates/core-desk/src/preview/explorer.rs` | cây Explorer, list-dir |
| `apps/core/crates/core-desk/src/preview/search.rs` | tìm trong file, tìm session |
| `apps/core/crates/core-desk/src/preview/secrets.rs` | `scan_staged`, `scan_push` (chặn lộ token khi commit / push) |
| `apps/core/crates/core-desk/src/preview/deny.rs`, `perm.rs` | quyền deny / permission advisor |
| `apps/core/crates/core-desk/src/preview/i18n.rs` | quét chuỗi i18n dự án |
| `apps/core/crates/core-desk/src/preview/envcheck.rs`, `scripts.rs`, `checks.rs` | .env, Scripts, kiểm dự án |
| `apps/core/crates/core-desk/src/preview/memdoctor.rs`, `memlint.rs` | kiểm file memory / CLAUDE.md |
| `apps/core/crates/core-desk/src/preview/exec.rs`, `spawn.rs` | chạy git / lệnh ngoài có timeout |

Phía vỏ: `apps/shell/src/preview_host.rs` (callback `preview.*`), `preview_grants.rs` (`GrantStore`), `preview_serve.rs` (server file ảnh `http://127.0.0.1:<port>/<token>/<rel>`), `files_ipc.rs` (`ccm:preview-url`, `ccm:open-path`, `ccm:reveal`), `preview_watch.rs`.

## Bất biến

- Thiếu arm `preview.*` ở `apps/shell/src/hosts.rs` thì list-dir / Git / search trả null hoặc `denied` (lỗi Explorer rỗng 4.0.x).
- Xoá nhánh, push, ghi file ngoài root đều cần xác nhận / grant; push dùng tài khoản GitHub của repo, token không về trang.
- File thực thi bị chặn mở (`EXECUTABLE_EXT` trong `files_ipc.rs`).
- Watcher bỏ `node_modules`, `.git`, `build`, `.gradle`, `.dart_tool`, `dist`; debounce 150 ms.
- Worktree config (v111 G2): `preview/wtconfig.rs` đọc `.orrery/worktree.json` (`{copy?, setup?, cleanup?}`, field sai kiểu bị bỏ và báo trong `errors`), hash duyệt lưu `<data>/worktree-approved.json` theo repo + loại, `cleanup_target` chặn dirty / main / unknown, `run_cleanup` timeout 120 s kill tree. Kênh `ccm:wt-config` / `wt-approve` / `wt-suggest` / `wt-cleanup` ở `ipc_wt.rs`; native confirm là `run` kind `wtsetup` / `wtcleanup` / `wtconfig` (`apps/shell/src/preview_text.rs`).
- Shell `worktree.rs`: `wt.add` ưu tiên `copy` của file và trả `setup: true`; `wt.remove` có `deleteBranch` chạy `git branch -d` (không `-D`), trả `branch: deleted | kept`.

- Checklist trước merge (v111 G3): `preview/checklist.rs` lưu `<data>/checklists/<sha256 16 hex của đường dẫn worktree đã chuẩn hoá>.json` `{items:[{id,text,done,by,at}], check:{script,state,at,head}, seq}`; ghi nguyên tử (tmp + rename), file hỏng / quá 256 KB đọc như rỗng, tối đa 50 việc, text 1 dòng ≤ 200. Kênh `ccm:checklist` (`ipc_wt.rs`): `dir` là pane đã duyệt, `wt` phải nằm trong `git worktree list` của repo đó (`worktree_of`), op list / add / toggle / remove / setScript / setCheck; UI luôn là `owner`. `head` trả về là HEAD hiện tại, UI so với `check.head` để báo "cũ". Data dir cho MCP lấy qua `checklist::set_data_dir` (gọi ở `preview::register`).
- Shell `worktree.rs`: `wt.remove` nhận thêm `squashed`: `git branch -d` từ chối thì chỉ khi `git diff --quiet <main> <branch>` rỗng mới `-D`; còn khác thì giữ nhánh như cũ.

## Liên kết

- Web: `web-git.md`, `web-files.md`. Plan: `.planning/PLAN-v30-git-log.md`, `PLAN-v109-*.md` (secret guard). E2E: `log`, `split-diff`, `explorer`, `v109-secret-guard`.
