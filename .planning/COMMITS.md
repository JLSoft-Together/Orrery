# Commit đề xuất v79 (chưa commit — chờ chủ nhân nói "commit")

Các WP chạm chung file (i18n en/vi, e2e, ui.ts), nên gom 1 commit là an toàn nhất:

🧹 v79 simplify the UI: Settings as a sidebar window with search, Monitor grouped by need with three-line cards, one-line Workspace rail, lean pane headers, one status popover in the top bar, cleaner History / Usage / Devices / Extensions / Git Log, Vietnamese status labels

Nếu muốn tách theo WP (thứ tự, mỗi commit `git add` đúng file trong report WP đó; en.ts / vi.ts / e2e chung đi với WP cuối chạm):
1. 🪟 Settings as a sidebar window with search, ten pages and an events by channel notification table
2. 🧹 Monitor: group sessions by need, three-line cards with a more menu, project filter dropdown and calmer Agent Map
3. 📐 Compact Workspace rail with one-line rows, agent-first project sections, filter box and icon open button
4. 🧹 Simplify workspace pane headers: tab bar hosts the lone pane header, extra actions move to the more menu
5. 🧹 Fold header limits, keep-awake, disk, resources and connection into one status popover, move Compact to the more menu and show only the guide on a fresh profile
6. 🧹 Simplify History, Usage, Devices, Extensions pages and the Git Log branches toolbar
7. 🌐 Vietnamese status labels everywhere and remove the unused session bar, dim toggle and dead strings

Không add: spikes/, bg_native_ad_cta.xml, git-gui.bat, apps/desktop/test/__snapshots__/fwp-host.test.ts.snap, apps/*/.vitest/, .planning/HANDOFF-2026-11-01-v76.md (sửa cũ).

## 2026-11-09 perf (PLAN-v86) — chưa commit
- perf(collector): linear usage scan, idle office commit cadence, coalesced per-cwd git diffstat
  apps/collector/src/{usage.ts,office/hub.ts,gitstat.ts}, apps/collector/test/{usage.test.ts,office-hub.test.ts,gitstat.test.ts}
- perf(web): debounce workspace persist and attachment scan, skip dev-server feed for agent panes, no-op attachment reconcile
  apps/web/src/{components/workspace/TermPane.vue,composables/usePaneAttachments.ts(+test),stores/attachments.ts(+test),stores/workspace.ts(+test),stores/workspaceRestore.test.ts,lib/fileEditor.test.ts}
- perf(web): idle-terminate shiki worker, close idle AudioContexts, lazy-load Monitor and Office view
  apps/web/src/{lib/highlight.ts,lib/highlight.idle.test.ts,lib/sound.ts,lib/sound.test.ts,office/sound.ts,office/sound.test.ts,App.vue,components/AgentMap.vue}, packages/habblaud/client/src/audio/board.ts
- chore: point update feed and README to the renamed Orrery repo
  apps/desktop/src/update/github.ts, README.md
- perf: Office view off by default, collector starts hub and timelapse recorder only when a client enables it in Settings
  apps/collector/src/{index.ts,server.ts,office/runtime.ts,office/hub.ts}, apps/collector/test/{office-runtime.test.ts,office-hub.test.ts}, packages/shared/src/types.ts, apps/web/src/{stores/settings.ts,stores/connection.ts,stores/connection.office.test.ts,components/AgentMap.vue,components/settings/SettingsGeneral.vue,components/settings/index.ts,i18n/en.ts,i18n/vi.ts}
- perf(desktop): create main window and spawn collector earlier in ready()
  apps/desktop/src/main.ts
- perf(workspace): batch hidden pane output, keep WebGL for 4 recent panes, scrollback 3000
  apps/web/src/{lib/termBuffer.ts,lib/termBuffer.test.ts,lib/webglLru.ts,lib/webglLru.test.ts,components/workspace/TermPane.vue,components/workspace/DropTerminal.vue}
- docs: code index + PLAN-v86 performance plan, WP briefs/reports
  .code_index/*, .planning/{PLAN-v86-performance.md,PROGRESS-2026-11-09-perf.md,DECISIONS.md,COMMITS.md,wp/WP-perf-*}
Ghi chú: apps/collector/test/__snapshots__/tracker.test.ts.snap và apps/desktop/test/__snapshots__/fwp-host.test.ts.snap chỉ đổi line ending do vitest — bỏ ra khỏi commit (git checkout -- <file>).

## 2026-11-09 v87 bước 0 + 1 — đã commit + push
- ⚡️ Drop chrome blur: opaque material tokens, design rule updated
  apps/web/src/{style.css,themes.css,components/workspace/CodeEditor.vue}, CLAUDE.md, .planning/AUDIT-v18-deck-reference.md
- ⚡️ Pause the 1 s clock, history poll and tool-span prune while the window is hidden (usePageActive)
  apps/web/src/{composables/usePageActive.ts(+test),components/FlipClock.vue,components/history/HistoryView.vue,stores/toolSpans.ts}
- ⚡️ Renderer under load: per-frame event batching, frozen shallow entity stores, WAAPI list motion, cursor blink only when focused
  apps/web/src/{stores/connection.ts,stores/agents.ts,stores/terminals.ts,stores/toolSpans.ts,lib/patch.ts,lib/motion.ts,lib/stableEdges.ts(+test),components/SessionList.vue,components/AgentMap.vue,components/TreeEdge.vue,components/workspace/TermPane.vue,components/workspace/DropTerminal.vue,stores/connection.batch.test.ts}
- 📈 perf-bench script (--stage, --trace, --profile) and Windows baselines 3.4.0 / v87
  scripts/perf-bench.mjs, docs/perf/*
- 📝 Plan v87, decisions, WP briefs/reports, code index
  .planning/*, .code_index/*

## 2026-11-09 v87 bước 1c — đã commit + push
- ⚡️ Adaptive WS flush (220 ms under load, urgent bypass, flush on window show)
- ⚡️ Rail cards skip layout and paint off-screen (content-visibility)
- ⚡️ Agent Map mounts only visible nodes and edges
- 📈 v87 step 1c benchmarks (3 runs), WP briefs / reports, code index, handoff

## 2026-11-09 v87 bước 1b — đã commit + push
- ⚡️ Heavy-load WS flush 300 ms
  apps/web/src/stores/connection.ts
- 📦 Smaller installer: drop node-pty .pdb and WebGPU-only dxcompiler / dxil on Windows
  apps/desktop/electron-builder.yml (filter), apps/desktop/after-pack.cjs
- 📦 Download the layout daemon jar + loose kit on first XML Preview use (sha256-pinned release asset, CI builds one reproducible zip)
  apps/desktop/{build.mjs,daemon-kit.mjs,electron-builder.yml,src/layout/*,src/main.ts,test/layout-kit.test.ts}, apps/layout-daemon/{build.gradle.kts,README.md}, .github/workflows/{desktop,release}.yml, .code_index/desktop.md
- 📝 WP brief / report, progress, handoff

## 2026-11-09 v87 bước 2: A1 + A2 đã commit, B chưa commit
- ♻️ PTY control contract: PtyBroker in main resolves spawns, cwd, branch and worktrees; Node host keeps PTY + ws and speaks the contract over parentPort or stdio NDJSON
  packages/shared/src/ptyControl.ts, apps/desktop/src/pty/{broker.ts,host.ts,supervisor.ts,spawnWait.ts}, apps/desktop/test/{pty-*.test.ts,fixtures/pty-control.json}
- ♻️ MCP orrery server moves to Electron main: pane tokens map to tool hosts, page / desk calls go straight to onOrrery, PTY host drops MCP and relays
  apps/desktop/src/pty/{mcp.ts,mcpTools/*,broker.ts,supervisor.ts,host.ts}, apps/desktop/test/{mcp-http,mcp-tools}.test.ts, docs/claude-code-integration.md, apps/desktop/e2e/shell-agent.cjs
- 🦀 Scaffold apps/core Rust workspace (core-pty skeleton + orrery-core control stub), buildCore in build.mjs, core/ extraResources per arch, rust steps in CI — CHƯA BIÊN DỊCH (chờ MSVC Build Tools)
  apps/core/**, apps/desktop/{build.mjs,electron-builder.yml}, .github/workflows/*, .gitignore, .code_index/core.md
- 📝 code index (desktop, protocol, _meta), plan step 2, WP briefs / reports, progress
