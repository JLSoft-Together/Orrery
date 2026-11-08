# PLAN v86 — Hiệu năng: boot, gõ trong Workspace, RAM, CPU nhiều agent (Windows + macOS)

Ngày 2026-11-09. Nguồn: 3 gói đo/đọc code song song (boot + collector, typing latency, RAM + Tauri),
main agent đã soát lại các điểm chính trong code. Audit trước: `AUDIT-2026-10-15-performance.md`, `PLAN-v45-performance.md`.

## Kết luận

**Không viết lại bằng Rust/Go/Tauri.** Mọi điểm chậm đo được đều do thuật toán, timer, thứ tự chờ lúc boot
hoặc việc chạy khi không ai xem — viết bằng Rust mà vẫn giữ cách làm cũ thì vẫn chậm y như vậy. Làm 4 gói dưới đây
trên stack hiện tại: khoảng 1.5–2.5 person-week, đổi lại phím trễ giảm còn khoảng một nửa, boot nhanh hơn ~2.4 s,
RAM giảm 80–150 MB.

| Phương án | Effort | RAM private giảm | Gõ phím | Boot | Rủi ro |
|---|---|---|---|---|---|
| **Fix trên stack hiện tại (plan này)** | 1.5–2.5 pw | 80–150 MB | −8–16 ms/phím | −2.4 s | Thấp |
| Collector thành sidecar Rust/Go | 8–12 pw | 55–65 MB | 0 | ~0 | Trung bình |
| napi-rs từng module | 2–4 pw | 0–10 MB | 0 | ~0 | Thấp, được ít |
| Chuyển hẳn sang Tauri v2 | 35–50 pw | 160–230 MB | 0 nếu chưa sửa timer | ~0 | Cao: `<webview>` + `capturePage`, `titleBarOverlay`, E2E macOS không có CDP, migrate localStorage (279 chỗ) và safeStorage |

Tauri trên Windows vẫn dùng WebView2 (Chromium): renderer + GPU giữ nguyên ~280 MB. Phần backend chỉ chiếm ~15 % CPU time của app
(renderer 109 s + GPU 73 s so với collector 21 s + main 11 s + pty 7 s trên app đang chạy).

## Số đo hiện tại (Windows 11)

| Mục | Số |
|---|---|
| App đang chạy: 7 process | private ~530 MB (WS ~975 MB): renderer 137, GPU 138, main 93, collector 79, PTY host 60, network 13, audio 10 |
| ConPTY echo 1 phím | p50 **0.30 ms** |
| `setTimeout(8)` trên Windows | thực tế **15.4–15.9 ms** |
| Ước tính 1 phím trong app | ~25–35 ms (chưa tính Claude Code tự re-render) |
| Collector `/health` warm | 128–141 ms; sessions load xong 263–269 ms |
| Intro cố định trước dashboard | **2 450 + 280 ms** |
| Cold usage scan 4.0 GB | **47.7 CPU-s**, ~18 s, RSS 300–348 MB |
| Demo 30×6 (1 362 agent, tick 250 ms) | 41 % 1 core, main thread bận 18.5 %, 310 lần spawn git / 80 s |
| Demo 6×3 (tải thực tế) | 2.9 % |

## Gói 1 — Gõ phím trong Workspace (ưu tiên cao nhất)

1. **Flush leading edge ở PTY host** — `apps/desktop/src/pty/host.ts:44,191`. Không có timer đang chờ và lần flush trước
   đã cách ≥ 16 ms → `ws.send` ngay (hoặc `setImmediate`); burst vẫn gộp bằng timer. −8–16 ms mỗi phím. ~10 dòng.
2. **Bớt việc trên mỗi chunk output trong renderer:**
   - `feedDev` (`TermPane.vue:113-119`, `devServer.ts:48-66`) chỉ chạy cho preset shell, không chạy cho pane agent.
   - Scan attachment 150 ms (`usePaneAttachments.ts:103-124`, `readScreen` `TermPane.vue:587-603`): debounce chờ hết output,
     chỉ đọc vùng quanh cursor; `store.reconcile` không tạo `byPane` mới khi danh sách không đổi (`attachments.ts:71,155`).
3. **Watcher `deep: true` trên `panes`** (`workspace.ts:558-600`): debounce 300–500 ms trước khi `JSON.stringify` + ghi storage.
4. **Pane ẩn** (`WorkspaceView.vue:1629-1636`, mount bằng `v-show`): gom output, ghi theo đợt lớn khi idle; giữ WebGL cho 4–6 pane
   dùng gần nhất (LRU) thay vì dispose mỗi lần ẩn (`TermPane.vue:525-539`). Clock 1 s (`TermPane.vue:286`) dừng khi pane ẩn.
5. Đo lại bằng Performance trace qua CDP trên dev Electron; so pane shell với pane Claude để tách độ trễ của Claude Code.

## Gói 2 — Boot

1. **Intro không chờ cố định** — `main.ts:738-740`, `pages.ts:6,8`: rời intro ngay khi `/health` + `sessions.loading` xong
   (giữ min ngắn ~400 ms cho animation không bị cắt). −~2.4 s.
2. Spawn collector sớm hơn: trước `createMainWindow` (`main.ts:2096`) thay vì trong `boot()` (`main.ts:732`); song song với
   đăng ký IPC, `preview.start()`, `runManager.loadSettings()`.
3. Lazy `await import()` cho module main ít dùng lúc boot (preview, devices, telegram, layout, flutterPreview, run): RAM main −10–20 MB,
   compile nhanh hơn. Cần đo trước/sau thời gian từ `whenReady` tới window paint (chưa đo được).

## Gói 3 — CPU khi nhiều agent + collector

1. **Cold usage scan** — `apps/collector/src/usage.ts:410-421`: `Buffer.concat([remainder, chunk])` + `Buffer.from` mỗi chunk 64 KB,
   73 % byte nằm trong dòng > 64 KB → bậc hai. Gom danh sách chunk, concat một lần khi gặp newline; `highWaterMark` 1 MB như `tailer.ts:4`.
2. **Office hub** — `office/hub.ts:48,82-90`: `commit` mỗi 200 ms tính lại toàn bộ state + `maskSecrets` + `appendFileSync` timeline kể cả khi
   0 subscriber. Không subscriber → chỉ ghi timeline theo nhịp thưa (vd 5 s) và chỉ khi `rev` đổi; ghi bất đồng bộ. ~26 % thời gian main thread bận ở demo nặng.
3. **gitstat** — `monitor.ts:730-750`, `gitstat.ts:29-31`: debounce theo `cwd` thay vì `terminalId`; cache "không phải repo".
4. Giữ nguyên: koffi process table (fallback PowerShell chỉ khi koffi lỗi), batch 50 ms ở `store.ts:392`.

## Gói 4 — RAM

1. Terminate shiki highlight worker sau ~60 s idle (`lib/highlight.ts:27`): −10–30 MB.
2. `AudioContext.close()` sau ~30 s im lặng (`lib/sound.ts:6`, `office/sound.ts:11`) để Audio service tự thoát: −~10 MB.
3. Tách Monitor / AgentMap / Vue Flow ra async chunk (`AgentMap.vue:54`, `App.vue:27`), entry hiện 1.6 MB: −5–15 MB.
4. Scrollback xterm 5000 → 2000, hoặc thu gọn khi pane ẩn (`TermPane.vue:888`, `DropTerminal.vue:198`): −5–7 MB mỗi pane bận.
5. Cân nhắc sau (có đánh đổi): gộp PTY host vào process collector, −30–40 MB nhưng node-pty crash sẽ kéo chết collector.

## Còn chưa đo

- Thời gian Electron main từ `whenReady` tới window paint (bị single-instance lock của app đang chạy chặn).
- Toàn bộ số đo trên macOS. Đo khi có máy Mac: RSS từng process, typing latency (macOS timer không bị 15.6 ms nên Gói 1.1 lợi ít hơn), boot.
- Cap của activity / transcript store bên renderer.

## Thứ tự làm

Gói 1.1 → 2.1 → 3.1 → 3.2 (mỗi cái vài giờ, lợi lớn nhất) → phần còn lại của Gói 1 → Gói 4 → 2.2–2.3 → 3.3. Đo lại sau mỗi gói,
ghi số vào `docs/perf/`. Chỉ xét lại Rust sidecar nếu sau đó collector vẫn là điểm nghẽn đo được.
