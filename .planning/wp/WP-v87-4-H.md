## WP-v87-4-H: giảm CPU collector — spawn mỗi tick + publish / store diff
- Bối cảnh: `.planning/PLAN-v87-step4-core-transcript.md` §5. Baseline T0 (`docs/perf/v87-step4-baseline-win32.json`, `.planning/wp/WP-v87-4-T0-report.md`): collector demo 30×6 seed 8704 = 10.3 % avg / 8.6 CPU-s / 120 s; cổng ≤ ~7.6 %. Top self-time: `monitor.ts publish` 6.5 ms/s, `spawn` 6.3 ms/s, `store.ts` anon 5.5 / `clean` 5.1 / `diff` 3.4, `stat` 3.1, `tailer.consume` 2.9, `usage.ingestLine` 2.4.
- Việc:
  1. **Spawn:** tìm ai spawn process con trong collector ở chế độ demo (stack của `spawn` trong profile — chạy `node scripts/perf-bench.mjs --stage --scenario demo-30x6 --seed 8704 --collector-prof` hoặc tương đương đã có từ T0; hoặc bọc `child_process.spawn/execFile` tạm trong test để đếm). Ứng viên: git branch / gitstat, process verify, codex / deepseek processes, resources sampler, ports, opener… Giảm: cache theo mtime / theo nội dung, gộp, tăng chu kỳ khi không đổi, bỏ khi không ai xem (cửa sổ ẩn). Hành vi người dùng thấy giữ nguyên (độ trễ cập nhật branch / status không tệ hơn ~1 chu kỳ hiện tại).
  2. **publish / store diff / clean:** đọc `apps/collector/src/store.ts`, `monitor.ts publish`; tìm việc lặp vô ích (diff toàn bộ khi chỉ 1 session đổi, clean / clone object mỗi tick, JSON.stringify để so…). Đổi sang diff theo key bẩn (dirty set), tránh clone sâu, giữ đúng protocol (`snapshot` rồi event tăng dần; không đổi shape event — `packages/shared`).
  3. Đo lại cùng điều kiện (seed 8704, demo 30×6, 120 s, instance cô lập, stage build mới), 2 lượt; ghi `docs/perf/v87-step4-H-win32.json` + top hàm mới. Mục tiêu ≤ 7.6 % avg.
- Phạm vi file: `apps/collector/src/**` (trừ `usage.ts`, `usageSource.ts`, `coreTranscript.ts`, `coreProcs.ts` — chỉ đụng nếu là nguồn spawn, ghi lý do), test collector, `docs/perf/`, `.code_index/collector.md`. Cấm đụng `apps/core/**`, `apps/web/**`, `packages/shared` (shape protocol giữ nguyên).
- Model: sonnet (escalate nếu cần).
- File nhỏ: file mới ≤ ~400 dòng; không làm dài thêm `monitor.ts` (936) / `tracker.ts` — tách module nếu thêm logic.
- Build được phép: `node apps/desktop/build.mjs` (stage dev) để đo. Không `dist:*`.
- An toàn máy (bắt buộc): đo bằng instance cô lập (`perf-bench` đã làm); KHÔNG chạm app Orrery đang chạy (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); dọn process chỉ theo pid tự spawn, KHÔNG `taskkill /IM`.
- Lệnh test: `npm test -w @ccm/collector -- --maxWorkers=4` (snapshot `tracker.test.ts` không đổi), `npm run typecheck -w @ccm/collector`, `npm test -w @ccm/web -- --maxWorkers=4` (renderer nhận event giữ nguyên).
- Tiêu chí:
  - [ ] Bảng: nguồn spawn tìm được (ai, bao nhiêu lần / phút) trước / sau.
  - [ ] CPU collector demo 30×6 trước / sau (2 lượt) + top hàm.
  - [ ] Protocol / snapshot không đổi; test pass.
- Nhìn thấy được: không (cập nhật status / branch vẫn kịp).
- Báo cáo: `.planning/wp/WP-v87-4-H-report.md`

## Nhật ký
- [audit+fix] spawn = gitstat.ts execFile git (~90/phút, 6 cwd demo không tồn tại, lỗi 128 không cache) -> MISSING_DIR TTL 30s (17/75s sau). store.ts: index agent theo terminal (agentsOf O(N)->O(k)), diff/clean bớt Set + clone. tests 436 pass, typecheck ok, build stage ok; đang đo perf-bench lượt 1
- [test+đo] store diff theo key (không clone khi không đổi) + test store-diff; perf-bench 2 lượt: collector 10.3% / 9.6% (baseline 10.3%); self-time publish 6.5->1.1, spawn 6.3->0.9, store anon+clean 10.6->~0, diff 3.4->2.2 ms/s. PARTIAL, chưa đạt 7.6%
