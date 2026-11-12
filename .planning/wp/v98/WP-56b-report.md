# WP-56b báo cáo: FCM có nội dung cho Android, mã hoá bằng khoá thiết bị

Ngày 2026-11-13, nhánh `feat/v98-pwa`, chưa commit. Hợp đồng: `docs/remote-integration.md` §7.7 (mới), §7.2 / §7.4 / §7.6 (sửa); hướng dẫn `docs/orrery-remote.md` §13; `CLAUDE.md` ngoại lệ thứ 7 (câu FCM).

## Sơ đồ mã hoá (để reviewer kiểm)

Sealed box tới khoá tĩnh X25519 của điện thoại (`devices.json pubkey`, chính khoá Noise đã xác thực lúc ghép):

```
E, e    = khoá tạm X25519 mới mỗi tin (ring SystemRandom)
ss      = X25519(e, D)                       ss toàn 0 → không mã hoá (gửi chữ cố định)
okm     = HKDF-SHA256(salt = E ‖ D, ikm = ss, info = "orrery-push/1" ‖ 0x00 ‖ machineId, L = 44)
key     = okm[0..32], nonce = okm[32..44]
aad     = "orrery-push/1" ‖ 0x00 ‖ machineId ‖ 0x00 ‖ k
plain   = JSON {t, b, p?} ‖ dấu cách tới bội 64 byte
e       = base64url_nopad(E ‖ ChaCha20-Poly1305(key, nonce, aad, plain) ‖ tag16)
FCM     = {message: {token, data: {v:"1", m, k, e}, android: {priority: high}}}   (data-only)
```

- Thư viện: desktop chỉ `ring` 0.17 có sẵn (`agreement::X25519`, `hkdf`, `aead::CHACHA20_POLY1305`), không thêm crate. Điện thoại: Dart thuần, package `cryptography` 2.9.0 (`DartX25519`, `DartHkdf(DartHmac(DartSha256()))`, `DartChacha20.poly1305Aead`). Không đi qua FRB vì: máy không có `flutter_rust_bridge_codegen` (sinh lại `frb_generated` đụng nhiều file sinh), isolate nền FCM sẽ phải `RustLib.init` thêm một điểm hỏng, và Dart thuần cho phép `check.mjs` kiểm thật vector Rust (2 bản cài độc lập so với nhau thay vì cùng một code).
- `k` để rõ vì (a) AAD cần nó trước khi giải mã, (b) giải mã lỗi vẫn hiện được chữ cố định đúng kind; Google trước đây đã thấy `k` qua chữ cố định. `p` (pane key) chỉ nằm trong bản mã ở tin data-only.
- Nonce: lấy từ HKDF; khoá khác nhau mỗi tin (khoá tạm mới) nên cặp (key, nonce) không lặp.
- Vector chung: `apps/mobile/test/fixtures/push_seal_v1.json` (khoá RFC 7748 §6.1, Bob = thiết bị, Alice = khoá tạm; có `shared`, `okm`, `e`, plaintext UTF-8 tiếng Việt). Rust `include_str!` và so từng byte; Dart mở và so; Node `crypto` (OpenSSL) mở độc lập khớp `shared`, `okm`, plaintext (script tạm trong scratchpad, không lưu repo).
- **Dùng chung khoá Noise — chọn dùng chung**: lộ khoá tĩnh = đã mất toàn quyền điều khiển (giả điện thoại với desktop), nên khoá push riêng không giảm thiệt hại mà thêm khoá phải lưu + trường `push.register` phải xác thực; domain `orrery-push/1` trong `info` + `aad` tách khỏi chuỗi băm Noise (`Noise_*_25519_ChaChaPoly_BLAKE2s`); điện thoại không bao giờ trả DH / plaintext ra mạng (không có oracle); chỉ desktop (giữ service account) gửi được tới token. Đổi lại: không có forward secrecy cho bản mã Google có thể lưu (giống RFC 8291).

## Chống mất thông báo

Ba mức theo thiết bị (`Ctx::fcm_message`):
1. **Cố định** (`push::message` y như trước): app cũ (không gửi `seal`), "Kèm nội dung" tắt, khoá thiết bị hỏng / điểm bậc thấp, `security` (job luôn `content: false`), mã hoá lỗi, `data` > 4096 byte.
2. **Thăm dò** (có `seal`, chưa `sealOk`): `push::message` nguyên vẹn (khối `notification` cố định → hệ thống tự hiện dù handler có chạy hay không) + `data.v`, `data.e`. Handler nền (FlutterFire gọi cả với tin có `notification`, đã đọc `FlutterFirebaseMessagingReceiver` 17.0.1) không hiện thêm; nếu mở được bản mã trong isolate nền **và** `flutter_local_notifications.initialize` trả `true` → kv `push.sealOk = on`.
3. **Data-only** (`sealOk: true` ở `push.register` gần nhất): app tự giải mã + hiện.

- Bằng chứng chỉ đến từ chính đường nền của chính điện thoại đó (đọc `SecureKv` trong isolate nền, giải mã, plugin thông báo đăng ký được). Điện thoại mà handler nền không bao giờ chạy (OEM, lỗi engine) mãi ở mức 2 = như trước, không mất.
- Data-only mà plugin hiện lỗi → thử lại bằng chữ cố định → vẫn lỗi thì `push.sealOk = off` → lần đăng ký sau về mức 2. Tin đó mất (plugin hỏng thì không hiện được gì từ app), các tin sau đi lại đường hệ thống tự hiện.
- Giải mã lỗi (thiếu / sai khoá, `v` lạ, `k` / `m` bị sửa, byte hỏng) → chữ cố định theo kind (ARB `pushFixed*` = `push.rs title_body`), ngôn ngữ hệ thống (giống `locale` của `push.register`).
- App mở: hệ thống không hiện tin foreground nên `onMessage` → `PushPresenter` hiện (cả tin cố định — trước đây foreground không hiện gì), trừ khi `resumed` và phiên tới đúng máy `m` đang ready (#20).
- Không khắc phục được (như trước): force stop / "Deep sleeping apps" của Samsung chặn mọi tin FCM, kể cả có `notification`.

## File + symbol

- Shell mới: `apps/shell/src/remote/push_seal.rs` (`info`, `aad`, `derive`, `padded`, `device_key`, `seal_with`, `seal`, `sealed_plain`, `data_size`, `sealed_message`, `probe_message`, `sealed_fcm`, `Ctx::fcm_message`), `push_seal_tests.rs` (8 test).
- Shell sửa: `push.rs` (`Job.seal_key` / `seal_ok`, `push_targets`, `deliver` → `fcm_message`, `push_register` đọc `seal` / `sealOk`), `devices.rs` (`push_seal`, `push_seal_ok`, `set_push_seal`), `push_setup.rs` (`push_test` giữ `content` để "Gửi thử" đi đường mã hoá), `mod.rs`, `push_tests.rs` (`TOKEN_A/B`, `ready` → `pub(super)`).
- Mobile mới: `lib/push/push_seal.dart` (`openSeal`, `pushSealOkm`, `readStaticKey`), `push_notice.dart` (`PushArrival`, `PushNotice`, `pushNotice`, `fixedPushNotice`, `fixedPushText`, `shouldShowPush`, `pushTapData`, kv `push.sealOk` / `push.content`), `push_background.dart` (`pushBackgroundMessage` `@pragma('vm:entry-point')`, `handleBackgroundPush`), `push_present.dart` (`PushPresenter`), `local_notifier.dart` (`PushNotifier`, `LocalPushNotifier`), `push_host.dart` (tách `PushSession` / `PushHost` / `pushLink` / kv keys khỏi `push_service.dart`, re-export, để file < 400 dòng).
- Mobile sửa: `push_gateway.dart` (`sealing`, `arrivals`, `show`, `_listen`, `initialOpen` + launch payload), `push_service.dart` (`content`, `setContent`, `resumed`, register `content` / `seal` / `sealOk`), `push_providers.dart` (`PushState.content`, switch `push-content`), `app/bootstrap.dart` (`resumed`), ARB vi / en (16 khoá: `pushContent*`, `pushFixed*`; sửa tay), `pubspec.yaml` (`cryptography: ^2.9.0`, `flutter_local_notifications: ^22.3.3` — bản 23 đòi compileSdk 37 + AGP 9.1.1), `android/app/build.gradle.kts` (core library desugaring, bắt buộc với plugin), test `push_test.dart` (register thêm field), `push_ui_test.dart` (switch), `support/push_fakes.dart`, mới `push_seal_test.dart`, `push_present_test.dart`, `support/push_vector.dart`, `fixtures/push_seal_v1.json`.
- Không đụng: `MainActivity.kt` (channel `attention` đã có; `visibility public` đặt theo thông báo), `apps/mobile/rust`, `apps/pwa`, `apps/relay`, `apps/web`, `scripts/spike-v98`.
- Docs / index: `CLAUDE.md`, `docs/remote-integration.md`, `docs/orrery-remote.md`, `.code_index/{shell,_meta}.md`, `apps/mobile/.code_index/mobile.md`.

## Test chạy thật

| Lệnh | Kết quả |
|---|---|
| `cd apps/shell && cargo test -j 4 --bin orrery-shell remote::` | **205 pass / 0 fail / 3 ignored** (trước 197; +8 `push_seal_tests`) |
| `cargo test -j 4 --bin orrery-shell` (toàn bộ) | **399 pass / 0 fail / 3 ignored** |
| `cargo clippy -j 4 --bin orrery-shell --tests` và không `--tests` | không warning mới ở file đụng (còn `devices.rs::set_push` có sẵn; `TAG_LEN` chỉ dùng trong test đã chuyển vào file test) |
| `node apps/mobile/scripts/check.mjs` | gen-l10n OK, format OK, **analyze sạch**, **flutter test 291 pass / 0 fail / 63 skip** (trước 278; +7 `push_seal_test`, +6 `push_present_test`; `push_ui_test` thêm kiểm switch), không file lib > 400 dòng |
| Node `crypto` (OpenSSL) mở vector | `shared`, `okm`, plaintext khớp |

Mobile Rust (`apps/mobile/rust`) không đụng nên không chạy `cargo test` ở đó. Không build APK (luật).

## Bước QA trên Samsung A21s (chủ nhân)

Chuẩn bị: build + cài app mới, desktop bản mới đã cài Firebase; trên điện thoại bật Thông báo, cho phép quyền. Desktop Settings → Điều khiển từ xa → chuông thiết bị phải ghi "FCM · kèm nội dung".
1. **Build**: `flutter build apk` phải qua `checkDebugAarMetadata` (desugaring) — nếu lỗi compileSdk / AGP, báo lại (đã ghim plugin 22.3.x cho compileSdk 36).
2. **Thăm dò, app nền**: mở app (phiên sẵn sàng → `push.register` có `seal: 1, sealOk: false`), bấm Home, desktop "Gửi thử" → thấy **1** thông báo "Orrery · Thông báo thử…" (không nhân đôi). Mở lại app (đăng ký lại) → `devices.json` thiết bị có `pushSeal: true, pushSealOk: true`.
3. **Data-only, app nền**: Home, để một pane Claude hỏi câu hỏi → thông báo tiêu đề `Claude · <session> · <project>`, thân là câu hỏi (≤ 120 ký tự, secret bị che), icon cam, chạm mở đúng pane.
4. **App bị kill**: vuốt app khỏi Recents (không Force stop), lặp bước 3 → vẫn hiện nội dung; chạm → app khởi động và mở đúng pane (đường `getNotificationAppLaunchDetails`).
5. **Màn khoá**: tắt màn hình, lặp bước 3 → nội dung đủ trên màn khoá (nếu Samsung "Ẩn nội dung" của hệ thống bật thì chỉ thấy tên app — kiểm Cài đặt → Thông báo → Màn hình khoá → "Hiện nội dung").
6. **Gộp theo pane**: 2 tin cùng pane cách > 30 s → chỉ còn 1 thông báo (tin mới thay tin cũ); pane khác → thông báo riêng.
7. **App mở**: đang ở Workspace của máy A, pane trên máy A hỏi → desktop không gửi (đúng #20). Đang mở app nhưng máy B không có phiên → tin từ B hiện như banner.
8. **"Kèm nội dung" tắt** trên điện thoại → tin về chữ cố định ("Agent đang hỏi"); bật lại → nội dung.
9. **Samsung**: Cài đặt → Pin → giới hạn nền: để app trong "Ứng dụng ngủ" → thử bước 3; "Ngủ sâu" → mong đợi: không có thông báo nào (cả cố định) — giống trước WP-56b, ghi kết quả. Bật "Tối ưu pin" mặc định và thử sau ~1 giờ màn tắt (Doze).
10. **Bảo mật**: ghép thêm một thiết bị → cảnh báo bảo mật trên A21s là chữ cố định.
11. `adb logcat | grep -i -E "orrery|FLTFireMsg"`: không có khoá, token, tiêu đề hay câu hỏi trong log.

## Rủi ro (cho review độc lập)

- **Chưa thử trên máy thật / chưa build APK**: desugaring + plugin 22.3.3 với AGP 9.1.0 / Kotlin built-in chưa được Gradle xác nhận; channel name truyền vào plugin chỉ dùng khi channel chưa có (MainActivity luôn tạo trước).
- **Bằng chứng một lần**: `sealOk` chứng minh đường nền chạy ở một thời điểm; nếu về sau handler nền không chạy (vd. cập nhật app phá callback handle, OEM đổi chính sách) mà process vẫn nhận broadcast, tin data-only có thể mất trong khi tin `notification` thì không. Tự sửa chỉ khi plugin ném lỗi (→ `off`). Có thể thêm: thăm dò lại định kỳ (vd. 1 tin / ngày đi mức 2).
- **Không chống phát lại**: Google / FCM có thể giao lại bản mã cũ (TTL mặc định FCM 4 tuần); không có thời điểm trong plaintext. Như tin cố định trước đây.
- Không forward secrecy (khoá tĩnh dùng chung với Noise; phân tích ở trên).
- Foreground giờ hiện cả tin cố định khi app mở nhưng không có phiên tới máy đó (trước đây không hiện gì) — đúng #20, nhưng là thay đổi hành vi.
- `flutter_secure_storage` trong isolate nền khi máy vừa khởi động lại chưa mở khoá lần đầu: app không chạy được tới khi mở khoá (credential storage), tin chờ / fallback chữ cố định — cần xem trên A21s.
- `security` luôn đi chữ cố định (cả với điện thoại đã chứng minh) — chủ ý: cảnh báo bảo mật không phụ thuộc handler nền.

## Vòng 2 (sau review crypto độc lập: 1 vừa, 3 thấp)

1. **[VỪA] `sealOk` hết hạn 48 h**: `devices.rs Device.push_seal_at` (giờ `push.register` gần nhất có `sealOk`, 0 khi không), `set_push_seal(id, seal, ok, now)`; `push_seal.rs SEAL_OK_TTL = 48 h`, `seal_fresh(device, now)`; `push_targets` chỉ đặt `Job.seal_ok` khi còn hạn → quá hạn quay về thăm dò (chữ cố định + bản mã). Lý do: Samsung "Put unused apps to sleep" có thể chặn isolate nền cho tin data-only trong khi tin có `notification` vẫn hiện; app không được mở 48 h là dấu hiệu đó. Mở app lại → đăng ký mới (kv `push.sealOk` vẫn `on`) → data-only. Test `the_proof_expires_after_48_hours_and_the_phone_falls_back_to_the_probe`. Docs §7.7.
2. **[THẤP] Xác thực người gửi** — sơ đồ **v2** (`orrery-push/2`, `data.v = "2"`): `ikm = X25519(e, D) ‖ X25519(d_desktop, D)`, `salt = E ‖ D ‖ M`. Desktop: `core_remote_proto::StaticKeypair::agree` (mới, snow Curve25519 đã có, từ chối kết quả toàn 0; đụng `apps/core/crates/core-remote-proto/src/keys.rs` ngoài phạm vi gốc, cần để dùng khoá Noise của desktop mà không gọi `ring::test`). Điện thoại: `pairedMachineKey(MachineRepository, m)` (máy đã ghép, chưa thu hồi, `mkey` 32 byte) → không phải máy đã ghép = chữ cố định. Phiên bản: **v1 chưa từng phát hành nên bỏ hẳn** — app gửi `seal: 2`, desktop chỉ nhận `seal >= 2`; `seal: 1` = như app cũ (chữ cố định). Vector đổi thành `apps/mobile/test/fixtures/push_seal_v2.json` (thêm `desktopPrivate` = scalar RFC 7748 §5.2, `desktopPublic`, `staticShared`); Rust, Dart và Node/OpenSSL khớp từng byte; test mới: mở bằng khoá desktop khác → hỏng (Rust + Dart), máy chưa ghép → cố định.
3. **[THẤP] `CLAUDE.md`**: câu FCM ghi rõ Google thấy bản mã cùng machine id + kind, gắn khoá tĩnh desktop, chữ cố định khi máy chưa ghép, bằng chứng trong 48 h.
4. **[THẤP] Link chỉ tới máy**: `push_host.dart pushLink` không có `p` → `orrery-remote://m/<m>`; `pane_link.dart PaneLink.paneKey` nullable, `parsePaneLink` nhận 1 segment; `deep_link_host.dart` mở `WorkspacePage` rồi dừng (không chờ pane). `p` có mà sai vẫn trả `null`. Test: `pane_link_test`, `push_seal_test` (data-only mở lỗi → link máy), `push_ui_test` (chạm tin không pane → `WorkspacePage`). Docs §7.1 / §7.2 (tin thử giờ mở máy).

| Lệnh (vòng 2) | Kết quả |
|---|---|
| `cargo test -j 4 --bin orrery-shell remote::` | **206 pass / 0 fail / 3 ignored** (+1) |
| `cargo test -j 4 -p core-remote-proto` (apps/core) | **48 pass / 0 fail** (8 nhóm) |
| `cargo clippy -j 4 --bin orrery-shell --tests`, `-p core-remote-proto` | không warning mới ở file đụng (còn `devices.rs::set_push`, `core-desk disk/space.rs` có sẵn) |
| `node apps/mobile/scripts/check.mjs` | analyze sạch, **295 pass / 0 fail / 63 skip** (+4 so vòng 1) |
| Node `crypto` mở vector v2 | `desktopPublic`, `shared`, `staticShared`, `okm`, plaintext khớp |

QA A21s thêm (sau bước 11 ở trên):
12. **Hết hạn 48 h**: sau bước 3 (đang data-only), không mở app 3 ngày (để máy tính chạy), rồi cho pane hỏi → vẫn thấy thông báo (bản cố định do hệ thống hiện, tức đang thăm dò); `devices.json` `pushSealAt` cũ hơn 48 h. Mở app một lần → lần push sau lại có nội dung. Lặp với Samsung Cài đặt → Pin → "Đưa ứng dụng không dùng vào trạng thái ngủ" bật.
13. **Máy chưa ghép / đã quên**: quên máy trên điện thoại khi desktop chưa kịp nhận `push.unregister` (máy offline) → push từ máy đó hiện chữ cố định, không có nội dung.
14. **Tin không pane**: "Gửi thử" → chạm → app mở Workspace của máy đó (không mở pane).
