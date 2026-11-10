# WP-v87-5-S2 — báo cáo

Ngày 2026-11-10. Brief `.planning/wp/WP-v87-5-S2.md`. Không commit / push, không sửa `apps/collector/src`, `apps/web`, `apps/desktop`, `packages/shared`, không chạm app Orrery đang chạy (mọi server test bind cổng 0 trên 127.0.0.1, data dir tạm).

## Đã làm

Crate mới `apps/core/crates/core-http` = `apps/collector/src/server.ts` 1:1, không dùng HTTP framework (head đọc bằng `httparse`, 101 tự trả, frame bằng `tokio-tungstenite`):

- `origin.rs`: `host_of`, `is_allowed_origin`, `has_app_key` (`subtle`, constant-time), `is_local_request`.
- `csp.rs`: `content_security_policy` (quét `<script>` tay theo regex Node, CR / CRLF → LF trước sha256, base64 tự viết), `cache_control_for` (chỉ `assets/<tên>-<8 ký tự>.<ext>` immutable).
- `static_files.rs`: gộp dot-segment như URL parser, percent-decode chặt (lỗi / NUL = 400), thoát root = 403, thư mục → `index.html`, thiếu file → SPA `index.html`, không có index → 404 `Web UI not built. Run: npm run build`; HTML đọc cả file + CSP, file khác stream kèm `content-length`; MIME map như Node.
- `http.rs`: đọc head (16 KiB, timeout 10 s), response `Connection: close`, HEAD không body.
- `server.rs`: `start(Config, StoreHandle, Hooks) -> RunningServer`; từ chối host không phải loopback; `AddrInUse` → `StartError::PortInUse`, `PORT_IN_USE_EXIT = 3`; route đúng thứ tự Node (không local 403 → `/health` không cần key `{"ok":true,"seq":N}` → thiếu `x-ccm-app` 403 → `GET /snooze?t&m&k` 204/404 → `POST /focus` body ≤ 2048 → static). `StoreHandle` (`Mutex<Store>` + `Notify`) chạy batch 50 ms bằng tokio (`next_flush_at` / `poll`).
- `ws.rs`: connect = `flush()` + add client + snapshot dưới cùng lock store; `resync` trả snapshot cho riêng socket; broadcast serialize 1 lần cho mọi client; `ClientContext::reply` = `ServerMessage::direct`; heartbeat 30 s (không pong giữa 2 tick → drop); `maxPayload` 128 KiB (quá = close 1009, có drain để client nhận được close frame); echo subprotocol đầu tiên.
- `client/` `parse_client_message`: mọi nhánh `ws.on('message')` của Node (shape, giới hạn độ dài tính theo UTF-16, `sanitize*` của `packages/shared` port sang `client/validate.rs`); `ClientMessage` serialize lại đúng JSON hook Node nhận.
- `control.rs`: stdin NDJSON (`{"type":"shutdown"}` → thoát; stdin EOF cũng thoát để không mồ côi; message khác giữ lại cho S10: `deviceUsed`, `diskAlert`, `agentNote`).
- bin `http-serve` (env `CCM_PORT` / `CCM_WEB_DIST` / `CCM_APP_KEY`, `--dev`): harness cho test process; `orrery-core serve` thật là S10.
- `core-store`: thêm duy nhất `replay::replay_store` (replay ops, trả `Store`) cho golden test. Không sửa `src/store/**`.
- Docs: `.code_index/core.md` (mục core-http), `.code_index/_meta.md`, plan §3.3 + đánh dấu dòng S2 xong.

## File

Mới: `apps/core/crates/core-http/` (`Cargo.toml`; `src/{lib,js,origin,csp,query,http,static_files,server,ws,control}.rs`, `src/client/{mod,validate,tests}.rs`, `src/bin/http-serve.rs`; `tests/{common/mod,hardening,static_files,golden,process}.rs`).
Sửa: `apps/core/Cargo.toml` (1 member), `apps/core/Cargo.lock` (entry `core-http`; dòng `libc` / `windows-sys` của core-store là của S3), `apps/core/crates/core-store/src/replay.rs`, `.code_index/{core,_meta}.md`, `.planning/PLAN-v87-step5-core-store.md`.
Dep mới của crate: `httparse` (đã có trong lock qua tungstenite); còn lại (`tokio-tungstenite`, `futures-util`, `sha2`, `subtle`, `serde*`) đã có sẵn. Mọi file ≤ 382 dòng (test 382, src ≤ 345).

## Test (số thật)

- `cargo test --locked -j 4 -p core-http`: 65 passed, 0 failed = unit 40 + `golden` 1 + `hardening` 15 + `process` 5 + `static_files` 4. `hardening` chạy lặp 10 lần: 10/10 pass (lần đầu `1009` flaky vì RST làm mất close frame, đã sửa bằng drain).
- Cả workspace `cargo test --locked -j 4`: 0 failed (≈ 273 passed, gồm test S3 đang làm dở trong cùng cây).
- `cargo clippy --locked -j 4 --all-targets -- -D warnings`: sạch.
- Không thêm test TS nên không chạy `npm test`; không sửa TS.
- Ánh xạ cổng: `hardening.test` → `tests/hardening.rs` + `tests/static_files.rs` + `src/origin.rs` + `src/client/tests.rs`; `csp.test` → `src/csp.rs` (đủ 4 case CSP + 2 case cache-control); các test `focus` / `folder-pick` / `opener` / `gitstat` / `usage retention` trong `hardening.test.ts` không thuộc server (S4 / S6 / S8). Golden: `tests/golden.rs` replay `monitor.ops.json` tới từng op `snapshot` rồi so frame WS đầu tiên với frame snapshot Node (gồm `seq`) = 0 khác biệt (2 snapshot).

## Khác biệt biên (cố ý)

- Origin ports dùng cổng thật đã bind (Node dùng cổng yêu cầu nên port 0 không khớp Origin thật).
- Từ chối bind host không loopback.
- `toolSpans.get.since` chỉ nhận dạng ISO-8601 (Node nhận mọi chuỗi `Date.parse`); client thật gửi `toISOString`.
- `sanitizeIgnore`: giữ thứ tự chèn cho 200 project đầu (JS đẩy key dạng số lên trước) — chỉ lệch khi > 200 project có key số.
- `/focus` dùng `Transfer-Encoding: chunked` coi như body rỗng; không keep-alive (`Connection: close`).
- `/api/timeline/*` chưa có (S9), đang rơi vào static như Node khi không có `timelineDir`.
- Lỗi ws `error` của Node chỉ giảm count; Rust luôn gọi cả `on_client_close` + `on_client_count`.

## Việc cho WP sau

- S4: monitor gọi `StoreHandle::with(...)` (tự đánh thức flush); `Hooks.on_client_message` nhận `ClientMessage` có kiểu, `ctx.reply(event)` cho direct.
- S10: thêm `serve` vào `core-bin` (gọi `start`, `control::spawn_stdin`, in `{"type":"listening","port":N}`), nối RPC / các message vào qua `ControlIn::Message`.
- S7 / S8: `on_snooze_link`, `on_focus_run` (async) là chỗ cắm.
- S9: hook cho `/api/timeline/*` trước khi rơi vào static.

STATUS: DONE
