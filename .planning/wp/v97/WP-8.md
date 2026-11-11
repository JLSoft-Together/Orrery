## WP-8: Gateway — viewer (Explorer, File, Changes / Git, Git Log + branch, Run) cho điện thoại
- Mục tiêu: điện thoại gọi được các viewer qua lệnh `fs.*`, `git.*`, `log.*`, `branch.*`, `run.*` (`docs/remote-integration.md` §4.5), đi đúng cổng preview của desktop (chỉ path nằm trong root của pane sống / grant), nhận event `fs.changed`, `git.changed`, `run.event` riêng cho thiết bị đã watch; thao tác nguy hiểm hỏi trên điện thoại.
- Spec item: PLAN-v97-mobile.md §3 dòng viewer, bất biến 5, 6; review #10; `.planning/research/v97-rust-side.md` §5 (bảng kênh + chỗ code), §6 (confirm); `docs/remote-integration.md` §4.5, §4.6.
- Phạm vi file: `apps/shell/src/remote/**` (thêm `viewer.rs`, `viewer_map.rs` (bảng lệnh → kênh + whitelist), `viewer_events.rs`, test), nối dây tối thiểu: `apps/shell/src/desk.rs` (`emit_app` thêm sink gateway), `apps/shell/src/preview_ipc.rs` / `preview_watch.rs` (`Subs` nhận owner không phải webview: sink theo `remote-<device>`), `apps/shell/src/hosts.rs` / `preview_host.rs` (`preview.confirm` khi lệnh từ điện thoại → hỏi điện thoại qua confirm router WP-6, không mở hộp native), `apps/shell/src/host_run.rs` / `host_telegram.rs` (dùng lại bộ gom log run 200 dòng nếu tiện), `.code_index/shell.md`, `docs/remote-integration.md` §4.5 (cập nhật shape thật). | Cấm đụng: `apps/core/**` (kênh core-desk dùng như có; cần đổi → "Ngoài phạm vi"), `apps/web/**`, `apps/mobile/**` (WP-11b đang sửa).
- Phụ thuộc: WP-5, WP-6 (confirm router, session phone). | Song song với: WP-11b | Worktree: không.
- Model: sonnet (main giao opus review sau).

### Thiết kế
- Bảng whitelist cố định lệnh → kênh `ccm:*` (KHÔNG chuyển tên kênh tuỳ ý từ điện thoại): `fs.list`→`ccm:list-dir`, `fs.files`→`ccm:list-files`, `fs.read`→`ccm:read-file` (max 2 MiB, nhị phân → `{binary:true}`), `fs.write`→`ccm:file-write` (20 / phút), `fs.watch|unwatch`→`ccm:watch|unwatch`, `git.status|diff|file`→`ccm:changes-status|diff|file`, `git.stage|unstage|commit|discard`→`ccm:git-stage|git-unstage|git-commit|changes-discard`, `log.refs|page|commit|diff`→`ccm:log-refs|log-page|log-commit|log-diff`, `branch.create|delete|update|fetch|push`→`ccm:git-branch-create|git-branch-delete|git-branch-update|git-fetch|git-push` (+ `git-push-preview`), `run.configs|start|stop|reload|sessions`→`ccm:run-*` tương ứng. Không có `grant-folder`, `pick-*`, `open-path`, `reveal`, `request-access`, `full-access`, `forget-grants`.
- Gọi bằng `desk::call_inside` / `preview_ipc::dispatch(app, "remote-<deviceId>", …)` trong `spawn_blocking`, timeout 15 s (git fetch / push 60 s). Mọi path phải tuyệt đối và qua `preview.allowed` / `explorerRoot` như desktop.
- Event: watch theo (device, root) → sink gửi `fs.changed {root, paths}` / `git.changed {root}` chỉ tới thiết bị đó; phiên đóng → unwatch hết. `run.event` → tới mọi thiết bị đã gọi `run.sessions` / `run.start` trong phiên (subscribe ngầm), gộp ≤ 100 ms.
- Confirm: `preview.confirm` / `hosts::confirm` có biến ngữ cảnh "lệnh từ thiết bị X" (thread-local / task-local hoặc tham số) → dùng confirm router WP-6 hỏi đúng thiết bị đó (60 s), không mở hộp native, không `show_main`.
- Rate limit chung viewer 120 lệnh / phút / thiết bị.

### Tiêu chí nghiệm thu
- [ ] `cargo test --locked -j 3 remote` + toàn shell pass; test mới: lệnh ngoài bảng → `denied`; path ngoài root pane sống → `denied`; `fs.read` file trong repo test → nội dung; `fs.write` + đọc lại; watch → sửa file → đúng thiết bị nhận `fs.changed`, thiết bị khác không; phiên đóng → watcher được nhả; `branch.delete` → `confirm.ask` tới thiết bị gửi, không hộp native (kiểm bằng `CCM_E2E_CONFIRM` không bị gọi / log confirm); `run.*` tối thiểu `configs` trả list.
- [ ] Không comment; file ≤ ~400 dòng; clippy `remote/` sạch; `.code_index/shell.md` + `docs/remote-integration.md` §4.5 cập nhật.

### Lệnh test
`cd apps/shell && cargo test --locked -j 3 remote && cargo test --locked -j 3`

- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/v97/WP-8-report.md`

### Nhật ký

- 2026-11-12 sonnet: viewer_map/viewer/viewer_events/scope + nối dây xong; remote 51 pass, shell 245 pass, clippy chỉ 8 lỗi có sẵn; docs §4.5 + shell.md cập nhật.
