## WP-v87-1-prof — DONE
- Đã đổi: `scripts/perf-bench.mjs` (cờ `--profile`, `--profile-seconds`, `--patch-seconds`, `--profile-dir`), `docs/perf/README.md` (mô tả cờ), `docs/perf/v87-prof-win32.json` (mới, 2 kịch bản demo). Không đụng `apps/`, `packages/`. Không comment, không dependency mới.
- Lệnh: `node scripts/perf-bench.mjs --stage --scenario demo-6x3,demo-30x6 --seconds 20 --settle 20 --profile --profile-seconds 25 --profile-dir <dir> --label v87-prof --out docs/perf/v87-prof-win32.json`
- Lưu ý đo: lần chạy 1 dùng probe `$subscribe` (deep watcher của Pinia) ngay trong lúc profile → `traverse` chiếm 54 % self, là số do probe tạo ra, đã bỏ. Số dưới là lần 2 (probe chạy 10 s SAU profile). Số patch/s đo lúc có probe nên hơi thấp hơn thực.
- Bundle không có source map: tên hàm là `file:line:col` của chunk minified (index-sNX6iJPS.js = vue + vue-flow + app, MonitorView-BqBb5elx.js = AgentMap + vue-flow edge/node), map về nguồn bằng đọc chunk.

### Kết quả (renderer main thread, 25 s)
| | demo-6x3 | demo-30x6 |
|---|---|---|
| renderer CPU (cả process) | 19.8 % | 115.5 % |
| main thread bận (không idle) | 9.5 % | 83 % |
| (program) native: style/layout/paint | 1.7 s / 25 s (7 %) | 9.1 s / 25 s (36 %) |
| GC | 0.07 s | 0.43 s |
| DOM node (header / rail / map / other) | 1203 (59 / 358 / 640 / 146) | 6425 (59 / 2019 / 4201 / 146) |

demo-6x3 gần như không có JS nóng (top self `get` 0.25 %); cost nằm ở native paint + frame (đã biết từ WP-v87-0). Toàn bộ điểm nóng JS là demo-30x6.

### Top hàm demo-30x6 (% main thread, self | total) → nguồn
1. Vue Flow edge path, `(anon) MonitorView:27:65247` (EdgeWrapper `getEdge`/path computed): total 7.3 %, kéo theo reactive `get` 8.6 % total (chủ yếu 2162 ms/25 s `get` index:2:13886 gọi từ hàm này + `Yn` MonitorView:7:12982). Chạy trong `Nb` (flushJobs) 41 % total.
2. Reactive `get` proxy index:2:13886 = 7.1 % self (cao nhất); `track` 1.6 %, `dh`/`get dirty` (kiểm computed dirty) 1.4 % self, 8.3 % total. Người gọi: edge path (trên), `id` MonitorView:30:48256 (`apps/web/src/components/AgentMap.vue` nav / layout watcher 30:48166), `iae` index:7:495660 (lodash-style trong computed 30:36266 = AgentMap `finished`/`shownByTerminal`/`structureKey` chain, `AgentMap.vue:131-247`).
3. Vue Flow `setEdges` + `ya` (validate cả mảng edge) MonitorView:27:5484 = 2.3 % total (watch `n.edges` → `setEdges`, `W` 27:28618); `AgentMap.vue:786-789` đã memo `edges` nhưng watch Vue Flow vẫn gọi lại khi mảng thay.
4. `Transition` leave: `onLeave` index:2:74036 → `forceReflow` (`wh`, `body.offsetHeight`) 2.2 % self, `getTransitionInfo` (`_k`/`fg`, `getComputedStyle`) 1.8 % self + dồn thêm vào (program) layout. Nguồn `SessionList.vue:184` (`TransitionGroup ccm-list`, `pinLeaving`) và `AgentMap.vue:1334` (`ccm-peer-capsule`), `MainNode.vue:250` `ccm-swap`. Cộng 4 % self + style recalc phía native.
5. Component patch/update props: `z` (patch) 27 % total, `E_`+`rk` (updateProps) 2.4 % self; `ag` (renderComponentRoot) 13.5 %. Số component không đếm được (build prod không có `__vueParentComponent`). Dòng edge/layout còn lại: `ft` (AgentMap `layout`, `AgentMap.vue` `syncNodes`, MonitorView:30:39169) 1.3 %, `getTotalLength` 0.9 % (`measureEdges`/`TreeEdge.vue`, MonitorView:29:8170 `Oi`), TreeEdge pulse rAF (`TreeEdge.vue:60-72`) nhỏ ở 30x6 (không vào top 25 self).
- Theo file: index chunk 32.8 %, MonitorView chunk 9.4 % tổng; ~36 % native (program) + 0.4 s GC.
- Vì flush: `Nb` (flushJobs) 41 % total, `runIfDirty` 32 %. Tức khoảng 41 % thời gian main thread nằm trong một lần flush scheduler theo store patch.

### Store patch / giây (agents: 124 patch/s và 42 action/s ở demo-30x6; 36 / 14.5 ở demo-6x3)
demo-30x6: agents 124.1 p/s (42.1 a/s), toolSpans 22.4 p/s (22.4 a/s), terminals 13.4 p/s (8.9 a/s), usage 5.9, snooze 12.1 a/s, awake 1.2, activity 0.3. Một action `agent.updated` thành ~3 patch (applyPatch gán từng key; chỉ trigger khi đổi giá trị nhưng `updatedAt` luôn đổi). `toolSpans` mỗi action tạo `new Map` + `version++` (`stores/toolSpans.ts:15-18`) → 22 lần/s mọi người đọc `byTerminal` nhận trigger.

### Đề xuất sửa top 5 (chưa sửa) + ước lợi
1. Gom event WS theo frame cho `agents`/`toolSpans`/`terminals` (một lần flush mỗi rAF/100 ms thay vì 42+22+9 action/s): `stores/connection.ts` (đã có batching theo `connection.batch.test.ts`, kiểm xem agent patch có vào cùng batch không) + `toolSpans.touch()` chỉ tăng version 1 lần mỗi frame. Ước lợi: giảm 30–50 % số flush `Nb` (41 % total) → renderer 30x6 −15–20 điểm % core.
2. `agents.byId` dùng `shallowReactive` + agent đóng băng (patch thay object, không deep proxy): bỏ phần lớn `get` 7.1 % self + `track` 1.6 % + `dh` 1.4 % + gọi qua 3 lớp proxy ở edge/layout. Ước lợi: 8–12 %. Rủi ro: mọi chỗ mutate `agent.x = ` trực tiếp (cần rg), đổi API store.
3. Edge: tắt tính lại path khi chỉ status/text đổi không đổi kích thước node (không gọi `updateNodeDimensions`/`setEdges`), và chỉ `setEdges` khi id/source/target đổi (hiện `class` đổi theo `a.status === 'working'` làm mảng đổi → validate cả ~210 edge). Dùng class trên node thay vì edge, hoặc CSS biến. `AgentMap.vue:786-818`. Ước lợi: edge path 7.3 % + setEdges 2.3 % → còn ~3 % (−6 %).
4. Bỏ `Transition` có forceReflow ở rail khi row rời do filter/gom nhóm (`SessionList.vue:184`, `AgentMap.vue:1334`): dùng class + `animationend` hoặc `css: false` với WAAPI, và bỏ hẳn khi số row > ~12 (motion giảm dần theo tải). Ước lợi: 4 % self + giảm layout native (~2–4 % thêm). Giữ luật chủ nhân "mọi thay đổi đều animate" → chỉ làm nhẹ (WAAPI không reflow), không bỏ animate.
5. Giảm DOM: rail 2019 node cho 30 session (~67/row) và map 4201 node cho ~210 agent (~20/agent). Virtualize rail (chỉ mount row trong viewport + 1 màn đệm) và map: node ngoài viewport Vue Flow `only-render-visible-elements`. Ước lợi: (program) 36 % → ~20 %, GC giảm, RAM renderer 504 MB → ~350 MB. Rủi ro: lớn nhất (focus, keyboard nav, animate leave).

### Tiêu chí
- [x] `--profile` chạy bằng 1 lệnh, ghi JSON (`docs/perf/v87-prof-win32.json`, đã chạy thật 2 lần, exit 0).
- [x] Bảng top hàm + map file:line cho demo-6x3 và demo-30x6 (trong JSON `profile.topSelf/topTotal/byFile`; map nguồn ở trên).
- [x] Đề xuất sửa top 5 kèm ước lợi (ước lợi là ước tính, chưa kiểm bằng sửa thật).
- CHƯA KIỂM: số component Vue mount (bản prod không có `__vueParentComponent`, `vueComponents: 0`); activity panel không mở trong kịch bản nên vùng activity = 0 node; số patch/s đo có probe `$subscribe` (deep) nên thấp hơn thực nhẹ; map minified → nguồn là đọc chunk bằng tay, hàm `id`/`iae`/`Yn` chưa xác nhận tên nguồn chính xác; nhánh darwin của script không đổi và chưa chạy.
- Audit: temp `perf-bench-*` và thư mục profile đã xoá; app chủ nhân vẫn LISTENING cổng 4317 (pid 10328); chỉ kill cây process instance tự spawn.

### Commit message đề xuất
Add --profile to perf-bench (renderer CPU profile, DOM nodes per region, store patch rate) and v87 Windows profile
