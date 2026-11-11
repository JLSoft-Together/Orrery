# PLAN v97 — Orrery Remote (app điện thoại)

Spec sản phẩm: `SPEC-v97-remote-control.md`. Quyết định: `DECISIONS.md` mục v97. Khảo sát:
`research/v97-rust-side.md`, `research/v97-renderer-side.md`, `research/v97-libs.md`.
Branch `feat/v97-mobile`, commit từng phần khi gói nghiệm thu xong, không push khi chưa được bảo.
Build: Android APK được build + cài lên máy USB để test (chủ nhân yêu cầu test Android); desktop chỉ
`cargo test` / `npm run qa`; iOS khi chủ nhân báo đang ở MacBook.

---

## 1. Kiến trúc chốt

```
Orrery Remote (Flutter, apps/mobile)
  UI (Riverpod) ── conn layer (Dart) ── orrery_remote_ffi (Rust, flutter_rust_bridge)
                                              └─ core-remote-proto (Noise, frame, pairing)
        │ WS + Noise KK                       │ WSS (CF) + Noise KK
        ▼ LAN :4320                           ▼ relay *.workers.dev  (apps/relay, Durable Object / machineId)
Orrery desktop (Tauri, apps/shell)
  remote/ gateway ── core-remote-proto
     ├─ StoreTap (core-serve, mới)        snapshot + event dashboard, lệnh ClientMessage
     ├─ PTY viewer (core-pty hub, sửa)    attach view / claim input, replay, resize theo người gõ
     ├─ Shell::ask("ccm:orrery")          lệnh Workspace → renderer (source 'phone')
     ├─ ccm:remote-publish (mới)          renderer → gateway: Workspace summary
     ├─ desk::call_inside + preview_ipc   viewer (whitelist kênh), event sink theo thiết bị
     └─ confirm router                    lệnh từ điện thoại → hỏi trên điện thoại, không mở hộp native
```

### 1.1 Quyết định kỹ thuật (con chốt, ghi lý do)

| # | Quyết định | Lý do |
|---|---|---|
| T1 | **Không TLS trên LAN**, chỉ WS + Noise | Noise đã mã hoá + xác thực 2 chiều bằng key tĩnh lấy từ QR; TLS self-signed chỉ thêm rcgen / tokio-rustls / pin cert ở Dart mà không thêm an toàn. Relay vẫn đi `wss://` của Cloudflare (bên ngoài), Noise bên trong |
| T2 | Ghép: `Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s`, PSK = HKDF(mã 1 lần **≥ 128 bit** trong QR, không có đường nhập tay); điện thoại so key tĩnh desktop (msg 2) với key trong QR **trước** khi gửi msg 3. Sau đó: `Noise_IK_25519_ChaChaPoly_BLAKE2s` (điện thoại là initiator, desktop đối chiếu key điện thoại với `devices.json`); prologue = `orrery-remote/<proto>` | XXpsk3 trao key tĩnh và chặn người không có mã, so key trước msg 3 chặn brute-force PSK offline (review #7); IK cho desktop biết điện thoại nào gọi khi có nhiều máy (review #6) |
| T3 | Crate `apps/core/crates/core-remote-proto` dùng chung: desktop link thẳng; mobile qua crate `apps/mobile/rust` (FRB) | 1 code Noise + frame cho 2 đầu (chủ nhân chốt) |
| T4 | Gateway ở `apps/shell/src/remote/`, runtime tokio multi-thread riêng (2 worker) | cần `Shell::ask`, `pty.endpoint()`, `desk::call_inside`, `hosts`; core-serve là `current_thread` |
| T5 | Ngoài Noise (chỉ đường relay): `[u16 connId][noise msg]`, DO gán `connId` + báo `open/close`. Trong Noise: `[u8 kind][u8 flags][u32 stream][payload]`; kind 0 = JSON, 1 = PTY output, 2 = PTY input, 3 = ping; flags bit 0 = `more` (khúc chưa cuối). Noise max 65 535 B ⇒ khúc 60 KiB, ghép lại tới max 1 MiB. Gộp frame (≤ 30 ms) làm ở gateway / điện thoại, không ở DO | desktop giữ 1 phiên Noise / `connId` (review #4); ghép được JSON lớn (review #13) |
| T6 | PTY: gateway là **viewer conn** của `/pty` (origin riêng thêm vào `init.origins`), hub sửa nhiều viewer + 1 input owner | desktop không bị `4000 replaced` khi điện thoại xem |
| T7 | Workspace: renderer làm chủ (cách A), publish summary debounce 200 ms qua `ccm:remote-publish`; điện thoại gửi lệnh `ORRERY_ACTIONS` source `'phone'` | dùng lại 100 % logic, không đụng IDE |
| T8 | Mobile state: Riverpod 3, snapshot + patch theo `seq`, cache offline bằng file JSON / máy (snapshot + 64 KiB tail mỗi pane) | quyết định offline = trạng thái cuối + giờ |
| T9 | Terminal mobile: `xterm` 4.0.0 nếu spike S0 đạt; không đạt → xterm.js trong `webview_flutter`; vẫn không được → native view (Compose / SwiftUI, chủ nhân cho phép) | chưa ai xác minh hiệu năng xterm.dart |
| T10 | mDNS: desktop `mdns-sd` quảng bá `_orrery._tcp`; mobile `bonsoir` 7.x; luôn có đường nhập IP / quét QR | iOS Local Network prompt, Android multicast hay lỗi |
| T11 | Relay: 1 Durable Object / `machineId`, WebSocket Hibernation. Đăng ký TOFU: lần đầu desktop gửi `machineId` + key Ed25519, DO lưu (SQLite); sau đó desktop vào `/m/<id>` bằng chữ ký challenge. Điện thoại vào `/p/<id>` bằng token `HMAC(key relay của máy, deviceId‖exp)`; desktop đẩy danh sách thu hồi lên DO. DO chỉ forward, không timer; keepalive bằng `setWebSocketAutoResponse` (ping ngoài Noise). Rate limit theo IP ở Worker trước DO; 429 / hết quota → báo rõ trên điện thoại | chặn người biết `machineId` đá desktop (review #5); giữ hibernation (review #15) |
| T12 | Push: FCM HTTP v1 từ Worker (JWT RS256 WebCrypto); iOS qua FCM + APNs key; payload chỉ `{machineId, paneKey, kind}` | không cần HTTP/2 APNs trực tiếp |

### 1.2 Bất biến (tuyệt đối không được phá)
1. Port 4317, `/ws`, `/pty` vẫn chỉ `127.0.0.1`; gateway tắt mặc định.
2. Không thiết bị nào chưa ghép vào được; ghép mới luôn xác nhận native trên máy.
3. Env, secret, app key, token PTY, token Telegram / GitHub không bao giờ ra điện thoại hay relay.
4. Relay chỉ thấy byte mã hoá, `machineId`, key công khai máy, `deviceId` + hạn token; push chỉ mang id mờ (HMAC `paneKey`) + loại sự kiện.
5. Điện thoại không có quyền nào desktop không có (cùng Gate preview, cùng limit 16 pane, cùng `since` cho phím xin quyền).
6. Lệnh từ điện thoại không đổi focus / tab desktop; confirm hỏi trên điện thoại.
7. Workspace desktop không chậm đi: summary debounce, không gửi gì khi không có điện thoại kết nối.
8. Key riêng Noise của máy + key relay nằm trong `vault.rs` (keyring OS); `devices.json` chỉ public key, tên, `lastSeen`. Điện thoại: secure storage `first_unlock_this_device`, cache không vào backup (Android `allowBackup=false`, iOS `isExcludedFromBackup`).

## 2. Giao thức lệnh (kind 0)

Envelope: `{id?, t, ...}`; reply `{re: id, ok, ...}`; event `{t, ...}` không id.

| Nhóm | Lệnh | Đi tới |
|---|---|---|
| phiên | `hello {proto, device, app, caps}` (lệch `minProto` → điện thoại báo cập nhật) + `remote.summary.get` khi vào → `welcome {machine, theme, locale, version}` + `snapshot` + `ws.snapshot` | gateway |
| dashboard | `resync`, các `ClientMessage` an toàn (alias, note, snooze, dismiss, history.get, tool-spans.get) | StoreTap |
| workspace | `ws.open {cwd, preset, prompt?, submit?, resume?}`, `ws.close {paneKey}` (bận → `confirm.ask`), `ws.rename {tabKey, name}`, `ws.move {paneKey, tabKey}`, `ws.reopen`, `ws.compact {paneKey}`, `ws.send {paneKey, text}`, `ws.keys {paneKey, keys, since}` | renderer (`Shell::ask`) |
| pty | `pty.attach {sessionId, cols, rows}`, `pty.detach`, `pty.claim {sessionId, cols, rows}`, `pty.release`, input = kind 2 | gateway → hub |
| viewer | `fs.list`, `fs.listFiles`, `fs.read`, `fs.write`, `fs.watch/unwatch`, `git.status/diff/stage/unstage/commit/discard`, `log.refs/page/commit/diff`, `branch.create/delete/fetch/update/push`, `run.configs/start/stop/reload/sessions` | `desk::call_inside` / `preview_ipc::dispatch` (whitelist) |
| confirm | `confirm.answer {id, ok}` | gateway |

Event tới app: `snapshot` / `events` (như `/ws`, đã lọc), `ws.snapshot` / `ws.patch`, `pty.ready / exit / resized / overflow`,
`fs.changed`, `git.changed`, `run.event`, `confirm.ask {id, title, detail, ok, cancel}`, `device.revoked`.

Giới hạn: `ws.*` 30 / phút, `fs.write` 20 / phút, attach max 4 / thiết bị, JSON max 1 MiB, text prompt max 2000 (= `INBOUND_MAX`).

## 3. Thay đổi theo vùng

| Vùng | File | Việc |
|---|---|---|
| core-remote-proto (mới) | `apps/core/crates/core-remote-proto/` | Noise (snow 0.10), pairing XXpsk3, KK, frame, chunk 60 KiB, key tĩnh, test vector + fuzz frame |
| core-pty | `crates/core-pty/src/hub/*`, `flow.rs`, `ws.rs` | viewers + input owner, `attach {mode}`, `claim`, `resized`, meter theo viewer, viewer chậm → 4001 riêng nó |
| core-serve | `crates/core-serve/src/lib.rs`, `core-http/src/{server,ws}.rs` | `StoreTap` (subscribe, snapshot, inject `ClientMessage` với `ClientContext` riêng) |
| shell gateway (mới) | `apps/shell/src/remote/{mod,lan,mdns,relay,pair,devices,session,pty_bridge,viewer,confirm,keepawake}.rs` | listener, mDNS, dial relay, ghép, `devices.json`, session / rate limit, cầu PTY, whitelist viewer, confirm, keep awake |
| shell nối dây | `ccm.rs`, `state.rs`, `hosts.rs`, `desk.rs` (`emit_app` sink), `preview_ipc.rs` (`Subs` sink), `main.rs` | kênh `ccm:remote-*`, confirm theo nguồn lệnh |
| renderer | `composables/useRemoteSummary.ts`, `useOrreryApi.ts`, `packages/shared/src/orreryApi.ts` (source `'phone'`, action mới), `components/settings/SettingsRemote.vue`, chip 📱 `TermPane` header, i18n `remote-v97.ts` | publish, thực thi lệnh, Settings → Điện thoại (bật / tắt, QR, list thiết bị, thu hồi, keep awake, relay status) |
| relay (mới) | `apps/relay/` (Wrangler, TS, `src/index.ts`, `src/room.ts`, vitest-pool-workers) | DO / machineId, auth token, forward, hibernation, push FCM (P7) |
| mobile (mới) | `apps/mobile/` Flutter + `apps/mobile/rust` | app Orrery Remote `dev.ccm.orrery.remote` |
| Telegram (a) | `core-desk/src/telegram/*` | thêm link `orrery-remote://m/<machine>/p/<paneKey>` vào tin chờ / xin quyền / xong |
| docs | `docs/claude-code-integration.md` §11 mới, `docs/remote-integration.md`, `.code_index/*` | ngoại lệ nội dung session ra điện thoại, protocol |

Hot file (chỉ main sửa, WP ghi "Hot file cần thêm"): `packages/shared/src/orreryApi.ts`, `apps/web/src/i18n/{en,vi}.ts`,
`apps/core/Cargo.toml`, `apps/shell/Cargo.toml`, 2 `Cargo.lock`, `apps/shell/src/main.rs`, `apps/shell/src/ccm.rs`,
`apps/shell/src/shim.js` (sinh lại), root `package.json`, `.code_index/*`.

## 4. Gói việc (WP)

| WP | Tên | Phụ thuộc | Model | Ghi chú |
|---|---|---|---|---|
| WP-0 | Hợp đồng giao thức: `docs/remote-integration.md` (frame, lệnh, event, lỗi, `proto` = 1) + type Rust (`core-remote-proto`), TS (`packages/shared/src/remote.ts`), Dart (sinh qua FRB) | — | main | trước lượt 2 (review #14) |
| WP-1 | Spike S0: `apps/mobile` khung Flutter + xterm.dart, đo replay 256 KiB, cuộn, truecolor, khung nhập tiếng Việt trên Android USB; dựng toolchain FRB (cargo-ndk, 3 target Android, cargokit), `network_security_config`, `allowBackup=false`, keystore debug | — | sonnet | nhìn thấy được, APK (review #22, #25) |
| WP-1b | Spike throttle: cửa sổ Tauri ẩn > 5 phút trên Windows, đo timer; thử WebLock pending / cờ WebView2; chốt cách (H1) | — | sonnet | trước WP-6 (review #2) |
| WP-2 | `core-remote-proto`: Noise XXpsk3 / KK, frame, chunk, test | — | opus | crypto |
| WP-3 | core-pty hub nhiều viewer + input owner + resize theo người gõ: **mọi bên gõ đều nhận**, ai gõ thành owner (claim ngầm), hub nhớ cỡ cuối từng viewer để trả khi đổi owner, viewer mất kết nối → release; reaper đổi sang "không còn viewer" | — | opus | concurrency (review #3) |
| WP-4 | `StoreTap` trong core-serve / core-http | — | sonnet | |
| WP-5 | Gateway LAN: listener, mDNS, ghép QR, `devices.json`, session, rate limit, hello / snapshot / dashboard | 2, 4 | opus | auth |
| WP-6 | Gateway: cầu PTY + Workspace (`Shell::ask`, `ccm:remote-publish`) + confirm router + keep awake | 3, 5 | sonnet | |
| WP-7 | Renderer: `useRemoteSummary` (+ trả lời `remote.summary.get`), source `'phone'`, action mới, mount Workspace ẩn khi có điện thoại (không đổi `ui.view`), đường close riêng cho phone (không `pendingClose`), `ptyClient` / `TermPane` xử lý `resized` + claim khi gõ, Settings → **Điều khiển từ xa** (tên khác trang ntfy "Điện thoại"), QR, list thiết bị, thu hồi, "Ngắt mọi điện thoại", IP + trạng thái listener, chip 📱, log lệnh từ điện thoại, notification thiết bị lạ / lần đầu trong ngày | 0 | sonnet | nhìn thấy được (review #1, #3, #11, #16, #20, #26) |
| WP-8 | Gateway viewer: whitelist kênh, sink event theo thiết bị cho `Subs` / `emit_app`, run log | 5 | sonnet | |
| WP-9 | Relay Worker + DO + gateway dial-out + tự chọn LAN / relay | 2 | sonnet | chủ nhân `wrangler login` |
| WP-10 | Mobile: FRB + conn layer (discover, QR ghép, KK, reconnect, seq resume, cache offline, nhiều máy) | 1, 2 | sonnet | |
| WP-11 | Mobile UI: Máy, project, tab + vuốt, chia pane ≥ 700 dp, terminal + khung nhập + phím phụ, Mission Control, inbox, mở pane, confirm, theme từ desktop | 10, 6 | sonnet | nhìn thấy được |
| WP-12 | Mobile viewer: Explorer, File, Changes / diff, Git Log + branch, Run, Monitor list | 11, 8 | sonnet | nhìn thấy được |
| WP-13 | Thông báo (a): link Telegram / ntfy → deep link app | 11 | sonnet | |
| WP-14 | Push (b): FCM / APNs qua relay, project `orrery-remote` | 9, 11 | sonnet | chủ nhân tạo Firebase |
| WP-15 | QA: E2E Tauri (gateway + renderer), test thật Android qua LAN + relay, `docs/qa/v97/` | cụm | sonnet | |
| WP-16 | Docs §11 + `docs/remote-integration.md` + `.code_index` | cuối | haiku | |

Lượt (≤ 3 song song, `JIN_MAX_SUBAGENTS` = 3):
1. WP-1 ‖ WP-2 ‖ WP-3 (main làm WP-0 song song)
2. WP-4 ‖ WP-1b ‖ WP-9 (relay riêng), rồi WP-10 (sau WP-0, WP-1, WP-2)
3. WP-5 → WP-6 ‖ WP-7 ‖ WP-8
4. WP-11 → WP-12 ‖ WP-13
5. WP-14, WP-15, WP-16

Cổng chờ chủ nhân: sau WP-1 (ảnh terminal trên điện thoại, chọn xterm.dart hay phương án khác); sau WP-11 (ảnh UI app trước khi làm viewer); trước WP-9 deploy (`wrangler login`); trước WP-14 (Firebase).

## 5. Test

- Rust: `cargo test --locked -j 6` cho `core-remote-proto`, `core-pty` (hub + `core-bin/tests/pty.rs`), `core-serve`, `apps/shell` (remote/*). Test Noise: vector cố định + vòng ghép đủ + KK sai key bị từ chối + replay frame bị từ chối.
- Renderer: Vitest `useRemoteSummary` (hash, debounce, không gửi khi không có thiết bị), action `'phone'`.
- Relay: `vitest` với `@cloudflare/vitest-pool-workers` (2 đầu ghép, sai token, hibernation).
- Mobile: `flutter test` (conn layer với gateway giả, reducer snapshot / patch), `flutter analyze`; FRB test Rust chạy trên desktop.
- E2E: script Tauri mới `apps/shell/e2e/v97-remote.*` dùng client Rust giả lập điện thoại (crate proto) qua LAN; Android thật qua `adb` + ảnh `docs/qa/v97/`.

## 6. Chỗ hổng đã rà (review 2026-11-12)

| # | Chỗ hổng | Sửa plan | WP |
|---|---|---|---|
| H1 | Cửa sổ ẩn xuống tray → WebView2 có thể bóp timer của trang (renderer làm chủ Workspace ⇒ summary / prefill chậm). Vỏ chưa đặt cờ nào | Khi gateway bật: thêm `--disable-background-timer-throttling --disable-renderer-backgrounding` vào `additional_browser_args` của WebView2 (Windows) / kiểm tương đương WKWebView (macOS); test: ẩn cửa sổ 10 phút rồi gửi prompt từ điện thoại | WP-6 |
| H2 | Telegram không linkify scheme lạ `orrery-remote://` | Link Telegram / ntfy = `https://<relay>/o/<machineId>/<paneKey>` → trang redirect: Android `intent://` (App Links), iOS Universal Links (`apple-app-site-association` trên Worker); máy chưa cài app → trang báo | WP-9, WP-13 |
| H3 | LAN dùng `ws://` (T1) → Android 9+ chặn cleartext, iOS ATS | Android `network_security_config` `cleartextTrafficPermitted` (base-config, chỉ app này); iOS `NSAllowsLocalNetworking` + `NSLocalNetworkUsageDescription` + `NSBonjourServices` | WP-1, WP-10 |
| H4 | Keep awake đã có | Dùng lại `core-desk/src/power/{awake,native}.rs`, không viết mới | WP-6 |
| H5 | Workspace chưa mount (desktop đang ở Monitor) ⇒ `TermPane` không spawn / không gõ (`App.vue:85-86,185`, main đã kiểm) | Có điện thoại kết nối → đặt `workspaceOpened = true` (mount ẩn), không đổi `ui.view`; E2E "desktop ở Monitor, điện thoại mở pane + gửi prompt" | WP-7, WP-15 |
| H6 | Tauri `background_throttling` không hỗ trợ Windows (`tauri-2.12.5 webview_window.rs:1261-1269`) | Spike WP-1b; dự phòng: dời timer gửi `` / debounce sang gateway (Rust) | WP-1b, WP-6 |
| H7 | Confirm gửi điện thoại không deadline; `hosts::confirm` tự `show_main` | Deadline 60 s → cancel; chỉ thiết bị gửi lệnh trả lời, id 1 lần; lệnh nguồn phone không `show_main` | WP-6 |
| H8 | `Shell::ask` chặn thread (`state.rs:56-67`) | Gọi qua `spawn_blocking`, id do gateway sinh, timeout 5 s | WP-6 |
| H9 | Pre-auth DoS trên 4320 | Max 8 kết nối chưa xác thực, handshake timeout 5 s, rate limit theo IP; ghép chỉ mở khi QR đang hiện; bind chỉ interface LAN private | WP-5 |
| H10 | App update / restart giữa phiên | Gateway gửi `machine.restarting`; điện thoại hiện "máy đang khởi động lại", nối lại + `resync` + `remote.summary.get` | WP-5, WP-10 |
| H11 | Pin / 4G | Chỉ attach pane đang nhìn, detach khi app vào nền; pane khác dùng status từ summary | WP-10, WP-11 |
| H12 | Phím phụ / gõ thẳng không đi `ws.keys` (limit 30 / phút) | Phím phụ + gõ thẳng = kind 2 (claim); `ws.keys` chỉ cho nút xin quyền có `since` | WP-11 |
| H13 | StoreTap: lệnh phone không được bật `client.presence` / `terminal.open` / `focusWindow` / `folder.pick` | Whitelist ở gateway | WP-4, WP-5 |
| H14 | Thiết bị 30 ngày không dùng tự hết hạn | Kiểm lúc `hello` + quét mỗi ngày, thu hồi cả token relay | WP-5 |
| H15 | Link: SPEC ghi `orrery://`, PLAN `orrery-remote://` | Chốt `orrery-remote://`; trang bounce `https://` (H2) + `assetlinks.json`; sửa cả ntfy (`core-desk/src/phone/`) | WP-9, WP-13 |
| H16 | Thiếu test | Thêm: nhiều phone cùng pane, cửa sổ ẩn lâu, thu hồi giữa phiên, lệch `proto`, confirm hết hạn, desktop giả key khác (dừng ở msg 2), 2 phone + 1 desktop qua relay | WP-2, WP-9, WP-15 |

Nguồn review đầy đủ: `research/v97-plan-review.md` (28 mục; #1–#28 đã gộp ở trên hoặc vào T2 / T5 / T11 / bất biến 4, 8).
