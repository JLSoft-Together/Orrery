## WP-perf-3: RAM renderer — worker idle, AudioContext, Monitor chunk
- Mục tiêu: giảm RAM private renderer + Audio service theo PLAN-v86 Gói 4.1–4.3.
- Phạm vi file: `apps/web/src/lib/highlight.ts` (+ `highlight.worker.ts` nếu cần), `apps/web/src/lib/sound.ts`, `apps/web/src/office/sound.ts`, `apps/web/src/App.vue`, `apps/web/src/components/AgentMap.vue`, file import Monitor/AgentMap/VueFlow liên quan, test tương ứng.
  Cấm đụng: `TermPane.vue`, `WorkspaceView.vue`, `stores/workspace.ts`, `stores/attachments.ts`, `devServer.ts`, locale files, `apps/desktop`, `apps/collector`.
- Song song với: WP-perf-1, WP-perf-2.
- Model: sonnet.
- Việc:
  1. `lib/highlight.ts:~27`: terminate shiki worker sau ~60 s không có request; request mới thì tạo lại. Request đang chờ khi terminate phải được resolve/reject đúng (không treo).
  2. `lib/sound.ts`, `office/sound.ts`: `AudioContext.close()` sau ~30 s không phát âm; lần phát sau tạo lại. Giữ đúng hành vi unlock/resume hiện có (xem test `office/sound*.test.ts`, `stores/office.sound*.test.ts`).
  3. Tách Monitor / AgentMap / Vue Flow / OfficeView khỏi entry chunk: `defineAsyncComponent` ở `App.vue:~27` và chỗ import tĩnh (`AgentMap.vue:~54`). Không làm nháy trắng khi chuyển sang Monitor lần đầu (giữ loading nhẹ hoặc prefetch idle sau boot, vd `requestIdleCallback` có fallback setTimeout). Không đổi `v-if/v-else-if` hiện có.
- Tiêu chí nghiệm thu:
  - [ ] Test highlight: idle 60 s (fake timers) → worker terminate; request sau đó vẫn trả kết quả.
  - [ ] Test sound: im lặng 30 s → close; phát lại → ctx mới, không lỗi.
  - [ ] Không có import tĩnh Monitor/AgentMap/VueFlow/OfficeView từ entry (liệt kê bằng rg trong report).
  - [ ] `npm test -w @ccm/web` + `npm run typecheck -w @ccm/web` pass.
- Lệnh test: như WP-perf-2.
- Nhìn thấy được: có (chuyển Monitor lần đầu) → không được chạy app, ghi CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-perf-3-report.md`
- 02:37 xong highlight/sound/office sound/App/AgentMap + test; báo cáo ghi.
