## WP-v87-2-D: ring, replay, backpressure, census, owners, reaper, MAX_LIVE (Rust)
- Spec: `.planning/PLAN-v87-step2-core-pty.md` — §2.3 (hành vi lõi phải giống), §4.3 (Ring, Backpressure, Flush), §5 WP D + "Test từng WP", §6 cổng.
- Nền: WP-C đạt (2 vòng, 33 test pass). Báo cáo `.planning/wp/WP-v87-2-C-report.md`, kiến trúc + quyết định `.code_index/core.md` (mục "Required for WP-D"). Bản Node tham chiếu `apps/desktop/src/pty/host.ts` (ring, `bufferedAmount` pause/resume ~:111, census, owners, reaper, `MAX_LIVE`, attach / replaced 4000). Contract `packages/shared/src/ptyControl.ts`.
- Mục tiêu:
  - Ring byte cố định 256 KB mỗi session; replay khi attach bắt đầu ở đầu ký tự UTF-8; attach sau khi thoát → replay + `exit`; attach thay socket → đóng socket cũ code 4000 như Node.
  - Backpressure: đếm byte chưa gửi theo socket, > 512 KB dừng đọc PTY, < 128 KB đọc tiếp; chưa có socket vẫn đọc vào ring. Mọi channel `Out` / `Event` có giới hạn (điều kiện bắt buộc từ review C #3): RAM không tăng vô hạn khi client đọc chậm.
  - Flush leading-edge giữ như C (chunk đầu gửi ngay, gom chunk tích lại).
  - Census (`census` / `census-reply` theo contract), `owners` (process table, cache 1.5 s, sâu nhiều cấp, viết sao để thay bằng `core-procs` sau — `sysinfo` feature tối thiểu hoặc Toolhelp / `ps` tự viết, chọn cái nhỏ hơn, ghi lý do), reaper 30 phút session đã thoát không ai attach (clock giả được trong test), `MAX_LIVE` 16 → `error limit`.
- Phạm vi file: `apps/core/**`, `.code_index/core.md`. Cấm đụng: `apps/desktop/**` (WP-E1 có thể đang ở `apps/desktop/test/pty-parity/**`), `packages/**`, `.github/**`. Không xoá `target/`.
- Model: opus (concurrency, backpressure).
- Lệnh test: `cargo test --locked -j 4` + `cargo clippy --locked -j 4 --all-targets -- -D warnings` (Windows và `--target aarch64-apple-darwin`), chạy qua PowerShell.
- Tiêu chí nghiệm thu:
  - [ ] Test: ring wrap, replay đúng biên UTF-8, replay đúng đuôi khi > 256 KB, attach sau exit, replaced 4000.
  - [ ] Backpressure: child xả 8 MB với client đọc chậm → RSS core không vượt ~vài MB so với lúc đầu, không mất byte, child bị chặn rồi chạy tiếp.
  - [ ] Census, owners sâu 3 cấp, reaper bằng clock giả, MAX_LIVE 16.
  - [ ] Đo nhanh (ghi báo cáo, không phải cổng): RSS `orrery-core` khi 8 pane `cmd` / shell mở; p50 echo 1 byte qua WS (test đo, 200 lần).
  - [ ] Tests + clippy sạch; `.code_index/core.md` cập nhật.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-v87-2-D-report.md`

## Nhật ký
- 2026-11-09 sub D: đọc brief, core.md, C report, plan §2.3/§4.3/§5, host.ts + owners.ts, harness parity.
- sub D: viết `ring.rs`, `flow.rs` (Gate / Credit / Meter), `owners.rs`; nối hub / ws / server, `PtyEvent::Draining` ở pty_win + pty_unix. Clippy win sạch.
- sub D: unit 34 pass; `tests/flow.rs` 8 pass (sửa MULTI 6000 → 20000 dòng vì ConPTY nén còn 132 KB). Đo: 8 MiB stall 656 KB, RSS +1.1 MB; 8 pane cmd 9.7 MB; echo p50 10.3 ms.
- sub D: full `cargo test` 63 pass; clippy win + aarch64-apple-darwin sạch; parity FULL 34 pass / 1 skip ×2. Cập nhật `.code_index/core.md`, viết báo cáo.
- sub D vòng 2: 4001 overflow + gỡ meter, ps deadline 15 s, attach xoá conn của session cũ; 3 test mới; cargo test 66 pass, clippy win + mac sạch, parity FULL 34/1 skip.
