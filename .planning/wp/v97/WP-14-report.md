## WP-14 — PARTIAL (mobile done; relay reverted; shell kept as-is per main)

### Đã đổi
- Mobile `apps/mobile`:
  - `lib/push/push_config.dart` (PushConfig + kiểm ký tự), `push_gateway.dart` (FirebasePushGateway: `Firebase.initializeApp(options)` lúc chạy, không google-services), `push_service.dart` (bật/tắt, `push.register`/`push.unregister`, config theo máy lưu SecureKv `push.configs`, tap -> `orrery-remote://m/<m>/p/<p>`, `getInitialMessage`), `remote_host_push.dart` (cầu RemoteHost, event `push.config`), `push_providers.dart` (`PushTile`: switch hoặc hint "chưa thiết lập trên máy tính").
  - `lib/app/link_source.dart` (MergedLinkSource), `deep_link_host.dart` (thử lại 1 frame khi Navigator chưa dựng), `home_page.dart` (PushTile), `bootstrap.dart`, `main.dart` (push tạo trước `startAll`), `lib/remote/remote_host.dart` (`connectionCreated`, `connections`, `forget` gửi `push.unregister`).
  - Android: `AndroidManifest.xml` (POST_NOTIFICATIONS, channel mặc định `attention`, icon), `MainActivity.kt` (tạo channel `attention`), `res/values{,-vi}/strings.xml`, `res/drawable/ic_stat_orrery.xml`. Gradle: KHÔNG plugin google-services (đã gỡ phần tôi thêm lúc đầu).
  - `pubspec.yaml`/`pubspec.lock`: `firebase_core` 5.1.0, `firebase_messaging` 17.0.1. ARB vi/en (`tool/gen_arb.mjs` + gen-l10n): pushTitle/pushHint/pushNotSetUp/pushDenied/pushUnavailable.
  - `rust/tests/common/{mod,commands}.rs` + `flow.rs`: fake_gateway gửi `push.config` (giá trị giả) sau hello, nhận `push.register`/`push.unregister`.
  - `test/push_test.dart` (18 test), `.code_index/mobile.md`.
- Shared `packages/shared/src/remote.ts`: `push.register`/`push.unregister` trong `RemoteCommand` + parse, `RemotePushConfig`, `RemotePushConfigEvent`, kind/data push. Test mới `apps/web/src/lib/remotePush.test.ts` (4 pass; ngoài phạm vi apps/web nhưng là chỗ test của shared, file mới, không sửa file cũ).
- Docs: `docs/remote-integration.md` §7.2, `docs/orrery-remote.md` (dòng push).

### Đã REVERT (relay, theo lệnh main)
Xoá `apps/relay/src/fcm.ts`, `apps/relay/src/push.ts`, `apps/relay/test/push.test.ts`; trả `room.ts`, `envelope.ts`, `env.ts`, `vitest.config.ts` về HEAD. `git status apps/relay` sạch; `npm test` 34 pass, tsc sạch.

### Shell — giữ nguyên, KHÔNG mở rộng (do bản thiết kế cũ, main briefing lại)
File tôi đã chạm trong `apps/shell/src/remote/`: `push.rs` (mới: text cố định vi/en, frame control `push`, giới hạn 1/pane/30 s + 30/giờ/thiết bị, queue, `push.register`/`unregister`, `dead_token`), `push_tests.rs` (mới, 9 test), `devices.rs` (push_token/push_locale, set/clear), `ctx.rs` (field `push`, `push_wake`, `is_live`), `mod.rs` (mod push/push_tests), `relay.rs` (gửi frame push, nhận `push.dead`), `phone_ws.rs` (2 arm), `pane_link.rs` (`attention`, hook trong `call` đọc `reason`). Lưu ý: shell hiện còn thay đổi của người khác (cf_api.rs, relay_setup.rs, ccm.rs...) và `mod.rs` đã bị sửa thêm. Thiết kế mới cần: shell gửi event `push.config` sau hello; frame push theo thiết kế mới (không qua control relay `push`/`push.dead` nếu bỏ relay) — phần relay.rs/push.rs hiện còn logic "relay control push" cần main dọn.
- Core (hot): `with_pane_link` chưa truyền `reason` vào `remote.link`, và chỉ chạy khi Telegram bật.

### Tiêu chí
- Relay (test JWT/FCM): HUỶ theo thiết kế mới. Shell cargo test: chưa chạy lại sau thay đổi của main (trước đó `cargo test --locked -j 3 remote` 75 pass, gồm 9 test push). Mobile: analyze sạch, `flutter test --concurrency=4` 179 pass (17 skip tag integration), test register gửi đúng lệnh + tap -> TabPage đạt. Shared parse: 4 pass + typecheck sạch. vi+en, không comment, file < 400 dòng, docs+index xong.

### Audit / Review
- Không log token/options. Token chỉ gửi qua phiên Noise tới gateway. Options validate ký tự trước khi dùng. `push.unregister` khi xoá máy (phone) và khi tắt switch; xoá token FCM (`deleteToken`) khi tắt.
- Edge: token đổi -> gửi lại cho máy cùng project; quyền bị từ chối -> toast, switch giữ tắt; app bị kill -> `getInitialMessage` + options lưu SecureKv; thu hồi thiết bị -> gateway đã xoá token (shell); relay mất kết nối -> không ảnh hưởng phía phone.
- RỦI RO QUAN TRỌNG: `firebase_messaging` chỉ hỗ trợ app Firebase MẶC ĐỊNH ("Messaging does not yet support multiple Firebase Apps"). Không thể có `Firebase.app(name)` riêng mỗi projectId như brief. Thực hiện: dùng project của máy đầu tiên có config; máy khác project không đăng ký push. Cần chủ nhân/main quyết định (mỗi máy 1 project chung?).
- firebase_messaging 17: `getToken`/`deleteToken` deprecated (thay `register`/`onRegistered` trả FID, chưa rõ FCM v1 chấp nhận FID); tôi vẫn dùng `getToken` qua `dynamic`.
- Config lưu `SecureKv` (không phải `MachineCache`) vì cache bị xoá cùng snapshot; không tự xoá khi forget máy (nhỏ).

### Test (lệnh -> kết quả)
- `cd apps/relay && npm test && npx tsc --noEmit` -> 34 pass, sạch (sau revert)
- `cd apps/mobile && flutter analyze` -> No issues; `flutter test --concurrency=4` -> 179 pass
- `cd apps/mobile/rust && cargo test --locked -j 3` -> 13 pass (flow, gồm push + push.config)
- `npm test -w @ccm/web -- src/lib/remotePush.test.ts` -> 4 pass; `npm run typecheck -w @ccm/shared` -> sạch (`npm test -w @ccm/shared` không có script)

### Nhìn
CHƯA NHÌN / CHƯA KIỂM: cần Firebase project thật + build APK; Firebase.initializeApp runtime, FCM thật, channel `attention`, notification tap trên A21s chưa chạy. Integration Dart (tag) chưa chạy lại.

### Hot file cần thêm
- `apps/core` `telegram/pane_link.rs` (truyền `reason`, gọi `remote.link` cả khi Telegram tắt).
- Shell: gửi `push.config` sau hello (mod/phone_ws), dọn logic relay control push cũ.

### Việc chủ nhân/main cần làm
Tạo Firebase project, nhập config (apiKey/appId/senderId/projectId) vào cài đặt desktop (do shell mới); không cần google-services.json. Build APK để thử.

### Commit message đề xuất
feat(mobile): FCM push notifications with Firebase initialized at runtime from gateway-provided options, notification tap opens the pane, switch in the machine list

## Vòng 2 (fix review flutter, chỉ apps/mobile)
- CAO 1: tile hiện hint pushForeign (vi/en) khi máy khác project Firebase; máy foreign không nhận `push.register`. CAO 2: `setEnabled` dùng config project đã init / máy được chạm, bỏ `configs.first`.
- TRUNG BÌNH: listen `onMessageOpenedApp`/`onTokenRefresh` sau init; ghi chú không cần background handler (payload có `notification` + channel attention); pending `push.unregister` lưu kv; kiểm quyền lúc khởi động + resume (revoked -> switch tắt + hint); bỏ `dynamic`, catch typed (`PushException`) báo lỗi vào state tile, dispose huỷ subscription; `deprecated_member_use` ignore trong analysis_options.
- THẤP: `RemoteHostPush.dispose`, serialize toggle, `buildPushService` log lỗi, gradle kts format, `dart format` file chạm tới.
- Test: 23 + UI tests, `flutter test --concurrency=4` 187 pass, `flutter analyze` sạch. Test tách push_test / push_ui_test / support/push_fakes (<=420 dòng).
- LƯU Ý: lệnh `dart format test lib` lỡ format thêm nhiều file ngoài WP-14 trong apps/mobile/lib/ui/** (chỉ khoảng trắng, test vẫn pass); không có git revert nên giữ nguyên, main cân nhắc khi commit.
- Còn lại: tile toàn cục nên máy được chạm khi chưa init = máy vừa gửi `push.config` gần nhất (`_candidate`), chưa có switch riêng từng máy.
