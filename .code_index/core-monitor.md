# core-monitor: quan sát Claude / Codex / DeepSeek

Đường vào: `apps/core/crates/core-monitor/src/monitor/mod.rs` (`Monitor`, `tick(&mut Store)`), dựng ở `core-serve/src/runtime.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-monitor/src/monitor/reconcile.rs` | đối chiếu process + registry -> terminal / agent |
| `apps/core/crates/core-monitor/src/monitor/agents.rs` | gắn `CodexMonitor` / DeepSeek vào `Monitor` |
| `apps/core/crates/core-monitor/src/monitor/actions.rs` | hành động session (focus, open, launch, stop) |
| `apps/core/crates/core-monitor/src/monitor/deps.rs` | `Deps`, `SystemDeps`, `MonitorStores` |
| `apps/core/crates/core-monitor/src/native/mod.rs` | `NativeDeps`: focus / open / launch / pick chạy thẳng trên máy |
| `apps/core/crates/core-monitor/src/tracker.rs::MonTracker` | bọc `core_transcript::tracker::Tracker`, trang 4 MiB |
| `apps/core/crates/core-monitor/src/registry.rs::read_registry` | đọc registry session của Claude Code |
| `apps/core/crates/core-monitor/src/peers.rs::PeerLinkStore` | link session với session |
| `apps/core/crates/core-monitor/src/usage.rs` | usage index theo ngày / model / project |
| `apps/core/crates/core-monitor/src/host/protocol.rs` | `host.req` / `host.res` khi `CCM_HOST_RPC=1` |
| `apps/core/crates/core-transcript/src/tracker/mod.rs` | tracker transcript (`view.rs`, `subs.rs`, `read.rs`) |
| `apps/core/crates/core-transcript/src/signal.rs` | tín hiệu waiting / done / stuck từ transcript |
| `apps/core/crates/core-transcript/src/sidefiles/mod.rs` | thư mục hook / statusline bridge + outbox DeepSeek |
| `apps/core/crates/core-transcript/src/mask.rs` | `mask_secrets` |
| `apps/core/crates/core-sources/src/resources/mod.rs` | process CPU / RAM / port theo session |
| `apps/core/crates/core-sources/src/sensitive/mod.rs` | phân loại truy cập nhạy cảm |
| `apps/core/crates/core-sources/src/prompt_search/mod.rs` | tìm prompt (NFC, bỏ dấu) |
| `apps/core/crates/core-procs/src/lib.rs` | `table()`, `usage()`, `cmdline()`, `ports()` theo OS |

## Bất biến

- Danh tính terminal / session lấy từ process / session, không từ title.
- Ưu tiên hook và session file chính thức; không scrape ANSI. Không đoán định dạng chưa có trong `docs/claude-code-integration.md` / `docs/codex-integration.md`.
- Không hiện token count khi không có dữ liệu tin cậy; input = input + cache_creation, total = input + output, dedupe theo `message.id`.
- `ActivityEvent.data` chỉ chứa nhãn, không có tham số tool thô (ngoại lệ `tool.span` path: docs §11ar).
- Timer là "due time" (`next_due()`), không có timer ngầm; mọi method nhận `&mut Store`.

## Liên kết

- Docs: `docs/claude-code-integration.md`, `docs/codex-integration.md`, `docs/deepseek-integration.md`. Plan: `.planning/PLAN-v87-step4-core-transcript.md`.
- Phía web: `web-monitor.md`. Toast: `core-toast-bridge.md`.
