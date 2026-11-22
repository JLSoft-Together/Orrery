# map

Agent Map (Vue Flow: MainNode, SubPill, PeerEdge, TreeEdge), Quỹ đạo (Orbit view / star map / focus / detail) và Board (SessionBoard, BoardCard, BoardDone).

- Đường vào: `@features/map` (`mapTree`, `buildNav`, `parseGroupBy`...). MonitorView nạp `AgentMap`, `OrbitView`, `SessionBoard` trực tiếp.
- File chính: `components/AgentMap.vue`, `components/OrbitStarMap.vue`, `lib/mapText.ts`, `lib/orbitRings.ts`, `lib/starMap.ts`.
- `features/office/components/officeView.test.ts` đọc `components/AgentMap.vue?raw`.
- i18n: `i18n/{map-v47,orbit-v95,peer-map-v54,peer-v54,peer-ws-v54,board-v95}.ts` import tĩnh trong `src/i18n/{en,vi}.ts`.
