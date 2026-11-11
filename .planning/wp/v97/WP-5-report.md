## WP-5 — DONE

### Đã đổi
- Mới `apps/shell/src/remote/`: `mod.rs` (`Remote`, kênh), `ctx.rs` (`Ctx`, trait `Host`, `Control`), `host.rs` (`AppHost`, `install`, `dispatch`, `shutdown`), `config.rs`, `keys.rs`, `devices.rs`, `status.rs`, `limits.rs`, `link.rs`, `pair.rs`, `session.rs`, `dashboard.rs`, `lan.rs`, `mdns.rs`, `relay.rs`, `text.rs`; test `test_util.rs`, `pair_tests.rs`, `session_tests.rs`, `relay_tests.rs`, `unit_tests.rs`. File dài nhất 327 dòng.
- `main.rs`: `mod remote`, `remote::host::install` trong setup, `remote::host::shutdown` đầu `before_stop` (tray Quit / update / Exit đều qua đây → phiên nhận `machine.restarting`).
- `ccm.rs`: nhánh `remote::handles` → `host::dispatch` (`spawn_blocking`); 8 kênh thêm vào `LOCAL` (test shim).
- Theo tin bổ sung của main: `apps/desktop/src/preload.ts` thêm 9 method (`remoteStatus`, `onRemoteChanged`, `remoteEnable`, `remoteKeepAwake`, `remotePairStart`, `remotePairCancel`, `remoteRevoke`, `remoteRevokeAll`, `remotePublish`); `shim.js` sinh lại (`--check` sạch).
- `Cargo.toml` / `Cargo.lock`, `.code_index/shell.md` (mục "Remote gateway", 14 dòng), `.code_index/_meta.md` (1 dòng).

### Kênh `ccm:remote-*` (shape)
- `ccm:remote-status` → `RemoteStatus` của WP-7 `{enabled, listening, addresses[ip], port, relay: off|connecting|online|error, keepAwake, devices[{deviceId, name, platform, pairedAt, lastSeen, state: connected|idle, via?: lan|relay}]}` + thêm `running, relayDetail, relayUrl, relayRegistered, machine, name, pairing{exp}|null, error: keyring|runtime|lan|null`.
- `ccm:remote-enable [bool]`, `ccm:remote-keep-awake [bool]` → status. `ccm:remote-pair-start` → `{qr, exp}` hoặc `null` (gateway tắt). `ccm:remote-pair-cancel` → `true`. `ccm:remote-revoke [deviceId]` / `ccm:remote-revoke-all` → `true|false`. `ccm:remote-publish [summary]` → `true|false` (object ≤ 1 MiB, chỉ khi gateway đã dựng ctx; cất bản mới nhất). Event `ccm:remote-changed` = status (`desk::emit_app`).

### Crate mới + lý do
- `core-remote-proto` (path), `tokio`, `futures-util`, `rustls` (ring), `webpki-roots`, `ring` (ký Ed25519, đã có trong lock), `ureq` (POST register / revoke, cùng feature với core-desk), `if-addrs` — đều đã có trong lock. Crate thật sự mới: `mdns-sd` 0.21 (+ `flume`), `tokio-rustls` (feature `rustls-tls-webpki-roots` của tokio-tungstenite cho `wss://` relay), cùng nhóm snow/curve25519 kéo theo proto. Dev-dep: `core-http`, `core-store` (StoreTap thật trong test).

### Tiêu chí: 5/5 đạt
- [x] `cargo test --locked -j 3 remote` → 24 pass / 0 fail (chạy 3 lần); toàn bộ shell 218 pass. Có: ghép đủ vòng qua WS localhost + `CCM_E2E_CONFIRM=ok` (TestHost đọc env khi `env_confirm`; `AppHost` dùng `hosts::confirm` vốn đọc env này), mã dùng lần 2 → 4401, 5 lần sai → huỷ, QR hết hạn → không ticket (unit `Pairing`, không chờ 5 phút thật), huỷ QR → 4401, IK lạ → 4401 + notification 1 lần, proto cũ → 4426, `hello` → welcome + snapshot trùng `tap.snapshot()` (= `/ws`) + `events` sống + `ws.snapshot` từ publish, lệnh cấm (`client.presence`, `terminal.open`, `focusWindow`, `folder.pick`, `favorite.open`, `notify.*`, `office.*`) → `denied`, trước hello → `invalid`, `terminal.alias` tới hook monitor, > 8 kết nối chưa xác thực bị từ chối rồi mở lại được, path khác → 404, relay giả: URL ký đúng (verify Ed25519), 2 connId → 2 phiên tách (giải mã chéo được kiểm), token relay verify đúng thiết bị, close conn → phiên đóng; register POST ký đúng; thu hồi giữa phiên → `device.revoked` + 4401; tắt gateway → `remote.off`; sweep 30 ngày → thu hồi + `pendingRevoke`.
- [x] Không bind 0.0.0.0 / loopback: `select` test + test bật `Remote` thật với địa chỉ máy (mọi địa chỉ bind là IPv4 private). Test khác bind 127.0.0.1 chỉ qua `Options.addrs` (test-only).
- [x] Key riêng không trong JSON: test đọc `config.json` / `devices.json` sau ghép, không chứa 3 secret, thư mục chỉ có 2 file này.
- [x] `node scripts/gen-desk-shim.mjs --check` sạch; clippy: phần `remote/` sạch. CHƯA KIỂM được "toàn crate sạch": `cargo clippy --locked -j 3 --all-targets -- -D warnings` còn lỗi CÓ SẴN ở file ngoài phạm vi (`host_extra.rs:102`, `preview_host.rs:86`, `preview_watch.rs:106,204`, `vault.rs:61`, `preview_serve_tests.rs:95,101,103` — dead_code / lint mới của clippy 1.97), không do WP-5.
- [x] Không comment (rg `^\s*//|/\*` trong `remote/` rỗng); `.code_index/shell.md` mục mới 14 dòng.

### Audit
- Diff: `apps/shell/**` (remote/, main.rs, ccm.rs, Cargo.*, shim.js), `apps/desktop/src/preload.ts` (main cho phép), `.code_index/shell.md` + `_meta.md`, nhật ký brief. Không đụng `apps/core|web|mobile|relay`. Không println / log; không log key, mã, token, IP.

### Review (bảo mật + rủi ro còn lại)
- Auth trước dữ liệu: không có frame ứng dụng nào trước khi Noise xong; trước `hello` mọi lệnh → `invalid`; dashboard chỉ đi qua whitelist gateway + whitelist `StoreTap` (2 lớp).
- Phân biệt ghép / phiên bằng độ dài msg1: XXpsk3 msg1 luôn `2 + 32 + 16` byte (psk làm payload rỗng có tag), IK ≥ 98. Hợp đồng chưa ghi → main nên thêm vào `docs/remote-integration.md` §2.
- Mã ghép: 1 slot, 5 phút, 1 lần (`consume` trước khi hỏi confirm, hỏi lỗi hay từ chối cũng mất mã), 5 lần sai → huỷ; chỉ có khi Settings gọi `pair-start` (mất khi cancel / tắt). Sai mã chỉ phát hiện ở msg3 (đặc tính psk3).
- Confirm native max 120 s (`hosts::confirm` không đóng được hộp; hết giờ → `pair.denied`, hộp vẫn hiện). Sau `pair.ok` gateway đóng 1000, điện thoại nối lại bằng IK (WP-10 cần biết).
- Rate limit: 8 chưa xác thực / IP, 64 tổng, 60 lần mở / phút / IP, hạn 5 s từ accept tới hết Noise, max 16 phiên, 240 lệnh / phút / phiên, WS message ≤ 65 535, `Reassembler` max 1 MiB. Có header `Origin` → 403 (chặn trang web trên LAN; app Dart `dart:io` không gửi Origin — nếu WP-10 có gửi thì phải bỏ).
- Proto: crate trả lỗi trước msg2 nên 4426 không kèm payload `{error:"proto", min}` như docs §2; reason close `old` / `new`.
- Relay: desktop không đóng được 1 phone (DO bỏ qua connId 0 từ desktop) → 4401/4426 qua relay chỉ là ngừng trả lời; thu hồi vẫn đóng 4401 nhờ `POST /revoke`. Đề xuất WP-9: control `{t:"close", conn, code}` từ desktop.
- `msg3` của crate không mang `platform` → lấy từ `hello.platform` (chỉ `[A-Za-z0-9]{1,16}`).
- Keep awake dùng `core_desk::power::native::power()` riêng (không qua `KeepAwake` của Settings / mode `plugged`), bật cả khi chạy pin.
- mDNS chưa kiểm trên mạng thật (test chỉ kiểm instance / TXT).

### Test
- `cd apps/shell && cargo test --locked -j 3 remote` → 24 pass / 0 fail (×3); `cargo test --locked -j 3` → 218 pass / 0 fail.
- `cargo clippy --locked -j 3 --all-targets -- -D warnings` → chỉ còn 8 lỗi có sẵn ngoài `remote/` (liệt kê trên).
- `node scripts/gen-desk-shim.mjs --check` → up to date; `npm run typecheck -w @ccm/desktop` → sạch.
- CHƯA KIỂM: relay thật `orrery-relay.jinvu.workers.dev` (test dùng relay giả), mDNS thật, hộp confirm native thật, chạy trong app (không được chạy app của chủ nhân).

### Cho WP-6 / WP-7 / WP-8 (điểm nối)
- Lệnh mới: thêm nhánh trong `dashboard.rs Phone::command` sau check `hello` (có `self.device`, `self.sec.json`, `self.ctx`). `ws.*` → `Shell::ask("ccm:orrery", {source:"phone", action, args:{device, cmd, confirmed?}})` cần `AppHandle`: thêm method vào trait `Host` (vd `ask(&self, payload) -> Option<Value>` gọi qua `spawn_blocking`, timeout 5 s) — `AppHost` giữ `app`. Bỏ `confirmed` do điện thoại gửi; `needsConfirm` → event `confirm.ask` (cid gateway sinh, deadline 60 s) chỉ tới phiên đó.
- Workspace summary: `Ctx::publish` / `Ctx::summary` (bản mới nhất); `hello` đã gửi `ws.snapshot`. WP-6 diff thành `ws.patch` và đẩy cho phiên sống: thêm `Control`-like channel hoặc broadcast trong `Ctx` (hiện `Live.ctl` chỉ mang `Control`).
- PTY (WP-8): `Kind::PtyIn` trong `Phone::inbound` đang bỏ qua; gửi output bằng `self.sec.frame(Kind::PtyOut, stream, bytes)`.
- Confirm phía desktop cho ghép: `Host::confirm_pair`; text vi/en ở `remote/text.rs`.
- Renderer (WP-7): dùng 9 method preload; `RemoteStatus` có thêm field (không phá type).

### Hot file cần thêm / Ngoài phạm vi
- `docs/remote-integration.md`: ghi cách phân biệt msg1 ghép / IK; 4426 không có payload lỗi; `pair.ok` rồi đóng 1000; `welcome` có `relay`, `relayToken`, `relayExp` (gia hạn mỗi `hello`).
- WP-9 / relay: control close từ desktop (xem trên).
- Clippy lỗi có sẵn ở `host_extra.rs`, `preview_host.rs`, `preview_watch.rs`, `vault.rs`, `preview_serve_tests.rs`.

### Commit message đề xuất
feat(shell): add Orrery Remote gateway (LAN listener, mDNS, QR pairing with native confirm, IK sessions, dashboard over StoreTap, relay client) and ccm:remote-* channels

## Vòng 2 — DONE (review `.planning/wp/v97/WP-5-review.md` + bổ sung WP-10)

### Đã sửa
- CAO replay msg1 IK: sau msg2 chờ frame đầu max 5 s (`session::HELLO_WAIT`), frame đó phải giải mã được và là `hello` (`Phone::first_hello`), không thì 4400. Chỉ sau đó mới `admit_live` (tính vào 16 phiên), `touch` lastSeen, notification, trả `Pending`, mở budget. Test `a_replayed_ik_msg1_never_touches_last_seen_or_takes_a_session_slot`: 7 lần phát lại không đổi lastSeen, live = 0, phone thật vẫn vào, bản phát lại bị 4400 sau 5 s.
- VỪA 1 TOCTOU: `Ctx::admit_live` kiểm `authorize` + số phiên dưới lock `devices` → `live`; `revoke` xoá + `kick` trong cùng lock `devices`. Test `a_revoke_racing_a_new_session_never_leaves_it_alive`.
- VỪA 2 keyring: trait `KeyStore` mới trong `keys.rs` (`OsKeyring` phân biệt `keyring::Error::NoEntry`); chỉ `Missing` mới sinh key, `Failed` / giá trị hỏng → không bật, `error: "keyring"` trong status, không ghi đè. `vault.rs` không đổi. Test `a_failing_keyring_never_creates_or_overwrites_keys`.
- VỪA 3 inbox trước auth: `link::Budget` (LAN reader + relay `Conns::data`) cho tối đa 4 gói / 64 KB tới khi phiên được nhận, vượt thì đóng. Lệch so với chốt "2 gói": để 4 (msg1 + `hello` + 2 lệnh điện thoại có thể gửi ngay sau `hello` như `resync` / `ws.get`); 64 KB tổng giữ đúng. Main muốn đúng 2 thì đổi `PRE_AUTH_FRAMES`. Test `the_pre_auth_budget_allows_a_few_small_frames_then_opens_up`.
- VỪA 4 relay connId: `relay::Conns` map connId → (generation, tx, budget); `Open` trùng id → bỏ phiên cũ (inbox đóng → phiên cũ kết thúc) rồi tạo phiên mới; `Outbox::Relay(conn, gen, ..)`; Msg / Close lệch generation bị bỏ. Test `relay_conn_ids_are_tracked_by_generation`.
- THẤP đã sửa: lỗi msg3 không huỷ QR, thay bằng 5 lần thử ghép / phút / IP (LAN) hoặc / connId (relay) → 4429 (test `wrong_codes_keep_the_qr_but_one_ip_gets_five_tries_a_minute`; tiêu chí cũ "sai 5 lần → huỷ" bỏ theo quyết định của main). mDNS bỏ machineId, TXT `{v=1, k, n}` với `k` = base64url 8 byte đầu SHA-256(key Noise tĩnh) (theo WP-10); lần bật đầu có relay URL mà chưa `registered` → `register` đồng bộ trước khi quảng bá. `CCM_E2E_CONFIRM`: `AppHost::confirm_pair` ở bản release mà biến có đặt → từ chối ghép; `hosts.rs` KHÔNG sửa (E2E bản đóng gói vẫn dùng biến này cho hộp khác). Tên thiết bị `pair::clean_name` (bỏ control + U+202A–202E, U+2066–2069, U+200B–200F, U+061C, U+FEFF; max 40; rỗng → `Phone`) trước confirm và lưu. Windows `SO_EXCLUSIVEADDRUSE` (socket2 + windows-sys `Win32_Networking_WinSock`). WS relay max message 1 MiB. Writer LAN / relay xả max 64 gói / lượt. `rebind.rs`: 30 s kiểm interface, bind / bỏ IP, quảng bá mDNS lại.
- Bổ sung WP-10: `hello` thành công mà `tokenExp` của thiết bị còn < 7 ngày (hoặc chưa có) → event `relay.token {token, exp, relay}` (hạn 30 ngày, lưu `tokenExp` trong `devices.json`); welcome không còn mang token, chỉ `relay` URL. `resync` trả `{re, ok:true}` (đã có); các lệnh `ws.get` / `pty.*` / `confirm.answer` và `ws.patch.rev` thuộc WP-6 / WP-8. Event `device.revoked` / `remote.off` / `machine.restarting` gửi trước khi đóng (đã có, có test cho 2 loại đầu).

### Để sau (thấp)
- QR lộ key Noise qua msg2 khi QR đang hiện; UI chưa hiện lý do QR bị huỷ.
- Hộp confirm native còn hiện sau 120 s; ghép mới có thể chồng hộp.
- Reply `history.get` / `toolSpans.get`: kênh đầy thì bỏ im lặng, chưa có deadline 10 s.
- `Gate` dọn entry cũ chỉ lúc drop.
- Payload `/ws` gửi nguyên cho điện thoại (`statusLineCommand`, cwd, command line): cần main xác nhận khớp bất biến 3.

### Ngoài phạm vi
- macOS `NSLocalNetworkUsageDescription` + `NSBonjourServices` (`_orrery._tcp`): plist bundle nằm ngoài `apps/shell/src` (main xử lý).
- `docs/remote-integration.md`: TXT mDNS `k`, event `relay.token`, welcome không mang token, msg1 ghép 50 B, 4429 khi thử ghép quá 5 lần / phút.
- WP-10: không gửi gì ngoài `hello` làm frame đầu của phiên IK; tối đa 3 frame sau msg1 trước khi nhận welcome.

### Test vòng 2
- `cargo test --locked -j 3 remote` → 31 pass / 0 fail (chạy 3 lần); `cargo test --locked -j 3` toàn shell → 225 pass / 0 fail.
- Clippy: `remote/` sạch; 8 lỗi có sẵn ngoài phạm vi giữ nguyên như vòng 1.
- `Cargo.toml` thêm `socket2` (đã có trong lock) + feature `Win32_Networking_WinSock` cho windows-sys.
