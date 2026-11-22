# rust-core: workspace apps/core

Đường vào: `apps/core/Cargo.toml` (workspace 15 crate, release `opt-level = "s"`, lto, `panic = "abort"`), bin `apps/core/crates/core-bin/src/main.rs` (`orrery-core`).

## Subcommand của `orrery-core`

| Lệnh | Vào crate |
|---|---|
| `serve [--dev]` | `core_serve::main` (xem `core-serve.md`); vỏ Tauri không spawn lệnh này mà link `core-serve` trực tiếp |
| `desk` | `core_desk::stdio::run` (NDJSON, dùng cho test / tool ngoài) |
| `procs` | `core_procs::serve::run` |
| `transcript` | `core_transcript::serve::run` |
| (mặc định) | `core_pty::server::run` (chế độ pty, vỏ spawn làm process con) |

## Crate

| Crate | Làm gì | File index |
|---|---|---|
| `core-store` | model + `Store` + persist JSON | `core-serve.md` |
| `core-http` | HTTP + WS + cookie + origin + static | `core-serve.md` |
| `core-serve` | `orrery-core serve`, ráp store + monitor + toast | `core-serve.md` |
| `core-monitor`, `core-transcript`, `core-sources`, `core-procs` | quan sát Claude / Codex / DeepSeek | `core-monitor.md` |
| `core-pty` | hub PTY, ConPTY, viewers | `core-pty.md` |
| `core-desk` | mọi kênh `ccm:*` của trang | `core-desk.md` và `core-desk-*.md` |
| `core-disk` | quét ổ, index, dupes, MFT | `core-disk.md` |
| `core-toast`, `core-bridge` | toast + statusline / hook bridge | `core-toast-bridge.md` |
| `core-remote-proto`, `core-remote-wasm` | giao thức Noise cho điện thoại / PWA | `shell-remote.md`, `pwa.md` |

## Build và test

- `cargo test --locked -j 4` trong `apps/core` (PowerShell trên Windows: Git Bash có `link` che `link.exe`). Cần MSVC build tools.
- Tool chạy tay: `sources-cli`, `monitor-script`, `toast-script`, `store-replay`, `persist-cli`, `http-serve` (bin trong từng crate, harness parity cũ).
- Dist: `scripts/dist-tauri.mjs` build `core-bin` + `core-bridge` làm sidecar cho vỏ.

## Bất biến

- Mọi dữ liệu ra trang phải qua mask / sanitize (`core-transcript/src/mask.rs`, `packages/shared/src/secretMask.ts`); không log byte terminal, env, token, đường dẫn thô.
- Lock store luôn theo thứ tự store rồi clients; việc nặng (đọc transcript, process table) chạy ngoài lock (`core-serve/src/runtime.rs`).
- Định dạng file persist giữ ổn định (JSON compact, `<file>.tmp` rồi rename).

## Liên kết

- Plan: `.planning/PLAN-v87-*.md` (port Rust), `.planning/PLAN-v110-modules.md`. Lịch sử theo WP: `.code_index/archive/core-v87.md`.
- Rust conventions: skill `rust-conventions`.
