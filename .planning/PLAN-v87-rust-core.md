# PLAN v87 — Lõi Rust (`orrery-core`), thay dần từng module

Ngày 2026-11-09. Chủ nhân chốt hướng: **lõi Rust + vỏ mỏng**, xây lại dần từng module, Windows + macOS.
Nối tiếp `PLAN-v86-performance.md` (đã làm 6 WP tối ưu trên stack cũ, chưa commit).

## 1. Mục tiêu

- Giảm tối đa CPU / RAM / GPU mà không đập đi làm lại một lần.
- Sau bước 5: RAM private khoảng **530 → 280–330 MB**, CPU lúc rảnh gần 0, CPU khi nhiều agent giảm 2–3 lần.
- Mỗi bước app vẫn chạy đủ tính năng; module nào lỗi thì quay về bản Node bằng một cờ.
- Lõi dùng chung cho Windows + macOS; sau này mở đường cho vỏ Tauri hoặc UI Swift native trên macOS.

## 2. Không làm trong plan này

- Không viết lại UI Vue (renderer giữ nguyên, chỉ siết GPU/CPU ở bước 1).
- Không đổi vỏ Electron → Tauri (bước 6, quyết sau khi có số đo).
- Không tự viết terminal native thay xterm.js.
- Không đổi protocol mà renderer đang dùng (WebSocket + `packages/shared`).

## 3. Kiến trúc đích

**Process sau bước 5:**
- `Orrery` (Electron main): cửa sổ, tray, menu, IPC với renderer, preview, devices, telegram, github, update… (giữ Node).
- `orrery-core` (Rust, 1 process): PTY, process table, ports, transcript / usage / tracker, store, WebSocket server, hooks, codex.
- Renderer (Vue) + GPU process: như cũ.

**Bỏ đi:** utilityProcess collector (Node) và utilityProcess PTY host (Node).

**Crate trong `apps/core/` (Cargo workspace):**

| Crate | Việc | Thay cho |
|---|---|---|
| `core-pty` | ConPTY / openpty, ring scrollback, flow control, WS `/pty` | `apps/desktop/src/pty/host.ts` |
| `core-procs` | process table, ports, CPU sampler | `win-procs.ts`, `process.ts`, `resources/`, `ps-host.ts` |
| `core-transcript` | tail JSONL, parse, tool spans, usage index | `tailer.ts`, `transcript.ts`, `tracker.ts`, `tool-spans.ts`, `usage.ts` |
| `core-store` | state store, diff, batch 50 ms, WS server snapshot + events | `store.ts`, `server.ts` |
| `core-sources` | hooks, codex rollout, deepseek outbox, git stat, fs watch | `hooks.ts`, `codex/`, `deepseek/`, `gitstat.ts` |
| `core-bin` | binary `orrery-core`, cấu hình qua env `CCM_*` như collector | `index.ts` |

**Thư viện chính:** `tokio`, `axum` + `tokio-tungstenite`, `portable-pty`, `notify`, `sysinfo` + crate `windows` / `libproc`, `gix`, `serde` / `simd-json`, `ts-rs`.

## 4. Nguyên tắc chuyển đổi

1. **Protocol không đổi.** Type TS của message sinh từ Rust bằng `ts-rs`, so khớp với `packages/shared` bằng test; renderer không phải sửa.
2. **Cờ chọn bản chạy cho từng module:** `CCM_CORE_PTY`, `CCM_CORE_PROCS`, `CCM_CORE_TRANSCRIPT`, `CCM_CORE` = `rust` | `node`. Mặc định giữ `node` tới khi module qua đủ cổng nghiệm thu, rồi mới đổi sang `rust`. Bản Node giữ thêm 1 bản release sau khi đổi mặc định, rồi mới xoá.
3. **Test so khớp (parity):** cùng fixture (transcript thật đã ẩn dữ liệu, `scripts/demo-fixture.mjs`), chạy bản Node và bản Rust, so snapshot + chuỗi event. Như cách v45 đã làm cho tracker.
4. **Không mất dữ liệu user:** cùng `CCM_DATA_DIR`, cùng format file (usage cache, aliases, favorites, timeline). Rust đọc được file Node đã ghi và ngược lại trong giai đoạn song song.
5. **Hard rules giữ nguyên:** sanitize session content / secret / tool args trước khi gửi renderer; chỉ bind `127.0.0.1`; `x-ccm-app` key; không gọi LLM / Claude API.

## 5. Build + đóng gói

- Toolchain: Rust stable (pin trong `rust-toolchain.toml`), mỗi OS build bản của nó (giống `node-pty`).
  - Windows: `x86_64-pc-windows-msvc` (thêm `aarch64` sau nếu cần).
  - macOS: `aarch64-apple-darwin` + `x86_64-apple-darwin`, gộp bằng `lipo` hoặc 2 bản theo arch của dmg.
- `npm run desktop` / `dist`: thêm bước `cargo build --release -p core-bin` trước khi đóng gói; bỏ qua được bằng `CCM_SKIP_CORE_BUILD=1` (như `CCM_SKIP_DAEMON_BUILD`). Giới hạn `cargo -j $JIN_MAX_THREADS`.
- `electron-builder.yml`: binary vào `extraResources/core/`; macOS ký + notarize binary cùng app (hardened runtime); Windows ký cùng exe.
- GitHub Actions workflow `desktop`: thêm cache cargo, build trên runner Windows + macOS.
- Supervisor phía Electron: thay `utilityProcess.fork` bằng `child_process.spawn` binary, giữ cơ chế token / port / `/health` / shutdown qua stdin (thay `parentPort`).

## 6. Lộ trình

### Bước 0 — Đo chuẩn (2–3 ngày)
- Script `scripts/perf-bench.mjs`: RSS + private + CPU từng process của app (Windows: koffi / CIM; macOS: `ps` / `proc_pidinfo`), 3 kịch bản: rảnh 5 phút, demo 6×3, demo 30×6. Xuất JSON vào `docs/perf/<version>-<os>.json`.
- Trace GPU: Chrome tracing qua CDP trên dev Electron, tìm nguồn của 73 s GPU (blur, animation, WebGL terminal).
- Cổng: có số chuẩn Windows; macOS chạy được khi có máy / runner.

### Bước 1 — Siết GPU + renderer (1–2 tuần, chưa cần Rust)
- Bỏ / giảm `backdrop-filter` trên chrome layer (giữ được bằng setting nếu chủ nhân muốn), không chạy animation lặp khi rảnh.
- Cửa sổ ẩn / thu nhỏ / bị che: dừng vẽ (Electron `backgroundThrottling`, `visibilitychange`), dừng clock / timer UI.
- Virtualize list dài (activity, session list, git log nếu chưa).
- Cổng: GPU + renderer CPU lúc rảnh giảm ≥ 70 % so với bước 0; không mất animation theo rule chủ nhân (animation chạy khi có thay đổi, không chạy lặp khi rảnh).

### Bước 1b — Giảm size cài đặt trên Electron (3–5 ngày)
- Hiện tại (v3.4.0): Setup / Portable 146 MB, dmg 147–152 MB; cài xong 406 MB, trong đó runtime Electron/Chromium ~310 MB.
- Layout daemon (jar 30 MB + loose kit 10 MB) thành gói tải về lần đầu bật XML Layout Preview, có checksum, lưu vào `CCM_DATA_DIR`.
- NSIS / dmg nén mạnh hơn (`compression: maximum`), đo thời gian cài trước/sau.
- Bỏ `dxcompiler.dll` + `dxil.dll` (~26 MB, chỉ dùng cho WebGPU; app dùng WebGL) — chỉ làm nếu test WebGL terminal + Office + Agent Map trên máy thật vẫn ổn.
- Web assets: chỉ đóng gói grammar shiki cho ngôn ngữ hay dùng, phần còn lại tải lười trong cùng app (không lấy từ network).
- Cổng: Setup ≤ 115 MB, mọi tính năng chạy được (Layout Preview tải đúng lần đầu).

### Bước 2 — `core-pty` thay PTY host (2–3 tuần)
- Renderer đã nói chuyện thẳng với PTY host qua WebSocket `ws://127.0.0.1:<port>/pty` + subprotocol token (`PTY_PROTOCOL_PREFIX`), nên chỉ cần thay binary, giữ đúng message.
- Làm: spawn shell / agent preset, resize, write, ring scrollback 256 KB, replay, backpressure, census, owners, flush **leading edge** (bỏ timer 8 ms → hết 15 ms trễ trên Windows).
- **MCP `orrery`** (`pty/mcp.ts`, `mcpTools/`) và relay `open` / `mcp` lên main: giai đoạn này **chuyển MCP HTTP endpoint sang Electron main** (logic TS giữ nguyên), Rust chỉ lo PTY. Tránh port 2 thứ một lúc.
- `shellInit` / `shim` / `presets` (env, zsh wrapper, OSC 7): Rust đọc cùng file cấu hình / script đã sinh sẵn.
- Cổng: test parity cho từng message; E2E CDP mở pane shell + Claude, gõ, resize, đóng, `--resume`; độ trễ phím p50 ≤ 5 ms phía host; RSS `orrery-core` ≤ 20 MB với 8 pane.

### Bước 3 — `core-procs` (1–2 tuần)
- Process table, ports (LISTEN / ESTABLISHED), CPU sampler; Windows Toolhelp32 + `GetExtendedTcpTable`, macOS `libproc`.
- Giai đoạn đầu: collector Node gọi `orrery-core` qua local HTTP / WS để lấy process table (thay koffi + PowerShell fallback).
- Cổng: kết quả trùng bản koffi trên cùng máy; thời gian 1 lần quét ≤ 20 ms.

### Bước 4 — `core-transcript` (4–6 tuần, phần nặng nhất)
- Tail JSONL, parse dòng, tool spans, tracker trạng thái agent / subagent, usage index + cache.
- Giai đoạn đầu: Rust làm nguồn dữ liệu, đẩy record đã chuẩn hoá cho collector Node (Node chỉ còn store + server).
- Cổng: parity snapshot với fixture thật (main 27 MB + 41 subagent) và demo 30×6; cold usage scan 4 GB ≤ 5 CPU-s; CPU demo 30×6 giảm ≥ 50 % so với bước 0.

### Bước 5 — `core-store` + `core-sources`, bỏ Node collector (4–6 tuần)
- Store, diff, batch, WS server snapshot + events, hooks, codex, deepseek, gitstat, favorites, aliases, history, toast / alert, office hub (đang tắt mặc định).
- Đổi `CCM_CORE=rust` mặc định; giữ bản Node 1 bản release làm dự phòng rồi xoá.
- Cổng: toàn bộ test collector chuyển thành test parity / test Rust tương đương; E2E desktop pass trên Windows + macOS; RAM tổng đạt mục tiêu mục 1.

### Bước 6 — (Tuỳ chọn) vỏ Tauri
- Chỉ quyết sau bước 5, dựa trên số đo. Đây là cách duy nhất để size cài đặt xuống dưới 50 MB (ước Windows 20–35 MB vì dùng WebView2 có sẵn trên Win10/11, macOS 15–30 MB). Vướng: `<webview>` + `capturePage`, `titleBarOverlay`, migrate localStorage (279 chỗ) + safeStorage, E2E macOS không có CDP.

## 7. Rủi ro

| Rủi ro | Cách giảm |
|---|---|
| Lệch hành vi giữa Node và Rust (status agent sai) | Test parity bắt buộc, cờ quay về Node, giữ bản Node 1 bản release |
| ConPTY / openpty khác node-pty (flow control, resize, kill tree) | Bước 2 đo kỹ trên cả 2 OS, giữ `CCM_CORE_PTY=node` dự phòng |
| Format Claude Code / Codex đổi khi đang có 2 bản parser | Fixture theo version trong `docs/claude-code-integration.md`; sửa 2 bản tới khi bỏ Node |
| Build phức tạp hơn (Rust + Node + Gradle) | `CCM_SKIP_CORE_BUILD`, cache cargo trên CI, script kiểm toolchain như layout-daemon |
| Ký / notarize binary mới trên macOS | Thêm vào workflow ngay từ bước 2 |
| Người sửa app phải đọc Rust | `.code_index/core.md` + README crate, giữ module nhỏ |

## 8. Chủ nhân đã chốt (2026-11-09)

1. Cài Rust trên máy dev (Windows) và thêm vào CI macOS — đồng ý.
2. Bước 1 được giảm / bỏ blur trên toolbar + sidebar — đồng ý.
3. Bước 2: chuyển MCP server `orrery` từ PTY host sang Electron main — đồng ý.
4. Commit 6 WP của PLAN-v86 trước — đã commit (`8ba9d71`…`816f047`).
5. Giảm cả size cài đặt — thêm bước 1b; giảm mạnh chỉ có ở bước 6 (Tauri).

## 9. Theo dõi

- Mỗi bước: PLAN con `.planning/PLAN-v87-<n>-*.md`, PROGRESS + bảng WP theo `manager-work-agent`.
- Số đo sau mỗi bước vào `docs/perf/`, so với bước 0.
- `.code_index/core.md` mới cho `apps/core/`; cập nhật `desktop.md` (supervisor spawn binary) và `collector.md` (module nào đã sang Rust).
