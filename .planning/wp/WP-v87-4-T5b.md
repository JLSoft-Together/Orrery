## WP-v87-4-T5b: hooks + statusline + DeepSeek outbox sang Rust, sau cờ `CCM_CORE_SIDEFILES`

- Spec: `.planning/PLAN-v87-step4-core-transcript.md` (§2, §2.4 cách làm của T5a, §3 T5, §4 không được phá). Mẫu gần nhất cần bắt chước: T5a (commit `6721d74`) — module Rust `apps/core/crates/core-transcript/src/codex/`, seam TS `apps/collector/src/codex/{rolloutSource,coreRollout,nodeRollout}.ts`, fallback `apps/collector/src/coreFallback.ts`, test `apps/collector/test/codex-parity.test.ts`. Báo cáo T5a: `.planning/wp/WP-v87-4-T5a-report.md`.
- Bản TS tham chiếu: `apps/collector/src/hooks.ts` (`parseHookRecord`, đọc / dọn file hook JSON, `COMPACT_STALE_MS`), `apps/collector/src/statusline.ts` (`StatusLineRecord`, đọc / dọn file statusline), `apps/collector/src/deepseek/outbox.ts` (đọc outbox DeepSeek). Xem chỗ gọi trong `monitor.ts` / `deepseek/monitor.ts` và test hiện có (`rg -l "hooks|statusline|outbox" apps/collector/test`).
- Mục tiêu:
  1. Module Rust (file nhỏ, ≤ ~400 dòng / file) trong `core-transcript`: op đọc thư mục hook / statusline / outbox → trả record đã parse, đúng y kiểu TS (validate regex tên file, `MAX_BYTES` 4096, `TOOL_RE`, ISO date, `pct` 0–100…), và op dọn file cũ (`KEEP_MS`) chỉ trong đúng thư mục đã cấu hình. Ghi protocol vào plan §2.5.
  2. Seam TS file mới; cờ `CCM_CORE_SIDEFILES=rust|node`, mặc định `node`; `rust` + core lỗi → TS + 1 notification qua `coreFallback.ts` (thêm kind `sidefiles`). Desktop: thêm `sidefiles` vào `CORE_READERS` + `CORE_READERS_NODE_DEFAULT` trong `apps/desktop/src/main.ts` và chuỗi `coreFallbackSidefiles` (en + vi) trong `apps/desktop/src/strings.ts` (làm giống cách `codex` đã thêm). File gốc (`monitor.ts`, `deepseek/monitor.ts`) chỉ đổi chỗ dựng reader, không dài thêm.
  3. Parity: chạy test hiện có của hooks / statusline / outbox qua cả 2 đường (test mới, vd `sidefiles-parity.test.ts`) + fixture lạ (JSON hỏng, file > 4096 byte, tên file sai, ngày sai, số ngoài khoảng, file biến mất giữa chừng) → cùng kết quả.
  4. Thêm dòng `CCM_CORE_SIDEFILES` vào danh sách env trong `CLAUDE.md` (cạnh `CCM_CORE_CODEX`).
- Phạm vi file: `apps/core/**`, `apps/collector/src/` (file mới + chỗ dựng reader), `apps/collector/test/` (test mới), `apps/desktop/src/main.ts` + `strings.ts` (chỉ phần fallback nói trên), `.code_index/{core,collector,_meta}.md`, plan §2.5, `CLAUDE.md` (1 dòng env). Cấm đụng `apps/web/**`, `packages/shared`, `scripts/**`.
- KHÔNG viết comment trong code (mọi loại: `//`, `/* */`, `///`, docstring). Lý do / invariant ghi vào `.code_index/` hoặc plan.
- Tìm kiếm bằng `rg`, không `grep -r`. Văn bản tiếng Việt: viết `max` không viết "trần", `deadline chờ …` không viết "watchdog".
- Lệnh test (PowerShell): trong `apps/core`: `cargo test --locked -j 4`, `cargo clippy --locked -j 4 -- -D warnings` (+ `--target aarch64-apple-darwin` nếu target đã cài); `npm test -w @ccm/collector -- --maxWorkers=4`; `npm test -w @ccm/desktop -- --maxWorkers=4`; `npm run typecheck -w @ccm/collector`; `npm run typecheck -w @ccm/desktop`. Test parity cần core build debug: `cargo build --locked -j 4 -p core-bin` (xem `codex-parity.test.ts` cách tìm binary).
- An toàn máy: KHÔNG chạm app Orrery đang chạy của chủ nhân (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); thư mục hook / statusline / outbox thật của chủ nhân chỉ được đọc, test chỉ dùng thư mục tạm (op dọn file không được chạy trên thư mục thật). Kill chỉ pid tự spawn, không `taskkill /IM`. Máy ≤ 80 % CPU / RAM, mỗi lúc 1 việc nặng.
- Không git commit / push. Không build dist.
- Tiêu chí:
  - [ ] Parity TS ↔ Rust trên test hiện có + fixture lạ.
  - [ ] Cờ mặc định `node` không đổi hành vi; `rust` + core lỗi → TS + 1 notification + dòng Diagnostics.
  - [ ] Dọn file chỉ trong thư mục đã cấu hình, đúng `KEEP_MS`.
  - [ ] cargo test + clippy + collector + desktop test + typecheck sạch; index + plan §2.5.
- Báo cáo: `.planning/wp/WP-v87-4-T5b-report.md` (đã làm, file đổi, số test, khác biệt biên đã biết), dòng cuối `STATUS: DONE|PARTIAL|BLOCKED`.
