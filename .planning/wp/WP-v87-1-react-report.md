## WP-v87-1-react — PARTIAL
- Đã đổi:
  - `apps/web/src/lib/patch.ts` — thêm `applyEntityOps` (apply lô upsert / patch / remove: 1 object mới mỗi id mỗi lô, patch không đổi gì thì bỏ qua, `Object.freeze`).
  - `apps/web/src/stores/agents.ts`, `terminals.ts` — `byId` là `shallowReactive`, object frozen, thay object khi đổi; thêm `applyMany`; `upsert` / `patch` / `remove` đi qua `applyMany`.
  - `apps/web/src/stores/connection.ts` — `apply()` gom event terminal / agent thành ops, flush một lần cuối lô (và trước snapshot); `toolSpans.hold()` / `release()` (try/finally); bỏ chọn agent bị removed sau khi flush (xoá bộ nhớ removed khi gặp snapshot giữa lô).
  - `apps/web/src/stores/toolSpans.ts` — `touch()` dùng `triggerRef` thay `new Map` mỗi action; `hold` / `release` gộp `version++` một lần mỗi lô.
  - `apps/web/src/lib/motion.ts` — `listEnter` / `listLeave` / `capsuleEnter` / `capsuleLeave` bằng Web Animations API (tôn trọng reduced motion, `done()` ngay).
  - `apps/web/src/components/SessionList.vue` — TransitionGroup `:css="false"` + `@enter` / `@leave` WAAPI (`@before-leave="pinLeaving"` giữ nguyên).
  - `apps/web/src/components/AgentMap.vue` — capsule: TransitionGroup thành `<Transition :css="false" appear>` riêng từng capsule + WAAPI, bỏ block `<style>` capsule; `edges` dùng `stableEdges`.
  - `apps/web/src/lib/stableEdges.ts` (mới) — tách `sameEdge` ra, `stableEdges(prev, next)`.
  - Test: `stores/connection.batch.test.ts` (+7), `lib/stableEdges.test.ts` (mới, 3).
- Từng mục:
  1. LÀM. Store apply theo lô (agents, terminals; toolSpans không tạo Map mới, 1 trigger / lô). Ước lợi: 124 patch/s agents → tối đa 1 lần ghi mỗi id mỗi frame; toolSpans 22 Map copy/s → 0. Số đo thực CHƯA KIỂM.
  2. LÀM (agents + terminals). shallowReactive + frozen + thay object. Đánh đổi: mỗi lần agent đổi, mọi computed đọc `byId[id]` chạy lại (một lần mỗi lô) thay vì chỉ field con; đổi lại bỏ proxy sâu (reactive `get` 7.1 %). Không còn chỗ nào mutate tại chỗ (rg: chỉ `applyPatch` trong store, nay chạy trên bản copy).
  3. ĐÃ CÓ SẴN phần lớn: node `data` chỉ `{ agentId }`, `structureKey` là chuỗi nên đổi status / tool không chạy `syncNodes`; `edges` đã so sánh để giữ reference. Con làm: tách `stableEdges` + test. Edge `ccm-edge-hot` đổi theo status nên đổi status working↔khác vẫn tạo mảng mới (đúng: class đổi). Chưa giảm được việc `buildEdges()` chạy lại mỗi lô khi agent đổi.
  4. LÀM một phần. Hết `forceReflow` + `getComputedStyle` của leave / enter ở SessionList và capsule map. CÒN: TransitionGroup của SessionList vẫn đo vị trí (`getBoundingClientRect` + clone `getComputedStyle`) khi hàng đổi chỗ để chạy `ccm-list-move` (FLIP của Vue); bỏ nó cần tự viết FLIP, ngoài phạm vi. Các TransitionGroup `ccm-list` khác (CompactView, ActivityFeed…) không đụng.
  5. KHÔNG LÀM. `only-render-visible-elements`: node ngoài viewport không mount nên không đo `dimensions` → `fitView` / minimap / `ui.nodeSizes` (layout) sai; không kiểm được khi không chạy app. CHƯA KIỂM. Đề xuất: thử riêng khi có thể chạy app và đo `DOM nodes` map 4 201.
  6. Rail 2 019 node: ĐỀ XUẤT virtualize list rail (`SessionList` / rail cards) bằng windowing theo `data-terminal-id` hàng cao cố định; cần giữ `freeze order`, `scrollIntoView` theo `revealRequest`, FLIP move. Chưa làm.
- Tiêu chí: 3/4 đạt (lô, edges reference, test + typecheck pass; đo lại bằng `perf-bench` CHƯA KIỂM).
- Audit: connection.apply gọi action từng event; agents / terminals reactive sâu; peers, toolSpans không đọc agents / terminals nên gom ops về cuối lô an toàn; không chỗ nào khác mutate agent tại chỗ.
- Review edge case: removed rồi created cùng lô (test); patch sau removed bị bỏ, patch id lạ bỏ qua (test); snapshot giữa lô flush trước, selected agent không bị bỏ chọn nhầm nếu snapshot đưa lại (test); reconnect + snapshot dùng `replaceAll` (frozen); parent đổi / subagent của agent đã xoá: logic map đọc `byId` như cũ, không đổi; computed `byTerminal` / `subagents` vẫn cập nhật (test); filter / search rail, focus terminal, peer edges: không đổi code, CHƯA KIỂM bằng UI; reduced motion: WAAPI gọi `done()` ngay.
- Test: `npm test -w @ccm/web -- --maxWorkers=6` → 1845 pass / 0 fail (216 file); `npm run typecheck -w @ccm/web` → pass.
- Nhìn: CHƯA NHÌN: không được chạy app (animation hàng SessionList enter / leave, capsule map; `appear` của capsule cần xem thật).
- Commit message đề xuất:
  Batch agent and terminal updates per frame, freeze store objects, move list and capsule transitions to WAAPI

## Vòng 2
- AgentMap.vue: capsule quay lại một `<TransitionGroup tag="div" :css="false" @enter="capsuleEnter" @leave="capsuleLeave">` bao ngoài (giữ class `pointer-events-none absolute inset-0 z-10 overflow-hidden`), bỏ `<Transition>` từng capsule và `appear`. Capsule rời list nay chạy fade-out.
- composables/usePageActive.ts: `bind()` chạy ngay khi module load (idempotent), mọi nơi import `pageActive` (TreeEdge) đều đúng.
- lib/motion.ts `play`: `document.hidden` thì `done()` ngay, không chờ `onfinish`.
- Test: `npm test -w @ccm/web -- --maxWorkers=6` → pass (xem số dưới); `npm run typecheck -w @ccm/web` → pass. CHƯA NHÌN: fade capsule.
