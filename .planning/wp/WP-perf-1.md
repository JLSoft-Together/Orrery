## WP-perf-1: Collector CPU — usage scan, office hub, gitstat
- Mục tiêu: giảm CPU collector theo `.planning/PLAN-v86-performance.md` Gói 3, giữ nguyên dữ liệu/hành vi quan sát được.
- Phạm vi file: `apps/collector/src/usage.ts`, `apps/collector/src/office/hub.ts` (+ file office/ khác nếu cần cho gate), `apps/collector/src/monitor.ts` (chỉ vùng gitstat ~730-750), `apps/collector/src/gitstat.ts`, test tương ứng trong `apps/collector/test/`.
  Cấm đụng: `packages/shared`, `apps/web`, `apps/desktop`.
- Song song với: WP-perf-2, WP-perf-3.
- Model: sonnet.
- Việc:
  1. `usage.ts:410-421`: hết bậc hai. Không `Buffer.concat([remainder, chunk])` + `Buffer.from` mỗi chunk; gom các mảnh chưa có newline vào mảng, concat một lần khi gặp newline; `highWaterMark` 1 MB như `tailer.ts:4`. `entry.offset`/`consumed` phải giữ đúng nghĩa (dòng cuối chưa có newline không được tính).
  2. `office/hub.ts`: `commit` (setState toàn bộ + snapshot + maskSecrets + appendFileSync timeline) mỗi 200 ms kể cả 0 subscriber. Khi 0 subscriber: chỉ commit theo nhịp thưa (vd 5 s) để ghi timeline, và chỉ ghi khi `rev` đổi. Subscriber mới vẫn nhận state hiện tại ngay (`commit(true, clientId)`). Ghi timeline không chặn event loop lâu (gom/append bất đồng bộ nếu đơn giản; giữ thứ tự dòng). Timelapse 7 ngày phải vẫn đúng (xem `apps/collector/src/office/` và `timeline.ts`).
  3. gitstat (`monitor.ts:730-750`, `gitstat.ts:29-31`): debounce/dedupe theo `cwd` (nhiều terminal cùng cwd chỉ 1 lần git), cache "không phải repo" (TTL hợp lý, vd 5 phút). Kết quả vẫn gán cho mọi terminal cùng cwd.
- Tiêu chí nghiệm thu:
  - [ ] Test cho usage scan: file có dòng > 64 KB và > 1 MB, dòng cuối dở, cho cùng kết quả như trước.
  - [ ] Đo trước/sau cold scan bằng script tạm ngoài repo (file JSONL tổng hợp vài trăm MB có dòng 1–3 MB): ghi CPU-s vào report.
  - [ ] Test hub: 0 subscriber → không commit mỗi 200 ms; có subscriber → hành vi cũ; timeline vẫn ghi khi rev đổi.
  - [ ] Test gitstat: 3 terminal cùng cwd → 1 lần chạy git; cwd không phải repo → không spawn lại trong TTL.
  - [ ] `npm test -w @ccm/collector` và `npm run typecheck -w @ccm/collector` pass.
- Lệnh test: `npm test -w @ccm/collector`; `npm run typecheck -w @ccm/collector`.
- Nhìn thấy được: không.
- Báo cáo: `.planning/wp/WP-perf-1-report.md`

## Nhật ký
- usage.ts ingest: gom mảnh, highWaterMark 1 MB; hub.ts idleCommitMs 5s khi 0 subscriber; gitstat.ts createDiffStatReader (dedupe cwd + cache); test + typecheck pass
