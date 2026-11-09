## WP-v87-nolinux: bỏ hẳn hỗ trợ Linux khỏi app
- Bối cảnh: chủ nhân 2026-11-09: "Hiện tại anh chỉ build app desktop win/mac os thôi" → "bỏ hẳn phần Linux đi em". App chỉ ship Windows (NSIS / Portable) + macOS (dmg). Main đã bỏ backend Linux trong Rust (`apps/core/crates/core-procs`: xoá `linux.rs`, `procfs.rs`).
- Việc: tìm mọi nhánh / code chỉ để chạy app trên Linux trong `apps/collector/src`, `apps/desktop/src`, `apps/web/src`, `packages/shared/src`, `scripts/` và bỏ: vd `'linux'` trong danh sách platform hỗ trợ (`config.ts` verifyProcesses, `opener.ts`, `process.ts`, `resources/ports.ts`, `resources/procs.ts`, `codex/processes.ts`, `focus.ts`), data dir XDG (`XDG_DATA_HOME` / `~/.local/share/ccm`), `xdg-open`, hàm / nhánh chỉ phục vụ Linux. Platform khác Windows / macOS → trả `unsupported` / null như nhánh "khác" hiện có, không crash.
- KHÔNG bỏ: Flutter / Android run target `linux` (`apps/desktop/src/run/flutter/detect.ts`, `apps/web/src/lib/runUi.ts` — đó là platform của project người dùng); job CI `daemon` chạy trên ubuntu (máy build zip, không phải sản phẩm); nhánh POSIX dùng chung cho macOS (`ps`, `lsof`, `cfg(unix)`) — chỉ bỏ phần riêng Linux; test dùng chuỗi `'linux'` chỉ như tham số platform giả để kiểm logic phân biệt hoa thường (sửa sang `'darwin'` nếu ý nghĩa giữ nguyên, hoặc bỏ test chỉ kiểm nhánh Linux).
- Docs: `CLAUDE.md` (dòng env / data dir nếu nhắc Linux), `docs/*.md` chỗ nói Linux, `.code_index/{collector,desktop,web,protocol}.md`. KHÔNG sửa `.code_index/core.md` và `apps/core/**` (WP-T1 đang sửa; main tự cập nhật).
- Phạm vi file: như trên + test liên quan. Cấm đụng `apps/core/**`, `scripts/{demo-fixture,perf-bench,usage-corpus}.mjs` (WP-T0 Codex đang sửa), `apps/collector/src/{coreTranscript.ts, usage*}` và `apps/collector/src/index.ts` (WP-T1 đang sửa) — gặp Linux ở đó thì ghi báo cáo.
- Model: sonnet.
- Luật: không commit; không comment trong code; file nhỏ; tìm bằng `rg`; vitest `--maxWorkers=4`; không build Electron / dist.
- Lệnh test: `npm test -w @ccm/collector`, `-w @ccm/desktop`, `-w @ccm/web` (`--maxWorkers=4`), typecheck 3 package.
- Tiêu chí:
  - [ ] `rg -n "linux|xdg|XDG_" apps/collector/src apps/desktop/src apps/web/src packages/shared/src` chỉ còn chỗ Flutter / run target (liệt kê trong báo cáo).
  - [ ] Windows + macOS không đổi hành vi (test pass).
  - [ ] Docs + index cập nhật.
- Báo cáo: `.planning/wp/WP-v87-nolinux-report.md` (≤ 18 dòng: đã bỏ gì theo file, chỗ giữ lại + lý do, test).

## Nhật ký
- Bước 1-2: bỏ nhánh Linux trong collector (datadir, opener, config, process, ports, procs, codex/processes, deepseek/processes), desktop agentData/move, scripts bridge, comment; sửa test.
- Xong: test pass, report ghi.
