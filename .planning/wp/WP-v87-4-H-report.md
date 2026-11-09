## WP-v87-4-H — PARTIAL (spawn và store/publish đã giảm rõ ở profile JS; CPU tổng collector chưa đạt ≤ 7.6 %)
- Nguồn spawn: duy nhất `gitstat.ts` (`git diff --shortstat` + `git ls-files`), do `monitor.ts scheduleDiff` mỗi khi status session đổi sang non-busy. Demo 30×6: 6 cwd `D:\work\demo\*` không tồn tại -> git thoát 128 "cannot change to", không được cache -> chạy lại mỗi lần. Đo bằng preload bọc child_process (75 s, instance cô lập).
- Đã đổi: `gitstat.ts` thêm `MISSING_DIR` (128 + "cannot change to / no such file"), cache 30 s per cwd (`missingDirTtlMs`), not-a-repo vẫn 5 phút. `store.ts`: index agent theo terminal (`agentsOf` O(k) thay vì quét hết), `diff` không tạo Set, upsert terminal/agent giống hệt thì không clone / không set / không emit. Shape event, `packages/shared`, tracker không đổi.
- Test mới: `test/store-diff.test.ts` (3), `gitstat.test.ts` (+1).

| Chỉ số (demo 30×6, seed 8704, 120 s) | Trước (T0) | Sau |
|---|---|---|
| execFile git / 75 s (harness) | 107 (~90/phút) | 17 (~14/phút) |
| Collector CPU avg, Electron stage | 10.3 % (8.6 CPU-s) | lượt 1: 10.3 % (12.2 CPU-s); lượt 2: 9.6 % (11.5 CPU-s); lượt trước build cuối: 12.4 % (nhiễu) |
| publish self ms/s | 6.5 | 1.1 |
| spawn self ms/s | 6.3 | 0.9 |
| store anon + clean ms/s | 5.5 + 5.1 | gần 0 (không còn trong top 20) |
| store diff ms/s | 3.4 | 2.2 |
Top mới (ms/s): diff 2.2, stat 2.1, usage.ingestLine 1.8, fs stat 1.3, discoverSubagents 1.1, publish 1.1, path.normalize 1.1.
- Tiêu chí: bảng spawn [đạt]; CPU 2 lượt + top hàm [đạt, số thật ghi trên]; protocol / test pass [đạt]; mục tiêu ≤ 7.6 % [CHƯA ĐẠT]: số Electron nằm trong nhiễu ±1–2 điểm so với baseline, dù profile JS standalone giảm mạnh. Phần còn lại là syscall stat/read/open (mỗi tick tracker stat file), usage.ingestLine, discoverSubagents; giảm tiếp cần đổi tracker / tailer (T2/T3) hoặc skip publish khi tracker không đổi.
- Audit: diff chỉ gồm `gitstat.ts`, `store.ts`, 2 test, `.code_index/_meta.md`, `docs/perf/`. Review: patch `null` cho field thành undefined giữ nguyên, thứ tự agentsOf giữ insertion order; chưa kiểm hành vi branch / status trên máy thật (diff trễ không đổi, debounce 2 s giữ).
- Test: `npm test -w @ccm/collector -- --maxWorkers=4` 436 pass (+3 new, 2 skipped sẵn); `typecheck` ok; chưa chạy `@ccm/web` (không đụng shape event).
- Ngoài phạm vi: usage.ts / tailer / tracker không đụng. File dữ liệu: `docs/perf/v87-step4-H-win32.json` (lượt 1 + collectorProfile), `docs/perf/v87-step4-H-run2-win32.json` (lượt 2). Thư mục tạm /tmp/hh và %TEMP%\tmp.Y0ZXG0k8as (fixture) còn lại, process đã dừng.
- Commit message đề xuất: ⚡️ Collector: cache missing-cwd git failures for 30 s, index agents by terminal, skip clone / emit for unchanged store upserts
