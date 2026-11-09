## WP-v87-4-T1g: đo cổng cold usage scan 4.29 GB — Rust vs Node
- Bối cảnh: `.planning/PLAN-v87-step4-core-transcript.md` (cổng cold usage scan 4 GB ≤ 5 CPU-s). Baseline Node T0: `.planning/wp/WP-v87-4-T0-report.md` (5.06–7.02 CPU-s). Rust usage index T1 (`orrery-core transcript`, op `usage.scan`, plan §2.1; client `apps/collector/src/coreTranscript.ts`, cờ `CCM_CORE_USAGE=rust`). T1 chỉ đo corpus 507 MB.
- Việc (chỉ đo, không sửa code sản phẩm):
  1. Corpus: dùng `%TEMP%\orrery-usage-corpus-23300` nếu còn; không còn thì tạo lại bằng `node scripts/usage-corpus.mjs` (đọc `--help` / đầu file) đúng cỡ ~4.29 GB.
  2. Build core release nếu chưa có: trong `apps/core` chạy (PowerShell) `cargo build --release --locked -j 4 -p core-bin`.
  3. Đo cold scan (cache file mới, thư mục tạm) 3 lượt Node (`UsageIndex`, cách đo như T0) + 3 lượt Rust (spawn `orrery-core transcript`, gửi `usage.scan` với `cacheFile` tạm, chờ event `usage.reset`, `usage.stop`, đo CPU-s user+sys của process con, wall, peak RSS). Thêm 1 lượt warm mỗi bên (cache đã có). Script đo để trong `%TEMP%` (không commit), chỉ commit kết quả JSON.
  4. Ghi `docs/perf/v87-step4-T1-cold-win32.json` (máy, số core, từng lượt, trung vị) + 1 bảng trong báo cáo; kết luận đạt / chưa đạt ≤ 5 CPU-s.
- Phạm vi file: chỉ tạo `docs/perf/v87-step4-T1-cold-win32.json` và báo cáo. Không sửa code repo.
- An toàn máy: mỗi lúc chỉ 1 scan; cargo `-j 4`; máy ≤ 80 % CPU / RAM (đang cao thì chờ). KHÔNG chạm app Orrery đang chạy (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); không dùng `CCM_DATA_DIR` thật; kill chỉ pid tự spawn, không `taskkill /IM`.
- Không git commit / push.
- Báo cáo: `.planning/wp/WP-v87-4-T1g-report.md` (bảng + kết luận, dòng cuối `STATUS: DONE|PARTIAL|BLOCKED`).
