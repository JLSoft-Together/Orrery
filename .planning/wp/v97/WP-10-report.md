## WP-10 — DONE (iOS và quét QR bằng camera thật: CHƯA KIỂM)

- Đã đổi (chỉ `apps/mobile/**`, `.planning/wp/v97/WP-10*.md`, `docs/qa/v97/WP-10/`):
  - Rust `apps/mobile/rust`: path dep `core-remote-proto`, `src/api/remote.rs` (FRB, không IO), `tests/{flow,vectors}.rs`, `tests/common/mod.rs` (gateway giả dùng crate proto làm desktop), `examples/fake_gateway.rs`.
  - Dart `lib/remote/*` (crypto, crypto_rust, wire, protocol, connection, connection_config, lan_dialer, session_handshake, link_rules, link_state, pairing, machines, keys, secure_kv, discovery, network_watcher, store, monitor_reducer, json_patch, cache, pty_stream, remote_host, providers), `lib/app/{bootstrap,home_page,pair_page}.dart`, `lib/main.dart`; spike WP-1 giữ nguyên (`lib/src/spike`, mở bằng nút trên màn chính; test widget cũ vẫn pass).
  - pubspec: flutter_riverpod 3.5, web_socket_channel, flutter_secure_storage 11.2, bonsoir 7.1.6, mobile_scanner 7.5, connectivity_plus, path_provider, clock; dev: fake_async. `dart_test.yaml` khai tag `integration`.
  - `.code_index/mobile.md` viết lại.
- API FRB (`lib/src/rust/api/remote.dart`, sinh bằng codegen): `decodeQr`, `generateStaticKey`, `staticPublicKey`, `pairStart` -> `PairingStart{handshake,msg1}`, `PairingHandshake.finish(msg2)` -> `PairingDone{machineId,desktopKey,msg3,channel}`, `sessionStart` -> `SessionStart`, `SessionHandshake.finish(msg2)` -> `SecureChannel`; `SecureChannel.seal(kind,stream,payload)` (chia khúc 60 KiB + mã hoá, trả list ciphertext), `.open(ciphertext)` (giải mã + ghép khúc, trả `InboundMessage?`), `.isClosed()`. Lỗi: struct `RemoteError{kind: RemoteErrorKind, value}` (enum có field đòi `freezed`, tránh thêm dependency).
- API Dart chính: `RemoteCrypto` (giao diện, có bản Rust + bản fake), `MachineConnection` (state `LinkState`: idle / discovering / handshaking / ready / reconnecting / offline / protoTooOld / revoked; `connect`, `goBackground`, `goForeground`, `refresh`, streams `states` / `events` / `ptyOutput` / `readySessions`), `RemoteSession.request(t, fields)`, `Pairing.pair(qrText)`, `MachineStore` + `MachineView`, `PtyStreams.attach / detach / input / resize / suspendAll / resumeAll`, `RemoteHost`, providers Riverpod 3 (`remoteHostProvider`, `machinesProvider`, `linkProvider(id)`, `machineViewProvider(id)`).
- Tiêu chí: 6/6 đạt (xem Test; iOS là CHƯA KIỂM ngoài tiêu chí).
  - `flutter analyze` sạch; `flutter test --concurrency=4` pass (65): đủ state machine, chọn LAN / relay, backoff 1-2-4-8-16-30-30, seq / resync, JSON Patch + lệch rev, cache đọc lại, 4401 -> revoked, 4426 -> protoTooOld, 4503, 4429, token hết hạn, đổi mạng, nền / foreground, keepalive, 2 máy cùng lúc.
  - `cargo test -j 3` trong `apps/mobile/rust`: 10 pass.
  - Ghép + IK với gateway giả qua WS thật localhost: `test/integration_test.dart` (nạp `rust_lib_orrery_remote.dll` thật qua FRB, chạy `fake_gateway.exe`): ghép, hello + snapshot + ws.snapshot, PTY replay + echo, resync, lệnh lỗi `notFound`, đóng kết nối -> nối lại -> attach lại -> replay; thiết bị chưa ghép nhận đóng thật `4401` -> `revoked`; mã QR dùng lần 2 bị từ chối, QR hết hạn bị từ chối.
  - Key tĩnh + deviceId + relay token chỉ trong `flutter_secure_storage` (iOS `first_unlock_this_device`, `synchronizable: false`); `machines.json` không có token; cache nằm thư mục cache của app (iOS Caches không vào backup, Android `allowBackup=false`).
  - Không comment trong code do con viết (rg `//` / `#` = 0; file FRB sinh giữ nguyên).
  - code_index cập nhật.
- Audit: diff chỉ trong `apps/mobile/**` + docs/qa + file WP; không đụng `apps/core`, `apps/shell`, `apps/relay`, `apps/web`. Rust crate proto chỉ dùng qua path dep.
- Review (rủi ro còn lại):
  - Relay đóng `4401` cho cả token sai / hết hạn / bị thu hồi: điện thoại coi là `revoked` (dừng thử lại; chỉ ghi `revoked` vào máy khi 4401 đến từ LAN). Nếu token relay hết hạn mà LAN không tới được, app báo `offline / tokenExpired` và chờ LAN để gia hạn.
  - Ghép chỉ qua LAN (host trong QR hoặc mDNS): relay cần token mà token chỉ có sau khi ghép.
  - Gia hạn token relay: app nhận token mới từ trường `relayToken` trong reply `hello` hoặc event `relay.token {token}` (chưa có trong hợp đồng, xem mục dưới).
  - Pane `pty.ready` gửi lại replay từ đầu: UI (WP-11) phải xoá terminal khi nhận `pty.ready` (store đã xoá tail của pane).
  - iOS: chưa chạy được (không có Mac); `NSBonjourServices` / Local Network prompt chưa kiểm. mDNS thật (`bonsoir`) chưa kiểm vì không có gateway thật.
  - Màn tạm chỉ đủ ghép + xem trạng thái; không có nhập mã tay (đúng brief).
  - `connection.dart` 461 dòng, `store.dart` 347 (đã tách bớt, hơi vượt "~400" ở connection).
- Test:
  - `cd apps/mobile && flutter analyze && flutter test --concurrency=4` -> No issues; 65 pass / 0 fail (connection 23, store + json_patch 18, pairing 12, pty 8, integration 3, widget 1).
  - `cd apps/mobile/rust && cargo test -j 3` -> 10 pass / 0 fail (flow 8, vectors 2). `tests/vectors.json` chỉ dùng được cho QR + khoá tĩnh: API FRB luôn sinh ephemeral ngẫu nhiên nên không tái hiện msg1-3 cố định; luồng đầy đủ kiểm bằng responder thật của crate.
  - Máy thật Galaxy A21s (`192.168.1.217:43169`, máy mở khoá): `flutter build apk --debug --target-platform android-arm64` OK (264 s, cargokit build cả 3 ABI), `adb install -r` Success, app mở không crash (logcat không có FATAL / Unhandled), màn chính "Chưa có máy nào" + nút "Ghép máy": `docs/qa/v97/WP-10/01-home.png`. CHƯA KIỂM trên máy thật: quét QR bằng camera, ghép với desktop thật, mDNS (không có gateway thật trong LAN).
- Ngoài phạm vi cần main xử lý (gateway WP-5 / relay WP-9):
  1. `pair.ok`: `relayToken` là chuỗi token (hoặc `{token}`); app đọc `exp` từ 8 byte trước 32 byte MAC của token. Cần thêm vào `docs/remote-integration.md`.
  2. Cách gia hạn token relay ("gia hạn khi dùng"): hợp đồng chưa nói; app hỗ trợ `hello` reply có `relayToken` và event `relay.token {token}`.
  3. mDNS TXT: app đọc machineId từ key `machine` (cũng nhận `machineId` / `id`), IPv4 từ `hostAddresses`; chốt tên key trong hợp đồng.
  4. `resync`, `ws.get`, `pty.detach`, `pty.resize`, `confirm.answer` phải trả `{re, ok:true}` (app dùng `request` có timeout 10 s) ngoài event.
  5. `ws.patch.rev` phải đúng `rev hiện tại + 1`, không thì app tự gọi `ws.get`; `ws.snapshot` gồm `rev`, `tabs`, `closedCount`.
  6. Hợp đồng không có cờ `claim` cho kind 2: app chỉ gửi kind 2 khi người dùng gõ, gateway tự coi là claim.
  7. Relay nên phân biệt token hết hạn với thiết bị bị thu hồi (mã khác 4401) để app biết đường gia hạn thay vì dừng.
  8. Gateway nên gửi `device.revoked` / `remote.off` / `machine.restarting` trước khi đóng socket (app dùng để hiện lý do).
- Commit message đề xuất:
  `feat(mobile): add Orrery Remote connection layer (FRB bindings to core-remote-proto, LAN/relay state machine, pairing, store, cache, PTY streams) with fake gateway tests`
