## WP-v87-4-T5a: Codex rollout reader sang Rust, sau cờ `CCM_CORE_CODEX`

- Spec: `.planning/PLAN-v87-step4-core-transcript.md` (§2 thiết kế, §3 T5, §4 không được phá). Hướng chung: chủ nhân chốt Tauri không Node sidecar → mọi phần đọc file dần sang Rust. Đã có trong `apps/core/crates/core-transcript`: usage (T1), signals (T2: `tail`, `extract`, `sigserve`), tracker (T3: `tracker/`), server NDJSON `serve.rs`. Client TS mẫu: `apps/collector/src/coreTranscript.ts`, `trackerSource.ts` (cờ + lui Node), `coreFallback.ts` (notification 1 lần). Báo cáo T3 / T3b: `.planning/wp/WP-v87-4-T3-report.md`, `WP-v87-4-T3b-report.md`. Tài liệu nguồn Codex: `docs/codex-integration.md`.
- Bản TS tham chiếu: `apps/collector/src/codex/rollout.ts` (parse dòng rollout → `CodexSignal`, redaction), `codex/reader.ts` (`RolloutTailer`), `codex/monitor.ts` (`CodexMonitor`: tìm rollout, tail, state, dựng agent Codex dưới agent Claude), `codex/link.ts`, test trong `apps/collector/test/` (rg `codex`).
- Mục tiêu:
  1. Module Rust `codex` (nhiều file nhỏ) trong `core-transcript`: tail rollout theo offset byte + parse ra đúng `CodexSignal` (cùng redaction: chỉ đếm / id / tên / enum đã cắt, không nội dung). Op mới (vd `codex.open|read|close`, đọc theo trang `maxBytes` / `more` như `signals.*`). Ghi protocol vào plan (mục mới §2.4).
  2. Ranh giới: tối thiểu Rust đọc + parse (bước đệm (a)); nếu state Codex trong `monitor.ts` nhỏ và tách được thì đưa luôn state sang Rust — tự chọn, ghi lý do. Phần process / link (`processes.ts`, `link.ts`) giữ TS ở WP này.
  3. Seam TS ở file mới (vd `codex/coreRollout.ts`, `codex/rolloutSource.ts`): cờ `CCM_CORE_CODEX=rust|node`, mặc định `node` trong WP này; `rust` + core lỗi → TS + 1 notification qua `coreFallback.ts`. `codex/monitor.ts` chỉ đổi chỗ dựng reader (không dài thêm).
  4. Parity: chạy cùng kịch bản test Codex hiện có qua Rust (test mới, vd `codex-parity.test.ts`) → cùng signals / agent snapshot. Thêm fixture rollout lớn (sinh trong thư mục tạm) so TS ↔ Rust theo từng lượt đọc, gồm dòng dở, file bị cắt (truncate) và rollout `--resume`.
  5. Đo nhẹ: CPU-s đọc 1 bộ rollout tổng hợp ~200 MB TS vs Rust (script tạm, ghi số vào báo cáo; không cần perf-bench).
- Phạm vi file: `apps/core/**`, `apps/collector/src/codex/` (file mới + chỗ dựng reader trong `monitor.ts`), `apps/collector/src/coreTranscript.ts` / `coreFallback.ts` nếu cần dùng chung, test mới trong `apps/collector/test/`, `.code_index/{core,collector,_meta}.md`, plan §2.4, `docs/codex-integration.md` (1–2 dòng). Cấm đụng `apps/web/**`, `packages/shared`, `apps/desktop/**` (trừ truyền env `CCM_CORE_CODEX` sang collector nếu bắt buộc — ghi lý do), `scripts/**`.
- Model: opus.
- File nhỏ: mỗi file mới ≤ ~400 dòng; không làm `codex/monitor.ts` dài thêm.
- Chỉ Windows + macOS.
- KHÔNG viết comment trong code (mọi loại: `//`, `/* */`, `///`, docstring). Lý do / invariant ghi vào `.code_index/` hoặc plan.
- Tìm kiếm bằng `rg` / tool Grep, không `grep -r`. Văn bản tiếng Việt: viết `max` không viết "trần", `deadline chờ …` không viết "watchdog".
- Phối hợp: Codex đang chạy WP-v87-4-T4w (đo CPU + E2E). Trước mỗi lần `cargo build` / `cargo test` / vitest nặng: nếu có file `%TEMP%\orrery-bench.lock` thì chờ (kiểm lại mỗi ~2 phút, không vòng lặp dày) tới khi file biến mất. Không sửa `docs/perf/v87-step4-T4-win32.json`.
- Lệnh test (cargo qua PowerShell): trong `apps/core`: `cargo test --locked -j 4`, `cargo clippy --locked -j 4 -- -D warnings` (host) + `--target aarch64-apple-darwin` nếu target đã cài; `npm test -w @ccm/collector -- --maxWorkers=4`; `npm run typecheck -w @ccm/collector`.
- An toàn máy: KHÔNG chạm app Orrery đang chạy của chủ nhân (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`, `~/.codex` thật chỉ được đọc, không ghi); kill chỉ pid tự spawn, không `taskkill /IM`. Máy ≤ 80 % CPU / RAM.
- Không git commit / push.
- Tiêu chí:
  - [ ] Parity Codex TS ↔ Rust trên test hiện có + fixture lớn (dòng dở, truncate, resume).
  - [ ] Cờ mặc định `node` không đổi hành vi; `rust` + core lỗi → TS + 1 notification.
  - [ ] Redaction giữ y; Rust không log nội dung dòng.
  - [ ] Số đo CPU-s TS vs Rust.
  - [ ] cargo test + clippy + collector test + typecheck sạch; index + plan §2.4.
- Nhìn thấy được: không (agent Codex hiển thị phải giống hệt).
- Báo cáo: `.planning/wp/WP-v87-4-T5a-report.md` (đã làm, file đổi, số test, số đo, việc còn lại cho T5b: hooks / statusline / DeepSeek outbox), dòng cuối `STATUS: DONE|PARTIAL|BLOCKED`.
