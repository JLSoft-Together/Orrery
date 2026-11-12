# WP-56a báo cáo: push có nội dung + Web Push (VAPID, RFC 8291) + chuông desktop + ngoại lệ thứ 7

Ngày 2026-11-13, nhánh `feat/v98-pwa`, chưa commit. Hợp đồng đầy đủ: `docs/remote-integration.md` §7.6 (mới), §7.2 / §7.4 / §5.2 (sửa); hướng dẫn người dùng `docs/orrery-remote.md` §13; `CLAUDE.md` Core principle "Seventh exception".

## Thiết kế (quyết định để reviewer kiểm)

1. **Crypto chỉ bằng `ring` 0.17 đã có** (không thêm crate): ECDH P-256 (`agreement`), HMAC-SHA-256 (HKDF viết tay 2 hàm 5 dòng để so được `PRK_key`, `IKM`, `PRK` với Appendix A — `ring::hkdf` không lộ PRK), AES-128-GCM (`aead`), ECDSA P-256 FIXED (`signature`). `p256` / `aes-gcm` / `hkdf` RustCrypto không cần. Vector RFC cần khoá gửi cố định: test dùng `ring::test::rand::FixedSliceRandom` (API ẩn, `#[allow(deprecated)]`, chỉ trong test).
2. **Mã hoá** `aes128gcm`: 1 record, `rs` 4096, header 86 byte, `0x02` rồi đệm 0 tới bội 64 byte (che độ dài, ≤ 3993). Khoá ECDH + salt mới mỗi tin. `p256dh` kiểm thật là điểm P-256 bằng một lần ECDH thử lúc đăng ký. RFC 8291 ghi `Content-Length: 145` nhưng body của chính RFC là 144 byte — test so body, ghi chú trong docs.
3. **VAPID**: khoá ECDSA P-256 PKCS#8 mỗi máy, keychain `orrery-vapid` (đọc lại kiểm khớp như `Keys::load_or_create`), JWT ES256 `{aud: origin endpoint, exp: +12 h, sub}` ký mới mỗi lần gửi. `sub`: `config.json webpushSubject` → URL relay https → `mailto:orrery-remote@localhost`.
4. **Gửi**: `TTL 3600`, `Urgency high`, `Topic` 32 ký tự theo pane, không theo redirect; 2xx gửi, 404/410 xoá subscription của thiết bị đó, 429/5xx/offline thử lại 3 lần (2 s, 4 s, như FCM), 401/403 `denied`, còn lại `rejected`.
5. **Whitelist endpoint**: chỉ `https://`, không userinfo, cổng rỗng / 443, host chữ thường `web.push.apple.com`, `fcm.googleapis.com`, `updates.push.services.mozilla.com`, `<nhãn>.notify.windows.com`; ≤ 2048 ký tự in được, không `#`. Lý do: desktop sẽ POST tới URL do điện thoại đưa — không cho thành SSRF.
6. **Nội dung** (#16): nguồn = Workspace summary (agent, tiêu đề pane, tab, project) + Telegram episode của core (câu hỏi `ask.question`, tool xin quyền). Vỏ luôn bật `telegram_on`, nên episode luôn tới `episodes.rs`; móc 1 chỗ → `Remote::note_episode` (cache RAM 256 terminal / 10 phút). Thời gian "Xong sau 12m" đo trong `push_watch` (working → idle). Job chờ `settle` 1,2 s trước khi giao để episode kịp tới; `attention` được tinh thành `question` / `permission`. Mask = `core_desk::claude::mask::mask_hook_text` (đã có, không regex mới), bỏ control + bidi/ẩn (`pair::is_hidden`), cắt 120 code point có `…`, không bỏ dở dấu kết hợp / VS / skin tone. `content` tắt, `security`, `test` → chữ cố định cũ.
7. **FCM giữ chữ cố định** cho tới WP-56b (Google sẽ thấy plaintext); chỉ Web Push (E2E) mang nội dung. `push.register` FCM nhận + lưu `content` sẵn cho WP-56b.
8. **Một SW push riêng mỗi máy** (`/app/push-sw.js`, scope `/app/push/<machineId>/`): một PushSubscription gắn một `applicationServerKey`, mỗi máy có khoá VAPID riêng, chủ nhân có PC + Mac → cần nhiều registration. SW chính `sw.js` không đổi và không precache file này.
9. **Đăng ký từ web app**: xin quyền chỉ trong `push.enable()` từ nút "Bật" (iOS: chỉ khi standalone), lưu `notify.push`; `push.register` gửi mỗi lần nhận `webpush.config` (desktop gửi sau mỗi `hello` của platform `web`) và khi chuông / nhóm / "Kèm nội dung" đổi. Chuông tắt = `mute` cả 3 nhóm (cảnh báo bảo mật vẫn tới). "Kèm nội dung" mặc định bật (#18; trước đó mặc định tắt).
10. **Không push khi app mở** (#20): desktop đã bỏ thiết bị có phiên; SW cũng bỏ nếu có cửa sổ focus + visible, riêng WebKit hiện rồi đóng ngay (Safari thu hồi subscription nếu push không `showNotification`).
11. **Chuông desktop** (#19): Settings → Điều khiển từ xa → khối thiết bị → "Thông báo điện thoại · Bật n / N" mở danh sách; mỗi điện thoại trạng thái icon + chữ + màu và công tắc → `devices.json pushOff` (tắt hẳn, kể cả cảnh báo bảo mật). Bộ giới hạn 1 / pane / 30 s, 30 / giờ / thiết bị dùng chung FCM + Web Push (theo `deviceId`).

## File + symbol

- Shell mới: `remote/webpush.rs` (`derive`, `seal`, `encrypt(_with)`, `pad_len`, `valid_public`, `Vapid`), `webpush_send.rs` (`valid_endpoint`, `known_host`, `topic`, `PushHttp`, `WebPush::send`, `status_outcome`), `push_content.rs` (`Facts`, `Scene`, `scene_of`, `title_of`, `body_of`, `notice`, `refine`, `cut`, `clean`, `mask_text`), `push_web.rs` (`parse_sub`, `subject_for`, `web_payload`, `Ctx::{web_ready, note_episode, settled, notice, deliver_web, webpush_config_event}`, `Phone::push_register_web`, `Remote::note_episode`); test `webpush_tests.rs`, `push_content_tests.rs`, `push_web_tests.rs`, `webpush_live_tests.rs` (`#[ignore]`).
- Shell sửa: `push.rs` (`Job` + `Default`, `push_targets`, `push_pane`, `deliver` → web, `run` settle, `push_register` kind webpush + `content`), `push_watch.rs` (`Watch.started/worked`, `push_terminal_after`), `devices.rs` (`WebSub`, `web_push`, `push_content`, `push_off`, `set_web_push`, `set_push_off`, `clear_web_endpoint`, `push_channel`), `status.rs`, `push_setup.rs` (`push_device`, `push_test` gồm web), `ctx.rs` (`webpush`, `facts`, `settle`), `dashboard.rs` (`webpush.config`), `config.rs` (`webpush_subject`), `mod.rs` (kênh `ccm:remote-push-device`), `ccm.rs` LOCAL, `episodes.rs` (1 móc), `shim.js` sinh lại; test có sẵn chỉnh `Job { .. }` và số secret (3 → 4).
- Web app: `public/push-sw.js`, `src/push/webPush.ts`, `src/stores/push.ts`, `src/push/push.test.ts` (mới); `remote/hub.ts` (`pushKey`, `webpush.` event), `stores/hub.ts` (`onDevice`, `pushKey`), `storage/notifyPrefs.ts`, `stores/prefs.ts`, `views/SettingsView.vue` (`push-row`), `main.ts`, `i18n/{vi,en}.ts` (khoá mới `settings.push*`, sửa 2 câu mô tả), `vite/serviceWorker.ts` (SKIP), `test/dist.test.ts`; E2E `e2e/lib/push-steps.mjs` (mới), `scenario.mjs` (28 → 30 bước), `run.mjs` (`qaDirPush`), peer `conn.rs` / `data.rs`.
- Desktop web: `components/settings/remote/RemotePushBell.vue` + test, `SettingsRemote.vue`, `stores/remote.ts` (`setDevicePush`), `lib/remote.ts` (`remotePushDevice`), `lib/desktop.ts`, `i18n/remote-v98.ts` (`remoteV98.push.*`); `apps/desktop/src/preload.ts` (`remotePushDevice`); `packages/shared/src/remote.ts` (`RemoteDevice.push/pushOn/pushContent`).
- Docs: `CLAUDE.md`, `docs/remote-integration.md`, `docs/orrery-remote.md`, `.code_index/{shell,web,web-pwa,protocol,_meta}.md`.
- Không đụng: `apps/mobile`, `apps/relay`, `scripts/spike-v98`, `core-desk`.

## Test chạy thật

| Lệnh | Kết quả |
|---|---|
| `cd apps/shell && cargo test -j 4 --bin orrery-shell remote::` | **193 pass / 0 fail / 3 ignored** (trước 164; +9 webpush, +8 content, +9 web, +3 live ignored; 1 test cũ sửa số secret) |
| `cargo test -j 4 --bin orrery-shell` (toàn bộ) | **387 pass / 0 fail / 3 ignored** |
| `remote::webpush_tests::rfc8291_*` | vector §5 + Appendix A **khớp từng byte** (key_info, PRK_key, IKM, PRK, CEK, NONCE, header, ciphertext, body; ECDH từ khoá gửi cố định) |
| `remote::webpush_tests::rfc8292_example_token_verifies_with_its_key` | JWT mẫu RFC 8292 §2.4 verify bằng `k` mẫu; JWT của mình verify bằng public key, claims đúng |
| `cargo clippy -j 4 --bin orrery-shell --tests` | không warning mới (còn `mod.rs::call`, `devices.rs::set_push`, … có sẵn) |
| `cd apps/pwa && npx vitest run` / `vue-tsc` | **167 pass / 0 fail** (16 file; +11 push) / sạch |
| `npm run e2e -w @ccm/pwa` (`node e2e/run.mjs --no-peer-build`) | **60 pass / 0 fail, 60/60** (WebKit 30 + Chromium 30; bước "no CSP violation" pass) |
| `cd apps/web && npx vitest run` / `npm run typecheck -w @ccm/web` | **2151 pass / 0 fail** (255 file; +4 `RemotePushBell.test.ts`) / sạch |
| `npm run typecheck -w @ccm/shared`, `-w @ccm/desktop` | sạch |

E2E Chromium: PushManager giả qua init script (Chromium headless không subscribe được — GCM), quyền bằng `grantPermissions`; kiểm không gửi `push.register` trước khi bấm, `push.register` đủ field (`kind`, endpoint, p256dh, auth, `locale:"vi"`, `mute:["error"]`, `content:true`), subscribe đúng khoá VAPID của máy, 2 registration (`/app/` + `/app/push/<id>/`), chuông → mute 3 nhóm; CDP `ServiceWorker.deliverPushMessage` → `getNotifications()` đúng title / body / tag / `data.url`, tin `question` bị bỏ khi app focus, `security` vẫn hiện; message `orrery-open` → `/app/m/<id>/p/p1`. Click thông báo thật không giả lập được trong Playwright → handler `notificationclick` test bằng Vitest (SW chạy trong `new Function` với `self` giả). WebKit (không Push API): dòng trạng thái thay nút, `push-sw.js` được phục vụ đúng.

Sau E2E đã `git checkout -- docs/qa/v98/WP-55b` (ảnh bị ghi đè).

## Thử thật với Edge

Edge 157.0.4341.46, profile tạm trong scratchpad, quyền notifications đặt sẵn trong `Default/Preferences` (CDP grant chỉ là override — bài học WP-50bc), Edge chạy thẳng `msedge.exe --remote-debugging-port=9333` + Playwright `connectOverCDP` (Playwright `launchPersistentContext` thêm cờ tắt background networking → `subscribe` treo). Trang `http://localhost:8799` phục vụ đúng `apps/pwa/public/push-sw.js` (+ probe ghi log). Rust gửi qua đường thật `Ctx::deliver` (`webpush_live_tests::live_send`, `ORRERY_WEBPUSH_LIVE`):
- subscribe → `wns2-pn1p.notify.windows.com`; Rust → WNS **2xx (`Sent`)**; SW nhận sau ~1,5 s, **giải mã đúng**: `{"title":"Claude · Live push · orrery","body":"Gửi thật từ Rust tới WNS? API_KEY=••••","k":"question","tag":"<m>:p1","p":"p1"}` (mask chạy thật).
- Có cửa sổ app focus → banner bị bỏ (#20); không có cửa sổ → `showNotification` đúng title / body / tag, `data.url = /app/m/<m>/p/p1`.
- Gửi lại ngay sau `unsubscribe()` → WNS vẫn 2xx (WNS trả 410 muộn hơn, WP-50bc thấy 410 với subscription cũ) → không kiểm được 410 thật trong lượt này; 404 / 410 kiểm bằng fake HTTP.
- Log: `docs/qa/v98/WP-56a/edge-live.log` (chỉ host endpoint, không khoá / token). Edge + server đã tắt, profile tạm bỏ.

Bug thật tìm được nhờ Edge: `pushManager.subscribe` ngay sau `register` lỗi khi SW chưa `activated` (E2E giả không lộ) → `webPush.ts activated()` chờ SW trước khi subscribe.

## Ảnh

`docs/qa/v98/WP-56a/`: `desktop-bell-light.png`, `desktop-bell-dark.png`, `desktop-settings-remote-{light,dark}.png` (web dev + `ccmDesktop` giả), `settings-push-on-phone-light-chromium.png` (E2E). Ảnh thông báo trên màn khoá iPhone: chưa có (cần iPhone thật).

## Việc dở / rủi ro

- **iPhone thật chưa thử** (cần owner): quyền chỉ khi standalone; Apple có thể từ chối `sub` (mặc định URL relay https; `mailto:orrery-remote@localhost` chỉ khi không có relay — iPhone luôn có relay vì web app nằm trên relay); WebKit có cho **nhiều registration push trên 1 origin** không chưa kiểm — nếu không, cần gộp về 1 máy / origin; Safari thu hồi subscription nếu push không hiện thông báo (SW luôn hiện, focus thì hiện rồi đóng — banner có thể chớp).
- `settle` 1,2 s cộng vào độ trễ push; nếu episode tới muộn hơn, tin vẫn đi nhưng thiếu câu hỏi (rơi về "Đang chờ bạn").
- Episode `telegram` chỉ có khi core-serve chạy trong vỏ (mặc định); `CCM_CORE=node` (Electron cũ) → không có câu hỏi / tool, chỉ agent · session · project + thời gian.
- Tiêu đề pane có thể là tiêu đề tự đặt của Claude Code; chỉ bỏ ký tự spinner đầu.
- Dead code có sẵn không xoá: hook `self.orreryPush` trong `vite/sw.template.js` (không còn dùng, push nằm ở `push-sw.js`).
- `webpush_live_tests.rs` là test `#[ignore]` chỉ chạy tay; reviewer có thể muốn chuyển sang script riêng.

## Hợp đồng cho WP-56b (FCM mã hoá)

- Dựng nội dung dùng chung: `Ctx::notice(&Job) -> (title, body)` (đã tôn trọng `job.content`, `security` / `test` cố định, mask + cắt 120). `Job.content` = `Device.push_content` (đã lưu từ `push.register` FCM `content`), `Job.terminal` / `Job.worked` đã có, `Ctx::settled` tinh kind.
- Hiện `Ctx::deliver` FCM gọi `push::message(job)` chữ cố định — WP-56b thay bằng payload mã hoá bằng khoá thiết bị (data message), giữ `{m, p, k}`; chuyển sang `notice(job)` chỉ khi đã mã hoá.
- Giới hạn, mute, `push_off`, `is_live`, 404 / UNREGISTERED xoá token giữ nguyên trong `push_targets` / `push_pane` / `PushState` — không cần đổi.
- Kênh theo thiết bị: `Device::push_channel()`; `set_push_mute` (FCM) xoá `web_push` và ngược lại.

## Vòng 2 (sau review crypto độc lập: 0 chặn / 0 cao, 4 điểm thấp)

1. `webpush.rs audience()` bỏ `:443` → `aud` là origin chuẩn RFC 8292 (test `the_vapid_audience_is_the_origin_without_the_default_port`).
2. `push_content.rs clean()`: bỏ ký tự điều khiển / bidi / ẩn (khoảng trắng điều khiển → dấu cách) + gộp khoảng trắng **trước**, mask sau, cắt cuối; test `secrets_split_by_hidden_or_control_characters_are_joined_before_masking` (`API_​KEY=`, U+00AD, BEL, U+2066, `sk-ant-​…`, `Bearer abc‌…`).
3. `push.rs Ctx::still_wanted(job)` (thiết bị còn, `!push_off`, `web_push` == `job.web` hoặc `push_token` == `job.token`) kiểm ở đầu `deliver` và trước mỗi lần thử lại (`Fcm::send_while`, `WebPush::send(.., wanted)`); hết → `Fail("no_device")`, không gọi HTTP. Test `a_queued_push_is_dropped_when_the_phone_was_revoked_switched_off_or_resubscribed_meanwhile` (tắt, đổi subscription, thu hồi, tắt giữa 2 lần thử). `push_tests::job()` giờ dùng đúng id thiết bị; `webpush_live_tests::live_gone` lưu subscription thật. `Fcm::send` chỉ còn cho test (`#[cfg(test)]`).
4. `webpush_send.rs topic(machine, pane, auth)`: băm kèm `auth` của subscription (ổn định theo pane cho một thiết bị, khác giữa các thiết bị); test `the_topic_is_stable_per_pane_and_subscription_but_differs_across_phones`. `sub` mặc định giữ URL relay, rủi ro ghi docs §7.6.

Docs §7.6 cập nhật (thứ tự mask, kiểm lại trước khi gửi, Topic có `auth`, `aud` bỏ `:443`, rủi ro `sub`).

| Lệnh (vòng 2) | Kết quả |
|---|---|
| `cargo test -j 4 --bin orrery-shell remote::` | **197 pass / 0 fail / 3 ignored** (+4) |
| `cargo clippy -j 4 --bin orrery-shell --tests` | không warning mới ở file đụng (còn `devices.rs::set_push`, `fcm.rs::set_http` có sẵn) |
| `npm test -w @ccm/pwa` | **167 pass / 0 fail** |
