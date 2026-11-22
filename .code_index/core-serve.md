# core-serve: serve nhúng + core-http + core-store

Đường vào: `apps/core/crates/core-serve/src/lib.rs` (`run`, `run_with`, `run_with_tap`, `main`), gọi từ `apps/shell/src/serve.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-serve/src/config.rs::ServeConfig` | env -> config (`from_vars`), xem `docs/env.md` |
| `apps/core/crates/core-serve/src/runtime.rs` | thread monitor: hàng đợi `Cmd`, prefetch process table, `tick`, `after_tick` |
| `apps/core/crates/core-serve/src/dispatch.rs` | `ClientMessage` -> monitor / store / toaster |
| `apps/core/crates/core-serve/src/host_ops.rs` | pickFolder / focus / open / launch / prompt search ngoài thread |
| `apps/core/crates/core-serve/src/alerts.rs` | toaster + snooze link |
| `apps/core/crates/core-serve/src/sampler.rs` | mẫu Resources (CPU / RAM / port), `latest_resources` cho MCP |
| `apps/core/crates/core-serve/src/sensitive.rs` | Sensitive (deny / settings) watch |
| `apps/core/crates/core-serve/src/procs.rs` | `ProcCache`, `CachedDeps` |
| `apps/core/crates/core-http/src/server.rs::start` | HTTP + WS `127.0.0.1`, `StoreHandle` (`Arc<Mutex<Store>>` + `Notify`), batch 50 ms |
| `apps/core/crates/core-http/src/ws.rs` | mỗi kết nối 1 task: đọc, outbox, heartbeat 30 s |
| `apps/core/crates/core-http/src/client/mod.rs::ClientMessage` | parse + validate message trang gửi |
| `apps/core/crates/core-http/src/cookie.rs` | `GET /?k=<key>` -> cookie `ccm_app`, redirect |
| `apps/core/crates/core-http/src/origin.rs` | Host / Origin / app key (so sánh hằng thời gian) |
| `apps/core/crates/core-http/src/static_files.rs` | phục vụ `apps/web/dist`, SPA fallback |
| `apps/core/crates/core-http/src/tap.rs::StoreTap` | view in-process cho remote gateway |
| `apps/core/crates/core-store/src/store/mod.rs` | `Store`: terminals / agents / links / activity, setter phát event |
| `apps/core/crates/core-store/src/model.rs` | model serde, khớp `packages/shared/src/types.ts` |
| `apps/core/crates/core-store/src/persist/mod.rs` | `JsonFile` (`persist/json_file.rs`) + aliases / favorites / history / snooze / timeline / datadir |

## Bất biến

- Chỉ nghe loopback; mọi request trừ `/health` cần app key (header `x-ccm-app` hoặc cookie); Origin phải là chính app (hoặc cổng Vite khi `--dev`).
- Kết nối mới nhận 1 `snapshot` rồi chỉ event tăng dần; lệch `seq` thì client gửi `resync` (xem `protocol.md`).
- `/health` không chờ lock store (seq nằm trong atomic).
- Thêm field vào model: sửa cả `core-store/src/model.rs` và `packages/shared/src/types.ts`; test `core-store/tests/shared_ts.rs` so khớp.

## Liên kết

- Thêm message WS client -> server: `packages/shared/src/types.ts::ClientMessage`, `core-http/src/client/mod.rs`, `core-serve/src/dispatch.rs`, phía web `apps/web/src/app/stores/connection.ts`.
- Plan: `.planning/PLAN-v87-step5-*.md`. Tiếp theo: `core-monitor.md`, `core-toast-bridge.md`.
