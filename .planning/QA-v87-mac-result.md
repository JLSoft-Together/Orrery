# Kết quả QA v87 trên macOS (2026-11-09)

Máy: MacBook (Darwin 27.0.0, 10 core, 24 GB), Rust 1.99.1 (rustup cài mới), Node v26.12.1.

## Test tự động

| Bộ | Trước sửa | Sau sửa |
|---|---|---|
| `cargo test` (apps/core) | 1 fail: `core-transcript` `dedupes_globally_and_keeps_resume_copies_after_delete` | pass hết |
| collector | 11 fail (parity tracker Node ↔ Rust + recovery) | 465 pass / 3 skip |
| desktop | 8 fail (5 file) | 1515 pass / 27 skip, typecheck sạch |

## Lỗi chỉ có trên mac đã sửa

1. Thứ tự đọc thư mục: Rust `std::fs::read_dir` trên APFS trả thứ tự thô, Node (libuv `scandir`) sort theo byte trên unix. Lệch parity thật: message id trùng giữa 2 file usage gán khác file; thứ tự subagent trong tracker khác Node. Sửa: `read_dir_sorted` trong `jobs.rs` (unix sort byte, Windows giữ thứ tự OS như libuv), dùng cho `list_jsonl`, `find_transcript`, `discover`.
2. Symlink `/tmp`, `/var` → `/private/...`: so đường dẫn đã realpath với root chưa resolve → `denied` / `missing` sai. Sửa trong code (người dùng mac thật có thể gặp): `permRules.ts` `writeAllow`, `agentStatus/watch.ts` `readStatus`, `fileWrite.ts` `writeTextFile` (realpath roots bên trong), `logRead.ts` (file không tồn tại: so thêm root gốc để trả `missing`). Kiểm containment giữ nguyên.
3. `shim.test.ts`: đường dẫn `C:\` chỉ assert trên win32 (lỗi test).

## E2E trên mac (instance cô lập, port 4399)

- Sửa để chạy được trên mac: electron path qua `require('electron')` (`cdp.cjs`, `desktop-only.cjs`, `v49-open.cjs`); `core-pty.cjs` unix (echo, realpath cwd, `ps -axww` + args, SIGKILL, chặn `process.kill(0)` vốn giết cả process group khi không thấy core).
- `main.ts`: có `--user-data-dir` thì log vào `<userData>/logs` (trước đó E2E trên mac ghi đè `~/Library/Logs/Orrery/pty.log` của app thật).
- `core-pty`: rust 22/0, `CCM_CORE_PTY=node` 17/0, fallback (core thiếu) 18/0 — gồm core crash → pane chết hết → core mới, không process mồ côi.
- `tabs-cwd` 14/0, `shell-agent` 5/0.
- Script chỉ viết cho Windows (chưa port): p2 / v49-open / v80-mcp gõ lệnh PowerShell, p3 phím Alt, p4–p6 chờ `[data-launch-row]` (timeout picker, chưa điều tra).
- `cargo clippy -D warnings`: `kinfo.rs` đổi sang `as_chunks`, sạch.
- `cargo fmt --check` lệch cả repo (không dùng rustfmt) — bỏ qua.
- QA tay 12 bước trong `QA-v87-mac.md`: chờ chủ nhân chạy `npm run desktop`.

## QA 12 bước (con tự chạy qua CDP, instance riêng, 2026-11-10)

| Bước | Kết quả |
|---|---|
| 1 terminal host | `pty host: rust (…/apps/core/target/release/orrery-core)` |
| 2 shell, `ls`, `vim` rồi `:q` | ĐẠT (ảnh: màu, tiếng Việt, thoát vim pane vẫn chạy) |
| 3 Claude | ĐẠT: prompt ngắn → "mac ok", status idle → working → idle, link Monitor |
| 3 Codex | ĐẠT (sau khi chủ nhân trust folder): prompt ngắn → "codex ok"; pane Codex chạy độc lập không có dòng riêng trong Monitor (thiết kế: Codex hiện dưới agent Claude đã gọi nó) |
| 4 resize, 5 cd / branch, 6 đóng pane, 8 `--resume`, 9 kill core | ĐẠT qua E2E core-pty / p3 / p5 / tabs-cwd |
| 7 Quit | ĐẠT: sheet "Quit and close 2 terminals?" đủ 2 dòng; Cancel giữ app + pane; Quit → app, pane, `orrery-core` tắt hết |
| 10 Monitor | ĐẠT: session Claude của pane hiện, link đúng |
| 11 Usage | ĐẠT: hiện số trong ~6 s lần mở đầu, không treo |
| 12 Ports | ĐẠT sau sửa: cổng 8765 có dòng; lỗi tên process `cut` → sửa `tree.ts` `mainOf` ưu tiên process đang listen (lệnh pipe kiểu `npm run dev \| tee` lấy nhầm tên) |
