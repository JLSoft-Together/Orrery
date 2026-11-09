# HANDOFF — Orrery v87 trên MacBook (2026-11-10)

Thay `.planning/HANDOFF-2026-11-09-2035-v87.md` cho phần mac. Nhật ký: `.planning/PROGRESS-2026-11-09-v87.md` (cuối file). Kết quả mac: `.planning/QA-v87-mac-result.md`. Ràng buộc chung giữ như §4 handoff 2035 (commit + push luôn khi WP đạt, không trailer, không release, máy ≤ 80 %).

## Đã xong trên mac (đã push)
- Rust 1.99.1 + `npm install`. Sửa lỗi chỉ có trên mac: thứ tự `read_dir` (parity), realpath roots (4 module desktop), clippy `kinfo.rs`, log E2E tách khỏi `~/Library/Logs/Orrery`, tên process cổng với lệnh pipe (`tree.ts` `mainOf`).
- QA 12 bước đạt (Codex sau khi chủ nhân trust folder).
- WP-R2: `CCM_CORE_USAGE` / `CCM_CORE_TRANSCRIPT` mặc định rust, lui Node + notification 1 lần, Diagnostics `core readers:`; `js_ms` sửa lệch 1 ms startedAt.
- T4: CPU −35 %, cold scan 0.42 vs 1.24 CPU-s, E2E chạy được trên mac (`cdp.cjs` reload + `macKey`).

## Việc tiếp
1. T5: Codex rollout + DeepSeek outbox + hooks / statusline sang Rust (`.planning/PLAN-v87-step4-core-transcript.md` §3).
2. (Tuỳ) port nốt E2E còn fail trên mac: danh sách ở `QA-v87-mac-result.md` mục T4.
3. Windows: đo lại cổng sau R2 (mặc định rust) để xác nhận 7.6 % trên Windows.
