## WP-v87-1-react: Renderer khi nhiều agent — reactivity store + Agent Map
- Số đo (`.planning/wp/WP-v87-1-prof-report.md`, `docs/perf/v87-prof-win32.json`), demo-30x6 trên stage hiện tại: renderer 115.5 % một core, main thread bận 83 %, native 36 %, DOM 6 425 node (header 59, rail 2 019, map 4 201). Hot: `flushJobs` 41 % total, reactive `get` proxy 7.1 % self, Vue Flow edge path 7.3 %, `setEdges` 2.3 %, Transition leave `forceReflow` 2.2 % + `getComputedStyle` 1.8 % (`SessionList.vue:~184`, `AgentMap.vue:~1334`). Store: agents 124 patch/s (42 action/s), toolSpans 22/s (Map mới mỗi action, `stores/toolSpans.ts:15-18`), terminals 13/s. demo-6x3: JS không nóng, chủ yếu paint native.
- Đã có: WS event gom theo frame ở `stores/connection.ts` (`receive()` / `flush()`, WP-v87-1-load) — nhưng mỗi event trong batch vẫn gọi action store riêng.
- Việc:
  1. Store apply theo lô: `connection.flush()` gọi một action lô cho mỗi store (vd `agents.applyMany(events)`), patch một lần; `toolSpans` không tạo Map mới mỗi action (cập nhật tại chỗ + một trigger). Giữ thứ tự / ngữ nghĩa event (test `connection.batch.test.ts` + test store hiện có phải pass).
  2. `agents.byId` (và terminals nếu cùng mẫu): `shallowReactive` / `shallowRef` map + object agent `markRaw` / `Object.freeze`, thay object khi đổi (immutable update) để component vẫn nhận thay đổi; tránh reactive sâu trên payload lớn.
  3. `AgentMap.vue:~786-818`: thay đổi chỉ về status / tool / thời gian không được kéo validate / dựng lại toàn bộ edges; chỉ cập nhật data node bị đổi. Node / edge chỉ thêm / xoá khi tập id đổi.
  4. Transition leave `forceReflow` + `getComputedStyle` (`SessionList.vue:~184`, `AgentMap.vue:~1334`): thay JS hook đọc layout bằng Web Animations API (`el.animate`) hoặc CSS transition thuần, không đọc layout đồng bộ. Animation vẫn có (rule chủ nhân).
  5. Vue Flow: bật `only-render-visible-elements` (hoặc tương đương của bản đang dùng) cho Agent Map nếu không phá fit view / minimap / pan; đo node DOM trước / sau trong test nếu được, nếu không ghi CHƯA KIỂM.
  6. Rail 2 019 node: CHỈ ghi đề xuất virtualize (không làm trong WP này).
- Phạm vi file: `apps/web/src/stores/*`, `components/AgentMap.vue` + edge / node component của map, `components/SessionList.vue` (+ component rail liên quan chỉ cho mục 4), `lib/*` helper animation, test tương ứng. Cấm đụng: `apps/collector`, `apps/desktop`, `packages/`, `scripts/`, `docs/perf/`, `components/workspace/*`.
- Model: sonnet.
- Tiêu chí nghiệm thu:
  - [ ] Test lô: nhiều event cùng id trong 1 lô → 1 patch, kết quả cuối đúng; created / removed / updated trộn; snapshot giữa chừng.
  - [ ] Test: đổi status một agent không làm đổi reference mảng edges của map (hoặc không gọi hàm dựng edges).
  - [ ] `npm test -w @ccm/web -- --maxWorkers=6` + `npm run typecheck -w @ccm/web` pass.
  - [ ] Đo lại: main build + `node scripts/perf-bench.mjs --stage --scenario demo-6x3,demo-30x6 --profile` sau (CHƯA KIỂM trong WP).
- Nhìn thấy được: có (animation row / map) → CHƯA NHÌN.
- Báo cáo: `.planning/wp/WP-v87-1-react-report.md`

## Nhật ký
- 1. Audit xong: connection.apply gọi action từng event; agents/terminals reactive sâu + patch tại chỗ; edges đã có stable compare trong AgentMap.
- 2. Store lô: lib/patch.ts applyEntityOps + agents/terminals.applyMany; connection.apply gom ops, flush cuối; toolSpans hold/release + triggerRef (không Map mới).
- 3. shallowReactive + Object.freeze cho agents/terminals byId, patch thay object (bỏ qua nếu không đổi).
- 4. Transition: SessionList :css=false + WAAPI (lib/motion listEnter/listLeave); capsule Agent Map thành Transition riêng + WAAPI.
- 5. Edges: tách lib/stableEdges.ts + test. only-render-visible: KHÔNG bật (rủi ro fit view/minimap, CHƯA KIỂM).
- 6. Test: 216 file / 1845 pass, typecheck pass.
