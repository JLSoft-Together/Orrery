## WP-6 — DONE

### Đã đổi
- Mới `apps/shell/src/remote/`: `workspace.rs` (`Board` trong `Ctx.board`: summary + broadcast `WsEvent {base, rev, ops}`, rate `ws.*`, đếm attach / thiết bị), `patch.rs` (JSON Patch RFC 6902 tự viết), `confirm.rs` (`Confirms`, `confirm.ask`, `command_of`), `phone_ws.rs` (`Extra` state của phiên + `ws.*` / `confirm.answer` / `ws.patch`), `pty_bridge.rs` (task WS client tới hub, `Slot`), `phone_pty.rs` (`pty.attach|detach|resize`, kind 2, gỡ bit claim), `awake.rs` (browser args chống throttle); test `ws_tests.rs`, `pty_tests.rs`.
- Sửa: `ctx.rs` (trait `Host` thêm `ask` → `Asked::{Reply, Offline}`, `pty` → `PtyEndpoint`; `board`; `take_pair_try(key, max)`; `status.error = lan`), `dashboard.rs` (field `pub(super)`, `x: Extra`, 3 nhánh select mới, claim, `Kind::PtyIn`, lệnh lạ → `extra`), `host.rs` (`AppHost::ask` = `Shell::ask("ccm:orrery", {id, action, args, caller:{source:"phone"}}, 6 s)`, `pty` = `PtyHost::endpoint`), `mod.rs` (module, bỏ register đồng bộ, error), `session.rs`, `pair.rs`, `lan.rs`, `test_util.rs`.
- Nối dây ngoài `remote/`: `pty.rs` (`init.origins` thêm `orrery-remote://gateway`), `window.rs` + `guest.rs` (`additional_browser_args` khi remote bật lúc khởi động — 2 file này ngoài danh sách brief nhưng bắt buộc: WebView2 cùng user data folder phải cùng args, guest thiếu args sẽ không tạo được webview).
- Web: `composables/useRemoteAwake.ts` (+ test), gọi trong `installRemoteSummary` (`useRemoteSummary.ts` +2 dòng).
- `.code_index/shell.md` (4 dòng mục Remote gateway), `web.md` + `_meta.md` (1 dòng mỗi file).

### Luồng PTY
`pty.attach {stream≥1, sessionId, cols, rows}` → kiểm `sessionId` có trong summary mới nhất (không → `notFound`), stream trùng → `invalid`, quá 4 / thiết bị → `limit` → task: `PtyHost::endpoint` → WS `ws://127.0.0.1:<p>/pty` (Origin gateway + subprotocol token) → `{t:"attach", id, mode:"view", cols, rows}` → `ready` → reply `{re, ok:true, stream}` (`gone` → `notFound`, 5 s → `timeout`, không nối được → `offline`) → `resized` đầu → event `pty.ready {stream, cols, rows}` (cỡ PTY thật) → replay + output kind 1 (gom 30 ms / max 256 KB). Sau đó `resized` → `pty.resized`; `exit` → `pty.exit {code}` rồi đóng; close 4001 → `pty.overflow`; mất hub → `pty.exit {code:null}`. Kind 2 → binary vào hub; bit claim → `{t:"claim"}` trước. `pty.resize` → `{t:"resize"}`. `pty.detach` / phiên đóng / input đầy (64 → `pty.overflow`) → WS hub đóng, slot trả.

### Luồng ws.*
`ws.get`: có summary → `{ok}` + `ws.snapshot`; chưa có → hỏi `remote.summary.get`, lưu vào Board. Mỗi `remote-publish` → diff với bản trước → broadcast; phiên chỉ nhận `ws.patch {base, rev, ops}` khi `base` = rev nó đang có, không thì `ws.snapshot` (patch to hơn snapshot / lagged cũng snapshot). `ws.open|close|rename|move|reopen|compact|send|keys` → `Host::ask` trong `spawn_blocking`, gateway cắt 5 s → `timeout`; không có cửa sổ main → `offline`; 30 `ws.*` / phút / thiết bị → `rate`; max 4 lệnh chờ / phiên → `busy`. Args `{device: deviceId, cmd}` (cmd bỏ `id`, `confirmed`). Lỗi renderer map: `args`→`invalid`, `unavailable`→`offline`, `source|unknown`→`denied`, mã khác ngoài danh sách → `internal`.

### Confirm
`{needsConfirm}` → reply lệnh gốc `{re, ok:true, confirm: cid}` + event `confirm.ask {cid, re, title, detail, ok, cancel}` (chỉ phiên đó, text max 400 ký tự). `confirm.answer {cid, ok:true}` trong 60 s → gọi lại với `confirmed:true` (chỉ gateway đặt), reply của answer = kết quả thật; `ok:false` → `{ok:true, cancelled:true}`; cid dùng rồi / quá 60 s / lạ → `notFound`; gọi lại vẫn `needsConfirm` → `busy`. Max 4 cid / phiên (bỏ cũ nhất). Phiên đóng → mất hết.

### Awake / throttle
- Keep awake: WP-5 đã có (`Host::awake` → `core_desk::power::native::power()`, bật khi enable + `keepAwake`, tắt khi tắt remote / `shutdown` của app); test `unit_tests` có sẵn, không đổi.
- Throttle: remote bật lúc khởi động → mọi webview (main, compact, guest) dùng args mặc định wry + `--disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows` (chốt 1 lần / process, bật remote giữa chừng cần khởi động lại app). Renderer giữ Web Lock `orrery-remote-awake` khi `remote.connected > 0`. `\r` của `ws.send` vẫn do renderer.

### 5 lỗi thấp WP-5
- `register` không còn chạy đồng bộ trong `start` (`relay::run` đăng ký nền; TXT mDNS không chứa machineId nên quảng bá sớm không lộ gì) → bật / thoát không chờ HTTP.
- non-Windows `set_reuse_address(true)` (Windows giữ `SO_EXCLUSIVEADDRUSE`).
- `is_hidden` = toàn bộ `Cf` (Unicode 15, liệt kê) + U+2028 / U+2029.
- Thử ghép qua relay: 1 khoá chung `relay`, 20 / phút (envelope không mang IP).
- `error: lan` tính trong `Ctx::status` (running mà không còn địa chỉ) → rebind / `ctx.changed()` cập nhật đúng; `Remote.error` chỉ còn `keyring` / `runtime`.

### Tiêu chí: 5/5 đạt
- [x] `cargo test --locked -j 3 remote` 42 pass (×3) + toàn shell 236 pass. Test mới: hub thật (`apps/core/target/debug/orrery-core`, build 02:03 sau WP-3 vòng 3): desktop spawn shell, điện thoại attach nhận `pty.ready` + replay + output, kind 2 + claim → desktop nhận `resized 50x20` + output của phone, desktop claim lại → phone nhận `pty.resized 80`, socket desktop không bị đóng suốt test, attach id ngoài summary → `notFound`, attach thứ 5 → `limit`, detach trả slot, phiên rớt → mọi slot trả; `ws.*` timeout (đo 5–7 s), `offline`, `denied`; confirm 1 lần, `confirmed` của phone bị bỏ (args lần đầu không có), lần 2 có `confirmed:true`; unit quá 60 s → huỷ; JSON Patch 7 ca (snapshot + patch == mới) + patch qua WS áp đúng; rate 30 / phút.
- [x] Vitest `useRemoteAwake.test.ts` (lock chỉ giữ khi có thiết bị, nhả khi về 0, không có API thì bỏ qua) + `useRemoteSummary.test.ts` → 6 pass; `npm run typecheck -w @ccm/web` sạch.
- [x] 5 lỗi thấp đã sửa; test `enabling_with_an_unreachable_relay_never_waits_for_the_register_call` (relay im lặng: enable < 2 s, tắt < 3 s), `hidden_name_characters_include_line_separators_and_every_format_char`, `no_lan_address_shows_the_lan_error_and_it_clears_when_off`.
- [x] `cargo clippy --locked -j 3 --all-targets -- -D warnings`: `remote/`, `pty.rs`, `window.rs`, `guest.rs` sạch; còn đúng 8 lỗi có sẵn ngoài phạm vi (`host_extra.rs:102`, `preview_host.rs:86`, `preview_watch.rs:106,204`, `vault.rs:61`, `preview_serve_tests.rs:95,101,103`). `node scripts/gen-desk-shim.mjs --check` → up to date.
- [x] Không comment (rg rỗng); file dài nhất `pty.rs` 382, `guest.rs` 386, `dashboard.rs` 344; `.code_index/shell.md` cập nhật.

### Audit
Diff: `apps/shell/src/remote/**`, `pty.rs` (1 dòng), `window.rs` (+3), `guest.rs` (+6), `apps/web/src/composables/useRemoteAwake*.ts` + `useRemoteSummary.ts` (+2), `.code_index/{shell,web,_meta}.md`, nhật ký brief. Không đụng `apps/core|mobile|relay`, `ccm.rs`, `main.rs`, `state.rs`, `hosts.rs` (không cần: needsConfirm không hiện gì trên desktop). Không log key / token / payload / byte terminal.

### Review — rủi ro còn lại
- Crate `core-remote-proto` `Frame::decode` từ chối flag ≠ `more` → gateway tự gỡ bit claim trước khi decode; nhưng app (FRB tới cùng crate) KHÔNG encode được bit claim bằng `Frame::encode` → WP-11 phải tự đặt bit 1 vào byte flags của plaintext kind 2, hoặc main mở WP nhỏ cho core thêm `FLAG_CLAIM` (`apps/core/**` cấm với WP-6).
- Phone đang là owner (đã claim) mà đọc chậm vẫn pace PTY (theo WP-3); viewer `view` chưa claim chậm → hub 4001 → `pty.overflow`.
- Phiên rớt giữa lúc lệnh `ws.*` đang chờ: renderer vẫn làm lệnh (vd `ws.open` vẫn mở pane), chỉ mất reply. Rớt giữa confirm → cid mất, lệnh không chạy.
- Renderer reload: lệnh trong lúc reload → `timeout` (5 s) hoặc `offline`; Board giữ summary cũ tới lần publish sau (rev mới = giây epoch → phiên nhận snapshot).
- 2 điện thoại cùng pane: mỗi attach 1 viewer riêng, ai claim gần nhất là owner; giới hạn 4 tính theo thiết bị (gộp mọi phiên của thiết bị).
- Pane đóng khi đang attach: `exit` → `pty.exit` → gateway tự đóng attach + trả slot; app nên coi stream đó đã hết.
- App thoát: `before_stop` → `Remote::shutdown` (event `machine.restarting`, runtime tắt → mọi WS hub đóng) trước `PtyHost::stop`; keep awake tắt.
- Throttle args chỉ có tác dụng trên Windows (macOS: Tauri bỏ qua; App Nap chưa xử lý) và chỉ khi remote đã bật lúc mở app; bật remote giữa phiên thì cần khởi động lại (chưa có chữ báo trên Settings — WP sau).
- `ws.get` tính vào 30 / phút; app không cần gọi khi lệch rev vì gateway tự gửi snapshot.

### Test
- `cd apps/shell && cargo test --locked -j 3 remote` → 42 pass / 0 fail (×3); `cargo test --locked -j 3` → 236 pass / 0 fail.
- `cargo clippy --locked -j 3 --all-targets -- -D warnings` → chỉ 8 lỗi có sẵn ngoài phạm vi.
- `node scripts/gen-desk-shim.mjs --check` → up to date.
- `npm run typecheck -w @ccm/web` → sạch; `npm test -w @ccm/web -- --maxWorkers=2 src/composables/useRemoteAwake.test.ts src/composables/useRemoteSummary.test.ts` → 6 pass / 0 fail.
- CHƯA KIỂM: macOS (`SO_REUSEADDR`, browser args no-op) — không có máy / target mac; hiệu quả chống throttle thật khi cửa sổ ẩn xuống tray (cần chạy app, không được chạy app của chủ nhân); đường relay cho PTY (chỉ test LAN; cùng code `Phone`).

### Cho WP-8 (viewer) và WP-11 (app)
- WP-8: thêm lệnh `fs.*` / `git.*` / `log.*` / `run.*` vào `Phone::extra` (`phone_ws.rs`) — có sẵn đường `call(id, action, cmd, confirmed)` → `Host::ask` + `Done` + confirm; nếu đi `desk::call_inside` thay vì renderer thì thêm method vào trait `Host` và dùng lại `Done`. Rate riêng `fs.write` 20 / phút chưa có.
- WP-11 phải làm: (1) `pty.attach` chọn `stream` ≥ 1 duy nhất trong phiên, chờ reply `{ok, stream}` rồi `pty.ready {cols, rows}` (cỡ PTY thật, hiển thị theo cỡ này tới khi claim); (2) kind 2 với bit claim (0b10 ở byte flags) chỉ cho phím người dùng gõ / dán — tự đặt bit vì `Frame::encode` không hỗ trợ; câu trả lời query của terminal không claim; (3) `pty.resize` khi cỡ màn đổi (chỉ áp khi đang là owner); (4) `pty.exit` / `pty.overflow` → stream hết, overflow thì attach lại (stream mới hoặc cùng stream sau khi nhận event); (5) `ws.patch` có thêm `base`: chỉ áp khi `base` == rev đang có, không thì chờ `ws.snapshot` (gateway tự gửi); (6) confirm: lệnh gốc trả `{ok:true, confirm: cid}`, sau đó event `confirm.ask {cid, re, …}`; trả lời bằng `confirm.answer {id, cid, ok}`, kết quả thật nằm ở reply của answer; quá 60 s → `notFound`; (7) không gửi `confirmed` (bị bỏ); (8) max 4 attach / thiết bị, 30 `ws.*` / phút.
- Main / docs (`docs/remote-integration.md` ngoài phạm vi): ghi `ws.patch.base`, reply `{confirm: cid}` + `confirm.ask.re` + `{cancelled}`, reply attach `{stream}`, `pty.exit {code:null}` khi mất hub, `pty.overflow` khi hàng input đầy, stream 0 không hợp lệ, `limit` khi attach thứ 5, thử ghép qua relay 20 / phút chung.

### Commit message đề xuất
feat(shell): remote gateway PTY bridge (view attach to the hub with claim), ws.* commands via the renderer with phone-side confirm, ws.patch diffs, anti-throttle browser args and Web Lock; fix WP-5 low findings

## Vòng 2 — DONE (review `.planning/wp/v97/WP-6-review.md`)

### Đã sửa
- (vừa) Phone owner chậm / chết im: `Pump::put` gửi sang phiên với deadline 2 s (`SEND_WAIT`), backlog output max 4 MiB / phiên (`Extra.backlog`, trừ khi phiên đã gửi / bỏ); quá → bridge đóng WS hub ngay (hub `leave` → owner về desktop) và báo `Ended(stalled)` → phiên gửi `pty.overflow` (Ended chờ không deadline, task không còn giữ gì). LAN `pump`: 45 s không nhận frame nào → đóng (ghi vào docs §1: phone ping ≤ 30 s). Test `a_claimed_phone_that_stops_reading_never_stalls_the_desktop_pane` (hub thật: bridge claim rồi ngừng đọc, desktop flood 150 000 dòng: khoảng lặng dài nhất ≤ 3 s, có `stalled`, slot trả).
- (vừa) `ASK_WAIT` 4 s < `WS_TIMEOUT` 5 s. Gateway gửi `rid` = nonce phiên + hex `id` lệnh (`c.` cho lần gọi sau confirm); renderer `useRemotePhone.onceByRid` giữ 50 rid / 60 s, trùng → trả lại kết quả lần đầu, không chạy lại. Thêm `rid?` vào `RemotePhoneCall` + `parseRemotePhoneCall` và hằng `REMOTE_RID_*` trong `packages/shared/src/remote.ts` (cần để rid qua được parser; ngoài danh sách file nhưng tối thiểu). Docs §4.3: `timeout` = kết quả chưa rõ, client không tự retry.
- (thấp) `FromHub::Json/Out/Ended` mang `gen`, dữ liệu của bridge cũ bị bỏ; bridge kiểm `rx.is_closed()` trước reply OK; `Slot` chuyển vào task bridge, trả sau khi WS hub đóng (test chờ slot về sau detach).

### Để sau (thấp)
- Browser args chốt bằng `OnceLock` theo process: bật / tắt remote cần khởi động lại app (Settings chưa báo).
- Cùng `webview_dir` mà args khác giữa 2 lần chạy: nếu `msedgewebview2` cũ chưa thoát thì `create_main` có thể lỗi — chưa có thử lại không args.
- `has_session` dựa trên summary cuối; lúc phone vừa nối lại summary có thể cũ (hub vẫn trả `gone` → `notFound`).
- macOS chỉ còn Web Lock (args WebView2 không có tác dụng, App Nap chưa xử lý).
- Backlog tính theo phiên, không gộp nhiều phiên của cùng thiết bị.

### Test vòng 2
- `cargo test --locked -j 3 remote` → 43 pass (×2); toàn shell → 237 pass.
- clippy `--all-targets -D warnings`: chỉ 8 lỗi có sẵn ngoài phạm vi, `remote/` sạch.
- `npm run typecheck -w @ccm/web` + `-w @ccm/shared` sạch; Vitest `useRemoteAwake`, `useRemoteSummary`, `useRemotePhone` (+2 test rid), `lib/remoteParse` → 25 pass.

### File đổi thêm ở vòng 2
`remote/pty_bridge.rs`, `phone_pty.rs`, `phone_ws.rs`, `host.rs`, `lan.rs`, `pty_tests.rs`, `ws_tests.rs`; `apps/web/src/composables/useRemotePhone.ts` (+ test); `packages/shared/src/remote.ts`; `docs/remote-integration.md` (2 đoạn); `.code_index/shell.md`.
