# apps/mobile index (Orrery Remote, Flutter)

Hợp đồng: `docs/remote-integration.md`. Crypto + frame nằm ở crate `apps/core/crates/core-remote-proto`, Dart gọi qua FRB 2.13 (`rust/src/api/remote.rs`).

## Rust (rust/)
- Cargo.toml: path dep `core-remote-proto`; crate-type cdylib + staticlib + rlib; dev-dep `tungstenite` + `serde_json` cho test / example
- src/api/remote.rs (FRB, không IO): `decode_qr`, `generate_static_key`, `static_public_key`, `pair_start` -> `PairingStart{handshake,msg1}`, `PairingHandshake.finish(msg2)` -> `PairingDone{machine_id,desktop_key,msg3,channel}`, `session_start` -> `SessionStart`, `SessionHandshake.finish(msg2)` -> `SecureChannel`; `SecureChannel.seal(kind,stream,payload)` -> các ciphertext đã chia khúc 60 KiB, `.open(ciphertext)` -> `Option<InboundMessage>` (ghép khúc trong Rust), `.is_closed()`; lỗi `RemoteError{kind: RemoteErrorKind, value}` (enum có field cần freezed nên dùng struct)
- src/api/simple.rs: `proto_version`, `echo` (spike WP-1)
- tests/common/mod.rs: gateway giả (tungstenite, dùng crate proto làm desktop: ghép, IK, `hello`/`snapshot`/`ws.snapshot`, `pty.attach` + echo, đóng 4401 / 4400)
- tests/flow.rs (8 test ghép / IK / chunk / key mismatch / hết hạn / 4401 / mã dùng 1 lần), tests/vectors.rs (QR + key theo `tests/vectors.json` của crate)
- examples/fake_gateway.rs: `cargo run --example fake_gateway -- [port] [exp_giây]`, in 1 dòng JSON `{qr,port,machine}` rồi chờ; dùng bởi test/integration_test.dart

## Dart lib/remote/
- crypto.dart: giao diện `RemoteCrypto` / `SecureLink` / `PairHandshake` / `SessionHandshake`, `FrameKind`, `CryptoException`; crypto_rust.dart: bản FRB (`RustRemoteCrypto`)
- wire.dart: `WireSocket`, `Inbox` (StreamIterator dùng chung handshake + session), `IoWireSocket` (WebSocketChannel, close code qua `closeCode`)
- protocol.dart: `RemoteSession` (frame json / pty / ping, `request(t, fields)` timeout 10 s theo `id`, events + ptyOutput broadcast, keepalive: text `"ping"` relay, kind 3 LAN), `CloseCode` 44xx, `RemoteCommandException(code)`
- connection_config.dart: `ConnectionConfig`, `ConnectionDeps`, `NetworkWatcher`, `relayPhoneUrl` (connection.dart re-export); lan_dialer.dart: `openLan` (lastHost song song mDNS, `Opened`); session_handshake.dart: `runSessionHandshake` (IK msg1/msg2, timeout 5 s); link_rules.dart: `classifyClose` (mã đóng 44xx -> `LinkState`), `rememberRoute` (lưu host / port / token / tên sau hello); monitor_reducer.dart: `applyMonitorEvent`
- connection.dart: `MachineConnection` state machine (`LinkState` trong link_state.dart: idle / discovering / handshaking / ready / reconnecting / offline / protoTooOld / revoked), LAN (lastHost song song mDNS, 1,5 s) rồi relay, backoff 1-2-4-8-16-30 s, đổi mạng nối lại ngay, nền 30 s đóng WS, `goForeground` = resync + ws.get, keepalive 30 s / chết sau 75 s, 4401 -> revoked (LAN thì ghi `revoked` vào máy), 4426 -> protoTooOld, 4503 -> desktopAbsent, 4429 -> chờ max, token relay hết hạn -> offline `tokenExpired`; `ConnectionDeps`, `ConnectionConfig`, `relayPhoneUrl`
- pairing.dart: `Pairing.pair(qrText)` chỉ LAN (host trong QR hoặc mDNS), so `mkey` trước msg3, chờ `pair.ok` / `pair.denied` tối đa 90 s, lưu `Machine`; `PairFailure`
- machines.dart: `Machine`, `RelayToken` (exp đọc từ 40 byte cuối token), `MachineRepository` (machines.json không chứa token; token ở secure storage; `directory` null = chỉ RAM cho test)
- keys.dart: `KeyStore` (key tĩnh + deviceId trong `SecureKv`); secure_kv.dart: `FlutterSecureKv` (`first_unlock_this_device`, không sync iCloud), `MemorySecureKv`
- discovery.dart: `LanDiscovery`, `BonsoirLanDiscovery` (`_orrery._tcp`, TXT key `machine` | `machineId` | `id`, host IPv4); network_watcher.dart: connectivity_plus -> chữ ký mạng
- store.dart: `MachineStore` + `MachineView` (snapshot theo `seq`, hụt seq -> 1 lần `resync`; `ws.snapshot` / `ws.patch` theo `rev`, lệch hoặc patch lỗi -> `ws.get`; `confirm.ask` sống 60 s; stale khi mất link; cache debounce 2 s); `applyMonitorEvent` áp `MonitorEvent` lên snapshot JSON
- json_patch.dart: RFC 6902 add / replace / remove; cache.dart: `MachineCache` file JSON mỗi máy (snapshot + workspace + tail 64 KiB mỗi pane, base64) trong thư mục cache của app (iOS không backup; Android đã `allowBackup=false`)
- pty_stream.dart: `PtyStreams` (max 4 attach, stream id tăng dần, input gộp 30 ms kiểu leading-edge, resize, suspend / resume khi nền, attach lại sau reconnect và sau `pty.overflow`, `pty.ready` xoá tail; UI phải xoá terminal khi nhận `pty.ready` vì replay gửi lại từ đầu)
- remote_host.dart: `RemoteHost` (bundle connection + store + pty mỗi máy, `pairWithQr`, `forget`, `setForeground`); providers.dart: `remoteHostProvider` (override ở main), `machinesProvider`, `linkProvider(id)`, `machineViewProvider(id)` (Riverpod 3)

## Dart lib/app/ + khác
- main.dart: `RustLib.init`, `buildRemoteHost()`, ProviderScope, vòng đời app -> `setForeground`; `OrreryRemoteApp(host: null)` vẫn mở SpikePage (test cũ)
- app/home_page.dart (danh sách máy + trạng thái, màn tạm), app/pair_page.dart (mobile_scanner, thông báo lỗi ghép), app/bootstrap.dart (thư mục app support / cache, deps thật)
- src/spike/*: màn spike terminal WP-1 (xterm 4.0.0), src/rust/*: sinh bởi `flutter_rust_bridge_codegen generate` (không sửa tay)

## Test
- test/support/fakes.dart: FakeCrypto (frame không mã hoá), FakeServer / FakePairServer / FakeDiscovery / FakeNetwork / FakeNet
- connection_test (23, fake_async), store_test + json_patch_test (18), pairing_test (12), pty_test (8), integration_test (tag `integration`: build `cargo build --examples`, nạp `rust/target/debug/rust_lib_orrery_remote.dll`, chạy `fake_gateway`, ghép + IK + PTY qua WS thật), widget_test (spike)
- Lưu ý fake_async: không `await` `StreamSubscription.cancel()` của broadcast stream (future gốc ở root zone, không chạy trong fakeAsync)

## Android
- network_security_config (cleartext), data_extraction_rules, allowBackup=false, minSdk 24, org.gradle.workers.max=4; rust_builder/ cargokit build 3 ABI
