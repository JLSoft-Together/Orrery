## WP-v87-4-T3b: giảm CPU tổng (collector + core) ở chế độ Rust
- Bối cảnh: T3 (`.planning/wp/WP-v87-4-T3-report.md`, `docs/perf/v87-step4-T3-win32.json`): demo 30×6 seed 8704, `CCM_CORE_TRANSCRIPT=rust` → collector 7.0 / 8.3 % + core 2.9 % = tổng 9.9 / 11.2 %, node 10.5 / 10.2 %. Cổng bước 4: collector ≤ ~7.6 % và **tổng (collector + core) phải thấp hơn rõ node** (mục tiêu tổng ≤ 7.6 %). Plan `.planning/PLAN-v87-step4-core-transcript.md` §2.2–2.3.
- Việc:
  1. Đo profile cả hai bên ở chế độ rust (`node scripts/perf-bench.mjs --stage --scenario demo-30x6 --seed 8704 --collector-prof` + cách xem CPU core con; với Rust có thể dùng đếm thời gian theo op hoặc `cargo build --release` với symbols + sampling nếu có sẵn — không cài tool mới).
  2. Bật kèm `CCM_CORE_USAGE=rust` khi đo (usage `ingestLine` đang chạy Node trùng việc đọc file) — đo riêng từng cờ.
  3. Rust: parse typed / bỏ qua body (không dựng `serde_json::Value` cho cả dòng khi chỉ cần vài field; `RawValue` / struct borrow), tránh đọc lại file 2 lần giữa signals + usage nếu cùng file; giảm kích thước reply (delta, bỏ field không đổi).
  4. TS: giảm parse reply (chỉ parse khi có thay đổi, `JSON.parse` 1 lần), `buildTerminalOf` / `store.diff` / `publish` chỉ cho terminal bẩn (dirty) — giữ shape event.
  5. Đo lại 2 lượt mỗi chế độ (node / rust / rust+usage-rust), ghi `docs/perf/v87-step4-T3b-win32.json` + bảng.
- Phạm vi file: `apps/core/**`, `apps/collector/src/**` (giữ `tracker.ts` không đổi hành vi; không làm `monitor.ts` dài thêm), test collector, `.code_index/`, plan §2.x, `docs/perf/`. Cấm `apps/web/**`, `packages/shared`, `apps/desktop/**` (trừ truyền env nếu cần, ghi lý do).
- Model: opus.
- Parity giữ: `tracker-parity*.test.ts`, `transcript-parity.test.ts`, `usage-parity.test.ts`, snapshot `tracker.test.ts` không đổi.
- Lệnh test: `apps/core` (PowerShell) `cargo test --locked -j 4`, clippy host + `aarch64-apple-darwin` `-D warnings`; `npm test -w @ccm/collector -- --maxWorkers=4`; typecheck collector.
- An toàn: instance cô lập; KHÔNG chạm app Orrery đang chạy (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); kill chỉ pid tự spawn; máy ≤ 80 %. Build chỉ `node apps/desktop/build.mjs` + cargo release.
- Tiêu chí:
  - [ ] Bảng CPU node / rust / rust+usage (collector, core, tổng), 2 lượt.
  - [ ] Tổng rust < node rõ (mục tiêu ≤ 7.6 %); chưa đạt → top hàm còn lại hai bên.
  - [ ] Parity + test + clippy + typecheck sạch.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-v87-4-T3b-report.md`

## Nhật ký
- 2026-11-09 baseline (1 lượt, demo-30x6 seed 8704): node collector 10.0 %; rust 6.1 + core 2.2 = 8.3 %; rust+usage-rust 5.1 + core 3.0 (2 con) = 8.1 %.
- vòng 1 code: takeChanged (bỏ publish khi sync không đổi), view delta theo field (null = xoá), tail mở file 1 lần, parse prune (bỏ key không dùng); cargo test pass, parity collector 58 pass.
- đo vòng 1 (1 lượt, nhiễu): rust 7.0 + 2.9 = 9.9 %, rust+usage 5.2 + 3.1 = 8.3 %. Đếm 30 s collector --dev: 44 sync/s (31 skipResolved từ reconcile, 13 theo file), 59 % vẫn publish. Bench Rust: sync no-op skipResolved 381 µs (stat / read_dir Windows ~20–48 µs mỗi lần). Vòng 2: CoreTracker safetyNetMs 4 s (bỏ sync skipResolved của reconcile nếu tracker đã có sync full/skipResolved < 4 s, chỉ rust mode).
- vòng 2 đo (1 lượt): rust 7.2 + 1.4 = 8.6 %, rust+usage 5.1 + 1.9 = 7.0 % (core giảm ~50 %). clippy 2 target sạch, cargo test pass, collector 465 pass (lượt trước 2 flaky timing core-procs / prompt-search, chạy lại pass).
- đo cuối 2 lượt xen kẽ: node 10.3 / 10.1 %; rust 5.5+1.4=6.9 / 5.7+1.1=6.8 %; rust+usage 4.7+2.0=6.7 / 4.5+1.6=6.1 % → qua cổng ≤ 7.6 %. docs/perf/v87-step4-T3b-win32.json. Báo cáo WP-v87-4-T3b-report.md.
