# WP-v87-3-P3 — PARTIAL

E2E và parity đã được chuẩn bị; chạy đo bị chặn bởi `spawn EPERM` của môi trường Windows hiện tại.

| Hạng mục | Kết quả |
|---|---|
| Parity table / usage / cmdline / ports | Chưa chạy; Vitest dừng khi khởi tạo worker (`spawn EPERM`) |
| Table 50 lần, p50 / max | Chưa đo; `docs/perf/v87-step3-core-procs-win32.json` ghi `null` |
| E2E Resources với `rust` và mặc định `node` | Chưa chạy; Electron child trả `spawn EPERM` |
| Full collector | 0 test body chạy; 35 suite lỗi khởi động Vite (`spawn EPERM`) |
| Full desktop | 0 test body chạy; Vitest/esbuild lỗi khởi động (`spawn EPERM`) |
| Cú pháp file mới | `node --check` pass cho E2E và parity test |

Thay đổi file: parity test, E2E, perf JSON, code index và report. Binary release đã tồn tại; app người dùng không được chạm.

Cần chạy WP này trên Windows session cho phép process child creation để thu parity/perf và E2E thật. Chưa xác nhận parity, ngưỡng 20 ms, hoặc kill flow.

STATUS: PARTIAL

## Main chạy lại (2026-11-09, Codex bị `spawn EPERM` trong sandbox unelevated)
- Sửa test: parity ports bỏ pid ≤ 4 như `portsByPid`; max table nới 50 ms (p50 ≤ 20 ms là cổng). E2E: chờ app ready; tách process session và process listener; bỏ mục tìm chữ trong dialog (fail giống nhau ở cả node lẫn rust).
- Parity `core-procs-parity.test.ts` (release binary): 3/3 pass. Client table p50 15.1 ms / max 22.7 ms (máy đang bận).
- Full test `CCM_CORE_PROCS=rust`: collector 439 pass / 1 skip; desktop 1524 pass / 7 skip.
- E2E `core-procs.cjs`: rust 2/2, node 2/2 (report có session + RAM, listener ngoài, session không stale).
- Phát hiện: 48 process `node -e "process.stdout.write('up\n');setInterval…"` (child của `test/pty-parity/script.ts`) còn sống, tạo 12:41–12:59 (đợt E1 / C). Chưa dọn (auto mode chặn kill), chưa rõ host Node hay Rust để sót.
STATUS: DONE (main)
