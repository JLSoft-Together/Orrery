## WP-v87-0 — DONE
- Đã đổi / tạo: `scripts/perf-bench.mjs` (mới), `docs/perf/README.md` (mới), `docs/perf/3.4.0-win32.json` (4 kịch bản, kèm trace summary), nhật ký trong `.planning/wp/WP-v87-0.md`. Không đụng `apps/`, `packages/`.
- Bản đo: `C:\Program Files\Orrery\Orrery.exe` 3.4.0, Windows 11, 8 core, 48 GB. Mỗi kịch bản: settle 20 s rồi lấy mẫu 120 s (tick 2 s); instance riêng (user-data-dir, CCM_PORT, CCM_DATA_DIR, CLAUDE_CONFIG_DIR, CODEX_HOME, CODEWHALE_HOME temp), BelowNormal, không cờ tắt backgrounding. CPU = % của 1 core.

### Bảng số chuẩn (private MB avg / CPU avg % / CPU p95 %)
| Kịch bản | TOTAL | main | renderer | gpu | collector | pty-host | pty-child |
|---|---|---|---|---|---|---|---|
| idle-monitor | 314 / 0.3 / 1.5 | 81 / 0.1 | 78 / 0.1 | 74 / 0.0 | 69 / 0.1 | - | - |
| idle-workspace (4 shell) | 702 / 1.5 / 5.4 | 84 / 0.1 | 98 / 0.6 / 1.5 | 101 / 0.5 / 2.3 | 68 / 0.1 | 73 / 0.2 | 8 proc, 265 MB, 0.1 |
| demo-6x3 | 435 / 33.9 / 57.9 | 80 / 0.0 | 150 / 16.6 / 28.8 | 114 / 14.5 / 25.4 | 79 / 2.7 / 6.2 | - | - |
| demo-30x6 | 871 / 157.9 / 197.8 | 81 / 0.4 | 539 / 116.7 / 139.3 | 123 / 25.3 / 41.8 | 115 / 15.3 / 25.2 | - | - |
(network process 13.6 MB, ~0% ở mọi kịch bản; max private demo-30x6: renderer 589 MB, TOTAL 926 MB.)

### Top nguồn GPU / renderer CPU (trace 25 s; bundle minified 3.4.0, source hiện tại có thể đã đổi sau v86)
Idle (không có session): gần như không vẽ. Renderer 1 frame/s, GPU 0 `DrawAndSwap`.
1. Clock 1 s (`dk` index:5:37224 → `apps/web/src/lib/format.ts:7-18`, `setInterval(tick, 1_000)`): TimerFire 1/s kéo theo BeginMainFrame + UpdateLayoutTree + Paint + Commit 1/s dù màn rảnh (idle-monitor). Chi phí ~0.4 ms/s.
2. Workspace rảnh 4 shell: renderer 5.8 frame/s, GPU `DrawAndSwap` 1.64/s. Nguồn: xterm `refreshRows` ← rAF (WorkspaceView:69:38677 và :80:23867, 3.3 rAF/s) = `cursorBlink: true` ở `apps/web/src/components/workspace/TermPane.vue:907` (và `DropTerminal.vue:194`); cộng clock 1 s. GPU role 0.5% CPU.
3. demo-6x3 (có session đang busy): renderer 35 frame/s, GPU 48 `DrawAndSwap`/s, RasterTask 114/s, Paint 120/s. Nguồn rAF: (a) `L` index:110:3160 = pulse dot chạy dọc edge, `apps/web/src/components/TreeEdge.vue:60-72` (`step` rAF + `setAttribute('transform')`), 23 rAF/s; (b) Vue `Transition`/`TransitionGroup` `d1` (nextFrame 2 rAF) + `h1` (setTimeout) trong `onLeave` index:2:73775 / 2:74381: mỗi add/remove row bắt 2 rAF + timer 1400 ms (TimerInstall timeout 1400 singleShot, ~10/s); (c) `ccm-spin` SVG spinner (`StatusIcon.vue`, `style.css`) chạy liên tục; (d) message WS `Y.Be.onmessage` index:8:22085.
4. demo-30x6: renderer main thread busy 767 ms/s (CrRendererMain), `FunctionCall` 203/s, rAF 147/s, Paint 402/s, EventDispatch 109/s, MajorGC 0.24/s; top FunctionCall `(anon)` index:2:73775 (Transition onLeave) 14 ms/s, `L` TreeEdge 5.8 ms/s, `Y.Be.onmessage` 4.9 ms/s. DOM 6472 node. Renderer private 539 MB (so với 98 MB idle-workspace).
5. GPU: `SkiaOutputSurfaceImplOnGpu::FinishPaintRenderPass` 64–91 ms/s trong CrGpuMain (demo), theo số frame; thread `GpuVSyncThread` báo 40–995 ms/s nhưng là chờ vblank (CPU gpu role chỉ 14–25%), không phải tải thật.
6. `backdrop-filter` (saturate(1.8) blur(20px), class `ccm-material`): 3–4 phần tử luôn có (titlebar 1360x41 và 3 nút/chip nhỏ). Không thấy CSS animation infinite; running animation: `ccm-spin` (1–2), `ccm-breathe` trên node (1).
7. Idle renderer: `V8.GCFinalizeMCReduceMemory` ~0.2 MajorGC/s (memory-reducing GC) ở idle-monitor.
Chưa so được với số của app chủ nhân (109 s renderer / 73 s GPU trong 1.5 h = 1.7% / 1.1% avg): instance đo không có dữ liệu thật, nên idle thấp hơn; khoảng cách đến từ lúc có session busy (demo-6x3 đã 16.6% / 14.5%).

### Tiêu chí: 4/4 đạt
- [x] Script chạy 1 lệnh, in bảng + ghi JSON: `node scripts/perf-bench.mjs --scenario all --seconds 120 --trace --label 3.4.0 --out docs/perf/3.4.0-win32.json`
- [x] `3.4.0-win32.json` đủ 4 kịch bản, failures rỗng
- [x] Top nguồn GPU + renderer kèm file:line (xem trên)
- [x] App chủ nhân không bị ảnh hưởng: chỉ kill cây process của instance tự spawn; không đụng %APPDATA%\Agent Monitor, cổng 4317 (vẫn LISTENING pid 10328 sau khi chạy), %LOCALAPPDATA%\ccm; temp xoá hết.

### Audit / Review / CHƯA KIỂM
- Audit: temp `perf-bench-*` đã xoá, log/smoke của WP xoá, không còn process con của instance đo.
- Review: không comment trong code, không dependency mới (dùng `ws` có sẵn), CPU/RAM thấp (max 1 core đo + sampler BelowNormal).
- CHƯA KIỂM: nhánh darwin (`ps`) chỉ viết, chưa chạy (không có Mac). Kịch bản workspace dùng store `workspace.open('shell')` qua CDP, 4 pane shell cwd = temp.
- Lưu ý: `idle-monitor` trên 3.4.0 chưa vẽ office canvas trong instance trống (canvases: []), nên số office chỉ lộ khi có session. Số CPU instance đo thấp hơn app thật do không có dữ liệu thật. Trace raw không giữ (xoá cùng temp).

### Commit message đề xuất
Add perf-bench script and 3.4.0 Windows baseline (RAM, CPU per process role, Chromium trace summary)
