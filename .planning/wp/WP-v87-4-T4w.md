## WP-v87-4-T4w: đo lại cổng T4 trên Windows sau R2 (mặc định Rust) + E2E full

- Bối cảnh: `.planning/PLAN-v87-step4-core-transcript.md` (§3 T4, cổng CPU collector demo 30×6 ≤ ~7.6 %, cold scan ≤ 5 CPU-s). R2 (commit `9b7038e`) đổi mặc định `CCM_CORE_USAGE` / `CCM_CORE_TRANSCRIPT` sang `rust`. Kết quả mac: `.planning/QA-v87-mac-result.md` mục T4, `docs/perf/v87-step4-T4-darwin.json`. Số Windows trước R2: `docs/perf/v87-step4-T3b-win32.json`.
- Việc (chỉ đo + chạy test, không sửa code sản phẩm):
  1. Tạo file khoá `%TEMP%\orrery-bench.lock` lúc bắt đầu bước 3–4, xoá khi xong (WP khác chờ file này rồi mới cargo build).
  2. Build: `npm run build -w @ccm/web` không cần nếu stage còn mới; `node apps/desktop/build.mjs` (được phép); core release: trong `apps/core` (PowerShell) `cargo build --release --locked -j 4 -p core-bin`.
  3. Parity: `npm test -w @ccm/collector -- --maxWorkers=4` (gồm `tracker-parity`, `usage` parity) — ghi số pass / fail.
  4. CPU demo 30×6 seed 8704, 120 s, 2 lượt mỗi bên: `node scripts/perf-bench.mjs --stage --scenario demo-30x6 --seed 8704` với mặc định (rust) và với `CCM_CORE_TRANSCRIPT=node CCM_CORE_USAGE=node`. Tính tổng collector + `orrery-core`. Ghi `docs/perf/v87-step4-T4-win32.json` (máy, từng lượt, trung bình, so T3b).
  5. E2E full trên stage dev (mặc định rust): `npm run e2e -w @ccm/desktop` (xem `apps/desktop/e2e/run.cjs` cách chạy từng bộ / cả bộ). Liệt kê bộ fail; với mỗi bộ fail chạy lại 1 lần riêng để tách flaky. Bộ fail mà chạy lại với `CCM_CORE_PTY=node CCM_CORE_TRANSCRIPT=node CCM_CORE_USAGE=node` pass → ghi rõ là lỗi do core Rust (quan trọng nhất).
- Phạm vi file: chỉ tạo `docs/perf/v87-step4-T4-win32.json` và báo cáo. Script đo tạm để trong `%TEMP%`. Không sửa code repo; thấy lỗi thì ghi vào báo cáo (file:line + log), không tự sửa.
- An toàn máy: mỗi lúc chỉ 1 việc nặng (cargo `-j 4`, đo, E2E tuần tự); máy ≤ 80 % CPU / RAM (đang cao thì chờ). KHÔNG chạm app Orrery đang chạy của chủ nhân (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); perf-bench và E2E đã dùng instance cô lập, đừng đổi. Kill chỉ pid tự spawn, không `taskkill /IM`.
- Không git commit / push. Không viết comment vào code.
- Tìm kiếm bằng `rg`, không `grep -r`. Văn bản báo cáo tiếng Việt: viết `max` không viết "trần", `deadline chờ …` không viết "watchdog".
- Báo cáo: `.planning/wp/WP-v87-4-T4w-report.md` (bảng CPU, kết luận cổng đạt / chưa, danh sách E2E fail chia 3 nhóm: lỗi core Rust / flaky / script cũ), dòng cuối `STATUS: DONE|PARTIAL|BLOCKED`.
