## WP-v87-R1: bản trung gian — PTY mặc định Rust, tự rơi về Node có báo
- Bối cảnh: chủ nhân 2026-11-09 "cứ làm đi" — release trung gian: pty / usage / transcript mặc định Rust (usage + transcript đổi ở WP-R2 sau T3b), mac chủ nhân tự test tối nay. Plan pty: `.planning/PLAN-v87-step2-core-pty.md` §6 (cổng WP-F), §8 O1. Code: `apps/desktop/src/pty/supervisor.ts` (`PtyHostKind`, `RustHost`, chỗ chọn host theo `CCM_CORE_PTY`), `apps/desktop/build.mjs` (build core), E2E `apps/desktop/e2e/core-pty.cjs`.
- Việc:
  1. Mặc định `CCM_CORE_PTY` = `rust`; `CCM_CORE_PTY=node` vẫn chọn Node.
  2. O1 (main chốt thay chủ nhân, chủ nhân bảo "cứ làm"): Rust không khởi động được (thiếu binary, spawn lỗi, không `listening` trong deadline, crash lặp lại khi chưa có pane nào) → tự dùng Node host cho phiên đó và **báo rõ 1 lần** (dùng cơ chế thông báo có sẵn của app: notification native / toast / log `[ccm]`, chọn cái đã có, không thêm UI mới lớn). Không im lặng. Crash core khi đã có pane đang chạy: giữ hành vi hiện tại (pane báo thoát), lần mở pane sau mới xét rơi về Node; ghi rõ vào plan.
  3. Kill cây sạch khi quit / khi core crash (đã có test? kiểm, thiếu thì thêm).
  4. Cập nhật `CLAUDE.md` mục env (`CCM_CORE_PTY=node` để dùng node-pty), `.code_index/desktop.md`, plan pty §6 ghi "đổi mặc định 2026-11-09, chờ soak macOS".
  5. Viết checklist test macOS cho chủ nhân `.planning/QA-v87-mac.md`: cách lấy bản (CI artifact workflow `desktop` hoặc `npm run desktop` từ source trên mac: `git pull`, `npm install`, cần Rust toolchain `rustup`), các bước kiểm (mở pane shell / Claude / Codex, gõ, resize, `cd` OSC 7, đóng pane, quit không sót process `ps aux | rg orrery-core`, `--resume`, Monitor hiện session + subagent đúng, usage heatmap, port / process panel), cách tắt từng cờ nếu lỗi (`CCM_CORE_PTY=node`, `CCM_CORE_USAGE=node`, `CCM_CORE_TRANSCRIPT=node`), log cần gửi lại. Tiếng Việt, ngắn.
- Phạm vi: `apps/desktop/**` (không đụng `apps/desktop/test/__snapshots__/fwp-host.test.ts.snap` line-ending), `CLAUDE.md` (chỉ dòng env), `.code_index/desktop.md`, `.planning/PLAN-v87-step2-core-pty.md`, `.planning/QA-v87-mac.md`. Cấm `apps/collector/**`, `apps/core/**` (WP-T3b đang sửa), `apps/web/**` trừ khi bắt buộc cho thông báo (ghi lý do).
- Model: sonnet.
- Lệnh test: `npm test -w @ccm/desktop -- --maxWorkers=4`, `npm run typecheck -w @ccm/desktop`; E2E được phép trên instance cô lập: build stage `node apps/desktop/build.mjs` rồi `node apps/desktop/e2e/core-pty.cjs` (mặc định giờ là rust) + 1 lượt `CCM_CORE_PTY=node`; thử rơi về Node bằng `CCM_CORE_BIN` trỏ file không tồn tại.
- An toàn: KHÔNG chạm app Orrery đang chạy (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); kill chỉ pid tự spawn, không `taskkill /IM`; máy ≤ 80 %. Không `dist:*`.
- Tiêu chí:
  - [ ] Mặc định rust, `node` opt-out chạy.
  - [ ] Rơi về Node + báo 1 lần khi core hỏng (có test).
  - [ ] E2E rust + node pass; desktop test + typecheck sạch.
  - [ ] `QA-v87-mac.md` có đủ bước + cách tắt cờ.
- Nhìn thấy được: có (thông báo rơi về Node) — chụp nếu chạy được E2E, không thì CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-R1-report.md`

## Nhật ký
- 20:22 supervisor fallback + main default rust + strings + tests viết xong, chạy test
- 20:23 unit 130 files/1524 pass, typecheck sạch; sang e2e
- 20:24 e2e.cjs thêm FALLBACK mode, stage build xong, chạy e2e
- 20:25 e2e rust default 24 PASS
- 20:30 e2e rust/node/fallback chạy, docs + QA-v87-mac.md xong, ghi report
