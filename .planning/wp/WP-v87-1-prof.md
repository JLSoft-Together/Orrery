## WP-v87-1-prof: CPU profile renderer khi nhiều agent (chỉ đo, không sửa code app)
- Bối cảnh: sau v86 + bước 1 (`docs/perf/v87-step1-win32.json`) so với 3.4.0 (`docs/perf/3.4.0-win32.json`): demo-30x6 renderer 116.7 → 103.9 % một core (−11 %), GPU 25.3 → 23.5 %; demo-6x3 renderer 16.6 → 15.4 %, GPU 14.5 → 13.2 %. Lúc rảnh đã gần 0. Renderer khi nhiều agent vẫn là điểm nóng nhất, cần biết chính xác hàm / component nào.
- Việc:
  1. Thêm vào `scripts/perf-bench.mjs` cờ `--profile` (CDP `Profiler.start` / `Profiler.stop` trên target renderer chính, 20–30 s sau settle, chỉ ở kịch bản demo), lưu `.cpuprofile` vào temp, in + ghi JSON tóm tắt: top 30 hàm theo self time và total time, gom theo file nguồn (dùng sourcemap nếu stage có; nếu không, gom theo tên chunk + tên hàm). Thêm `--stage` đã có.
  2. Chạy với bản stage hiện tại (`apps/desktop/stage` đã build từ working tree; KHÔNG build lại): `--stage --scenario demo-6x3,demo-30x6 --profile`.
  3. Đếm thêm trong demo-30x6: số DOM node theo vùng (rail / session list, Agent Map, activity, header), số component Vue mount nếu lấy được, số lần store patch / giây.
  4. Báo cáo: top hàm → component / store / file:line trong `apps/web/src`, xếp theo % thời gian renderer; đề xuất cách sửa cho top 5 (chưa sửa).
- Phạm vi file: `scripts/perf-bench.mjs`, `docs/perf/README.md`, `docs/perf/v87-prof-win32.json` (mới). Cấm đụng `apps/`, `packages/`.
- Model: sonnet.
- Luật chạy: instance riêng như WP-v87-0 (không đụng app chủ nhân, cổng 4317, `%APPDATA%/Agent Monitor`, `%LOCALAPPDATA%/ccm`), BelowNormal, 1 kịch bản mỗi lúc, xoá temp khi xong. Không comment trong code.
- Tiêu chí nghiệm thu:
  - [ ] `--profile` chạy được bằng 1 lệnh, kết quả ghi JSON.
  - [ ] Bảng top hàm + map về file:line nguồn cho demo-6x3 và demo-30x6.
  - [ ] Đề xuất sửa cho top 5 kèm ước lợi.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-v87-1-prof-report.md`

## Nhật ký
- 04:03 thêm --profile/--profile-seconds/--profile-dir vào perf-bench.mjs (node --check ok)
- 04:07 run 1: probe $subscribe(deep) làm méo profile (pi=traverse 54%); đổi probe sang sau profile (--patch-seconds), chạy lại
- 04:12 run 2 xong (probe sau profile), README cập nhật, temp xoá; viết báo cáo
