# WP-v87-3 leak — PARTIAL

| Lượt | Số process chữ ký còn lại sau lượt |
|---|---:|
| Node 1–3 | 0, 0, 0 |
| Rust 1–3 | 0, 0, 0 |
| Huỷ Vitest PID 25548 | 0 (đã thấy PID child 24796 khi chạy; sau huỷ không còn) |
| Cô lập `cwd/branch`, `live limit 16` | 0, 0 |

- Scenario gây sót: chưa xác định; không scenario nào để lại process trong các lượt đã chạy.
- Nguyên nhân 48 process ngày 2026-11-09 chưa tái hiện. `Target.stop()` chỉ đợi/kết thúc host; các lượt thường và lượt huỷ này đều dọn hết child.
- Đề xuất nếu tái diễn: kiểm tra cleanup ở `apps/desktop/test/pty-parity/harness.ts:147` và `apps/desktop/src/pty/host.ts:378`; đối chiếu `apps/core/crates/core-pty/src/jobs/windows.rs:29` (Job Object Windows).
- Đã thêm callback ghi PID child và test hồi quy kiểm tra PID scenario `kill` đã thoát sau teardown: `apps/desktop/test/pty-parity/harness.ts`, `parity.test.ts`.
- Test: Node parity 3 lượt ban đầu (17 pass mỗi lượt) + 1 lượt sau sửa test (18 pass); Rust parity 3 lượt (34 pass / 1 skip mỗi lượt); focused regression + 2 scenario cô lập đều pass.
- Dọn dẹp: số process khớp chữ ký cuối cùng = 0.
STATUS: PARTIAL
