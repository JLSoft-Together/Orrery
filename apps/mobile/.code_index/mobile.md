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

## UI (WP-11) lib/ui/ + lib/l10n/
- main.dart: `Consumer` đọc `remoteThemeProvider` / `remoteLocaleProvider` (hello.theme + hello.locale của máy đang chọn `activeMachineProvider`), MaterialApp light + dark + l10n; `host == null` vẫn mở SpikePage
- l10n: `lib/l10n/app_{en,vi}.arb` sinh bằng `node tool/gen_arb.mjs` (nguồn chữ ở script) rồi `flutter gen-l10n` (l10n.yaml); `context.l10n` ở ui/widgets/common.dart
- ui/theme/orrery_theme.dart: `RemoteTheme.fromHello`, `buildOrreryTheme`, `StatusColors` (working / asked / failed / done / idle)
- ui/machine_api.dart: `MachineApi` / `PtyHandle` (cầu UI -> RemoteHost bundle; test thay bằng FakeMachineApi); ui/ui_providers.dart: `machineApiProvider`, `workspaceModelProvider`, `paneFactsProvider`, `activeMachineProvider`
- ui/model: `workspace_model.dart` (`WorkspaceModel` / `ProjectInfo` / `TabInfo` / `PaneInfo` / `LayoutNode`, pane theo `leaves(tree)`, inbox), `pane_facts.dart` (tool đang chạy + statusSince từ snapshot dashboard qua terminalId)
- ui/workspace: `workspace_page` (list project, needs nổi lên đầu), `tab_page` (PageView NeverScrollable + chấm tap / vuốt ở header + chấm; >= 700 dp `split_view` dựng cây chia, max 4 attach, ô < 260x220 thành placeholder), `mission_page`, `inbox_page`, `open_pane_page` (favorites từ snapshot + project trong Workspace), `pane_actions` (đổi tên tab / chuyển / mở lại / Compact / đóng), `machine_scope` (`machineRoute`: ConnectionGate + bottom sheet `confirm.ask` đếm 60 s), `connection_gate` (revoked xoá cache, protoTooOld)
- ui/terminal: `terminal_pane` (attach khi hiện / detach khi rời, `pty.ready` xoá terminal, `pty.exit` code null = mất terminal, đổi cỡ theo điện thoại khi đang gõ, giữ cỡ desktop + cuộn ngang khi chỉ xem), `terminal_surface` (xterm + pinch zoom), `input_bar` (khung nhập dòng + phím phụ), `permission_bar` (`ws.keys` kèm since), `pane_header`
- app/home_page.dart: danh sách máy (vuốt xoá có hỏi); app/pair_page.dart dùng l10n
- remote/store.dart: `ws.patch {base, rev}` áp khi base == rev hiện tại, lệch -> `ws.get`
- Test: workspace_model_test, ui_screens_test, terminal_pane_test (FakeMachineApi, test/support/ui_fakes.dart), screenshots_test (chỉ chạy khi `WP11_SHOTS=1`, ghi PNG vào `WP11_OUT`, font Roboto + MaterialIcons từ Flutter SDK, Consolas)

## WP-11b
- claim: `SecureChannel.seal(kind, stream, payload, claim)` (Rust, `FLAG_CLAIM` = bit 1, chỉ kind 2) -> `SecureLink.seal({claim})` -> `RemoteSession.sendPtyInput({claim})` -> `PtyStreams.input({claim})` (gộp 30 ms: claim nếu bất kỳ chunk nào claim) -> `MachineApi.input({claim})`; `terminal_pane._onOutput` claim = true trừ khi đang trong `_write` (xterm tự trả lời query -> không claim)
- protocol.dart: JSON có `re` nhưng cũng có `t` (vd `confirm.ask`) là event, không phải reply
- machine_scope.dart: sheet `confirm.ask` thử lại mỗi 150 ms khi route chưa `isCurrent` (dialog đóng chưa xong); link_widgets.dart: label LinkChip `Flexible` + ellipsis
- remote/cache.dart: `save` xếp hàng theo máy, bỏ qua `FileSystemException`; connection.dart tách `connect_attempt.dart` (`ConnectAttempt`), `lan_dialer.dart` (`openRelay` -> `RelayOpen`), `link_rules.dart` (`helloFailure`, `storeRelayToken`)
- Font terminal: `assets/fonts/CascadiaMono-Terminal.ttf` (primary `OrreryMono`, line height 2380/2048, box/block/Vietnamese in one font so rows join) + `assets/fonts/NotoSansSymbols{,2}-Terminal.ttf` (subset OFL, `LICENSES.txt`), family `OrreryTerminalSymbols` / `OrreryTerminalSymbols2` (pubspec), `ui/terminal/terminal_fonts.dart` (`orreryTerminalStyle`, đặt đầu `fontFamilyFallback`)
- rust/tests/common/commands.rs: fake gateway `Session {seq, rev, streams, confirms}`, `ws.rename` -> `ws.patch {base, rev, ops}`, `ws.close` -> `{confirm: cid}` + `confirm.ask {re}` -> `confirm.answer` (patch / `cancelled` / `notFound`), `pty.attach` reply `{stream}` (stream 0 / trùng -> `invalid`, thứ 5 -> `limit`, sessionId `vanishes` -> `pty.exit {code: null}`), gỡ bit claim rồi echo `claim+echo:`; flow.rs thêm 4 test
- Test: test/ui_integration_test.dart (tag integration: UI thật qua `fake_gateway`: mở máy -> project -> pane, gõ, echo claim, đóng pane + confirm), support/gateway.dart, support/fonts.dart (`loadFonts`), support/ui_samples.dart; ui_screens_test tách `ui_actions_test`

## WP-13 deep link
- app/pane_link.dart: `PaneLink`, `parsePaneLink(Uri)` (`orrery-remote://m/<m>/p/<p>` và `https://orrery-relay.jinvu.workers.dev/o/<m>/<p>`, kiểm ký tự); app/link_source.dart: `LinkSource`, `AppLinksSource` (package `app_links`); app/deep_link_host.dart: `DeepLinkHost` (trong `MaterialApp.builder`; máy lạ -> `linkUnknownMachine`; chọn máy -> `WorkspacePage` -> chờ summary max 20 s -> `TabPage`; không có pane -> `linkPaneGone`; bỏ link trùng trong 2 s)
- Android: 2 intent-filter (scheme `orrery-remote` host `m`; https relay `/o/` autoVerify) + `flutter_deeplinking_enabled=false`; iOS: `CFBundleURLTypes`, `FlutterDeepLinkingEnabled=false`, `Runner/Runner.entitlements` (`applinks:`) + `CODE_SIGN_ENTITLEMENTS`
- Test: pane_link_test, deep_link_test

## WP-12 viewer + Monitor
- ui/workspace/workspace_page.dart: `_Shell` NavigationBar 3 tab (Workspace · Monitor "Quỹ đạo" · Inbox); tab_page AppBar nút `viewer-menu` -> `viewers/viewer_menu.dart` (Explorer · Changes · Git Log · Run, root = pane cwd hoặc project)
- ui/machine_api.dart: `events` (stream sự kiện connection), `command(type, fields, timeout)`
- ui/viewers: `viewer_api.dart` (`ViewerApi.call` -> `ViewerReply{data,pending,cancelled}`, `ViewerFailure` từ `data.ok==false`; watch ref-count `fs`/`git` + gửi lại sau `hello`), `viewer_actions.dart` (`viewerAction`: confirm pending -> toast + reload khi `confirms` về 0), `async_panel.dart` (`AsyncPanel` load/reload theo event, `viewerErrorText`), models `viewer_models`/`log_models`/`run_models`
- Explorer (`explorer_page`, `fuzzy`): cây lazy `fs.list`, git mark từ `git.status`, tìm `fs.files` fuzzy; File (`file_page`, `code_view`, `highlight_lines` dùng package `highlight` MIT): số dòng, tìm trong file, sửa + `fs.write` kèm `base`, conflict -> banner không ghi đè; Changes (`changes_page`) stage/unstage/discard/commit; Diff (`diff_page`, `diff_view`: parse unified, gộp context > 12 dòng); Git Log (`log_page` graph 1 cột, phân trang 500, `commit_page`, `branches_page`: tạo/fetch/update/push (preview đích)/xoá); Run (`run_page`: configs/devices/start/stop/reload, log 200 dòng từ `run.event`)
- ui/monitor: `monitor_model.dart` (snapshot -> project -> session -> subagent tree), `monitor_page.dart` (context %, token chỉ khi có `totalTokens`)
- l10n: `tool/arb_viewers.mjs` (khoá `v*`) gộp vào `gen_arb.mjs`
- Test: viewers_test, ui_viewer_integration_test (tag integration, fake_gateway `tests/common/viewer.rs`), viewers_shots_test (`WP12_SHOTS=1 WP12_OUT=...`)

## WP-14 push FCM (mobile)
- push/push_config.dart: `PushConfig` (apiKey / appId / senderId / projectId, `tryParse` kiểm ký tự); push/push_gateway.dart: `PushGateway` + `FirebasePushGateway` (`Firebase.initializeApp(options)` lúc chạy, CHỈ app mặc định vì firebase_messaging chưa hỗ trợ nhiều app; `getToken` / `deleteToken` gọi qua `dynamic` để tránh cảnh báo deprecated, bản 17 chuyển sang register / onRegistered nhưng FCM v1 cần token cũ); push/push_service.dart: `PushService` (cũng là `LinkSource`): cờ bật lưu `push.enabled`, config theo máy lưu `push.configs` (SecureKv), `push.register {token, platform, locale}` khi bật / session ready / config mới / token đổi, `push.unregister` khi tắt, tap notification (`onMessageOpenedApp` + `getInitialMessage`) -> `pushLink` -> `orrery-remote://m/<m>/p/<p>`; push/remote_host_push.dart: cầu `RemoteHost` -> `PushHost` (event `push.config` từ `connection.events`); push/push_providers.dart: `pushServiceProvider` (null khi test), `PushTile` (switch / hint chưa thiết lập)
- app/link_source.dart `MergedLinkSource`; app/deep_link_host.dart thử lại 1 frame khi Navigator chưa dựng; remote_host.dart `connectionCreated`, `connections`, `forget` gửi `push.unregister`; main.dart tạo push trước `startAll`
- Android: `POST_NOTIFICATIONS`, meta channel mặc định `attention` + icon `ic_stat_orrery`, `MainActivity` tạo channel `attention`, `strings.xml` en / vi. Không có google-services
- Test: test/push_test.dart (service với fake gateway / host, link, widget tap -> TabPage, switch); fake_gateway Rust gửi `push.config` sau hello, nhận `push.register` / `push.unregister`

## WP-14 vòng 2 (mobile)
- Chỉ init 1 Firebase: `PushGateway.open(config)` khởi tạo app mặc định theo config đầu tiên, `activeProject`; máy có projectId khác = `PushService.isForeign` (không register), tile hiện `pushForeign` thay switch khi mọi máy đều foreign. `setEnabled(on, machineId:)` dùng config của project đã init, hoặc của máy được chạm khi chưa init, không bao giờ `configs.first`; bật/tắt xếp hàng tuần tự
- `onMessageOpenedApp` + `onTokenRefresh` chỉ listen sau `open()` (trong `FirebasePushGateway._init`), huỷ trong `dispose()`; lỗi Firebase/Platform -> `PushException` -> `PushService.error` -> tile `pushUnavailable`
- Không cần background handler: payload desktop có khối `notification` + `android.notification.channel_id: attention` (apps/shell/src/remote/push.rs:78-80), hệ thống tự hiện khi app nền / bị kill
- Tắt khi máy offline: `push.unregister` còn nợ lưu kv `push.unregister`, gửi khi máy đó ready lại (`_onSession`); bật lại xoá nợ
- Quyền POST_NOTIFICATIONS kiểm lúc `init` và khi app resume (`OrreryRemoteApp` -> `PushService.refreshPermission`); thu hồi -> switch tắt + `pushRevoked`
- analysis_options: `deprecated_member_use: ignore` vì `getToken`/`deleteToken`/`onTokenRefresh` của firebase_messaging 17 deprecated nhưng FCM v1 cần token cũ
- `buildPushService` log lỗi qua `dart:developer` (`orrery.push`); `RemoteHostPush.dispose`; test tách `test/push_test.dart`, `push_ui_test.dart`, `support/push_fakes.dart`

## WP-23 đổi tên
- remote/machines.dart: `Machine.alias` (lưu machines.json, `displayName`), `cleanAlias` (max 40 rune, bỏ ký tự điều khiển / ẩn, rỗng = null), `MachineRepository.rename(id, raw)`; `save` luôn giữ alias hiện có (copy cũ không ghi đè), `remove` (forget) xoá alias; `MachinesNotifier.rename`
- ui/widgets/name_dialog.dart: `showNameDialog` / `NameDialog` dùng chung (đổi tên tab, session, máy); ui/workspace/session_rename.dart: `renameSession` gửi `terminal.alias {terminalId, alias|null}` (max 80, cùng alias desktop, đã whitelist ở dashboard + StoreTap); `ui_providers.workspaceModelProvider` đè `pane.title` bằng alias từ snapshot (`sessionAliases`); `PaneFacts.alias`
- UI: home (menu ⋮ `machine-menu-<id>` + giữ lâu, dòng "Máy tính đặt tên …"), Monitor (`mon-menu-<terminalId>` + giữ lâu), pane actions (`action-rename-session`, `action-rename` = đổi tên tab `ws.rename` đã có sẵn); l10n `renameSession*`, `renameMachine*`, `machineOriginal`
- Test: test/rename_test.dart, rename_shots_test.dart (`WP23_SHOTS=1 WP23_OUT=...`)

- WP-25: `terminal_pane.dart` coalesces pty output to one `terminal.write` per frame (`_pendingOut`, `scheduleFrameCallback`, flushed before `pty.ready` / `pty.exit`); `remote/device_name.dart` `loadDeviceName` (device_info_plus, max 40) feeds `RemoteHost.deviceName`; shell `dashboard.rs` `logged` asks page `remote.logged` after a phone `terminal.alias` (web `useRemotePhone.ts` `logged`, i18n `remoteV97.log.alias`).
- WP-26: `pty_stream.dart` `_reattach` after `pty.overflow`: gateway already dropped the attach, so `pty.detach` answers `notFound`; that code is ignored and `pty.attach` is still sent (hub replays its 256K ring after `pty.ready`, pane clears then redraws); other detach errors / offline / suspended still stop, attach failure surfaces `pty.error`. Test: `test/pty_test.dart` 'overflow in the middle of 3000 lines' with `FakeServer.hub` (ring replay, `hubOverflow`).
