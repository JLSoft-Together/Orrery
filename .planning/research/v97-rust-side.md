# v97 — khảo sát phía Rust (2026-11-12)

Subagent sonnet khảo sát, main lưu lại. Path dưới `apps/`.

## 1. core-serve / core-http
- `core_serve::run_with` chạy **trong process vỏ** trên thread `core-serve` (`shell/src/serve.rs:66`); `run`/`run_with`/`serve` ở `core/crates/core-serve/src/lib.rs:53,57,61`. Runtime `new_current_thread` (`lib.rs:105`) ⇒ gateway nên có runtime / thread riêng.
- HTTP tự viết: `httparse` + `TcpListener` + `tokio-tungstenite 0.30` (feature `handshake`). Accept loop `core-http/src/server.rs:211`, `connection` :233, `upgrade` :262; WS handshake tay `core-http/src/ws.rs:129-138`.
- `bind_address` chặn non-loopback (`server.rs:154-161`, `NotLoopback`) ⇒ listener LAN **không** dùng lại `core_http::start`.
- `/ws` chỉ khi `local(request)` (Host/Origin, `origin.rs`). Auth: `x-ccm-app` (`cookie.rs:16-21`) hoặc cookie `ccm_app` (`cookie.rs:44-52`).
- Fanout: `StoreHandle` (`server.rs:79-119`, `Arc<Mutex<Store>>` + `Notify` + seq); `store.subscribe(|m| clients.broadcast(m))` (`server.rs:170`, `core-store/src/store/batch.rs:57`); `Clients::broadcast` (`ws.rs:73`) serialize 1 lần → outbox mpsc; `flush_loop` `server.rs:188`; client mới: flush → `snapshot_message()` → live (`ws.rs:143-156`); `resync` (`ws.rs:105`).
- Gateway subscribe được trong process qua `StoreHandle::with(|s| s.subscribe(..))` + `snapshot_message()`, nhưng `StoreHandle` / `Clients` private, `run_with` không trả ra ⇒ **phải mở API** (hook / return value). Nên làm `ws::run` / `Clients` generic theo `AsyncRead+AsyncWrite` để dùng cho stream TLS / Noise. Lệnh client vào monitor qua `Cmd::Client((ClientMessage, ClientContext))` (`lib.rs:202`); `ClientContext.tx` private.

## 2. Crate trong lock (core | shell)
- Có: tokio 1.54.1, tungstenite / tokio-tungstenite 0.30, rustls 0.23.46 + ring 0.17.14 (chỉ bắc cầu qua ureq / reqwest), ureq 3.5.1, getrandom, base64, sha2, hmac, rand, zeroize; shell thêm reqwest 0.13.5, hyper 1.12, aes-gcm 0.10.3, keyring.
- Chưa có: tokio-rustls, rcgen, snow, x25519 / ed25519-dalek, chacha20poly1305, mdns-sd, qrcode. QR: `core-desk/src/devices/qr.rs` có encoder tự viết (xem dùng lại được không).
- 2 lockfile riêng; crate thêm vào core-serve xuất hiện ở cả 2.

## 3. Lệnh client của core-serve
- Parse `core-http/src/client/mod.rs:204-322`, dispatch `core-serve/src/dispatch.rs:30-118`: terminal.* (alias, note, snooze, dismiss, open, focusWindow), favorite.*, folder.pick, history/timeline/heatmap/toolstats/diagnostics/tool-spans.get, job.*, prompt-search.*, client.presence, notify.*, sensitive.*, office.*, resync. Reply chỉ trả người hỏi (`ws.rs:35`). **Không có** lệnh publish sang client khác.
- Đường gateway → renderer có sẵn: `Shell::ask(app, id, "ccm:orrery", payload, timeout)` (`shell/src/state.rs:56-67`) → renderer `onOrrery` (`web/src/composables/useOrreryApi.ts:46`) → reply `ccm:orrery-reply` (`ccm.rs:141`). Actions `packages/shared/src/orreryApi.ts:243-273`; source thêm `'phone'` (`orreryApi.ts:8`).
- Renderer → gateway (Workspace snapshot): cần kênh mới, vd `ccm:remote-publish` trong `ccm.rs::dispatch` (`ccm.rs:79`); registry kênh local `ccm.rs:15` phải khớp `shim.js`.

## 4. PTY
- `orrery-core` là **process con** (`shell/src/pty.rs:236-304`), init `{token, origins:[app origin]}` (:257-258), endpoint `ws://127.0.0.1:<p>/pty` + subprotocol `ccm.pty.<token>` (:297). Renderer lấy qua `ccm:pty-endpoint` (`ccm.rs:82-86`).
- Check `core-pty/src/ws.rs:16-61`: path, Origin ∈ origins, token `ct_eq`; bind `127.0.0.1:0` (`core-pty/src/server.rs:102`).
- Gateway trong vỏ gọi `Shell.pty.endpoint()` rồi mở WS với Origin app + protocol được luôn; sạch hơn: thêm origin riêng của gateway vào `init.origins`. (Nhưng hub chỉ 1 conn / session — xem `v97-renderer-side.md` §4: phải sửa hub cho nhiều viewer.)

## 5. Viewer (đều đã port lên Tauri)
- Explorer: `ccm:list-dir` (`core-desk/src/preview/ipc_files.rs:82`), `list-files` / `watch` / `unwatch` (shell `preview_ipc.rs:119-135`, event `ccm:preview-changed`), `create-entry` / `rename-entry` / `trash-entry` / `search` (`ipc_files.rs:86-117`).
- File: `ccm:read-file` :44, `file-write` :59 (`preview.writeRoots`), `log-read`, `lh-list`, `lh-read`.
- Changes: `changes-status|diff|file|discard` (`ipc_git.rs:46-75`), `changes-watch` (`preview_ipc.rs:136-152`, event `ccm:changes-updated`); git `stage|unstage|commit|commit-cancel|can-amend|blame|merge-state|merge-abort` (`ipc_git.rs:177-247`).
- Git Log: `log-refs|page|commit|contains|diff` (`ipc_git.rs:88-106`); branch `create|delete|update`, `git-fetch`, `git-push-preview|push` (`ipc_git.rs:110-153`).
- Run: 23 kênh `ccm:run-*` (`core-desk/src/run/ipc.rs`), log qua event `ccm:run-event` (`run/manager.rs:99`); `host_telegram.rs:14,220` đã gom log 200 dòng × 20 session (dùng lại).
- Event: `desk::emit_app` (`desk.rs:54`) chỉ `emit_to(main|compact)` ⇒ gateway cần sink riêng (mở rộng `emit_app` hoặc `listen_any` như `host_telegram.rs:224`).
- Gate: roots = pane sống (`preview_host.rs:227`); owner label chỉ dùng cho bookkeeping watch + `emit_to(owner)` (`preview_ipc.rs:38-54`) ⇒ `Subs` cần sink trừu tượng cho owner `remote-<device>`.
- Gọi không cần webview: `desk::call_inside(app, channel, args)` (`desk.rs:100`, args mảng JSON, **không** check `is_page_channel` ⇒ gateway phải tự whitelist kênh); `preview_ipc::dispatch(app, owner, …)` (`preview_ipc.rs:100`) nhận owner string; tránh `grant-folder` (hộp chọn native). `desk::dispatch` (`desk.rs:114`) cần Webview — không dùng.

## 6. Confirm native
- `hosts::confirm` (`shell/src/hosts.rs:100-117`) = `blocking_show` OkCancel; caller `preview_host.rs:217-221` (git branch delete `ipc_git.rs:124` …), `preview_ipc.rs:107-117` (access), `hosts.rs:130,144,155`. Chỉ `CCM_E2E_CONFIRM` trả lời được (:91-98). Plugin dialog không đóng được từ code ⇒ với lệnh từ điện thoại: **không mở hộp native**, hỏi trên điện thoại (đúng quyết định chủ nhân), cần tham số "nguồn lệnh" đi theo host call.

## 7. Telegram → pane idle
- Rust: `core-desk/src/telegram/routes.rs:306-319` → host `telegram.ask` → `host_telegram.rs:45-53` `Shell::ask("ccm:orrery")`. Renderer `useTelegramInbound.ts:161-173` `answer` quyết idle (`sessionOf(pane).status === 'idle'`). `INBOUND_MAX` 2000 (`telegram/sanitize.rs:12`). Dùng lại 100 % cho phone.

## Gap phải làm
1. Mở `StoreHandle` / subscribe từ core-serve. 2. Server LAN TLS riêng (core-http chặn non-loopback), `ws::run` generic. 3. `Subs` / `emit_to(owner)` giả định webview. 4. Source `'phone'`. 5. `hosts::confirm` chưa có đường remote. 6. Crate TLS / Noise / mDNS mới. 7. Kênh renderer → gateway publish Workspace. 8. Hub PTY nhiều viewer.
