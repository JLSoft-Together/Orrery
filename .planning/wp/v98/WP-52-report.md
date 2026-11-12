# WP-52 — báo cáo: ghép 2 đường (QR v2 LAN + relay + vé 1 lần), hộp duyệt có IP

**Kết luận: XONG phần code + test trên máy này.** Điện thoại khác mạng ghép được qua relay bằng vé 1 lần do desktop mở; desktop hỏi duyệt native có IP / quốc gia / đường; sai mã 5 lần huỷ QR; mã sống 2 phút. Chưa kiểm trên Android thật (máy này không có điện thoại) và chưa chạy E2E với dev app Tauri thật + `wrangler dev` (lý do ở "Việc dở").

## Thiết kế đã chốt + lý do

### QR v2 (`core-remote-proto/src/pair/qr.rs`)
- `base64url(JSON {v:2, machine, name, lan?: ["ip:port", …], relay?, pt?, mkey, code, exp})`. Giữ tên field đầy đủ như v1 (spike / vector WP-50a sửa thẳng `wire.mkey`, đọc log dễ). `lan`: mọi IP gateway đang nghe, max 4, host không có `:` (chỉ IPv4 / tên), port 1–65535. `pt` chỉ khi relay `online` lúc tạo QR, và bắt buộc đi kèm `relay`.
- Chặn trộn: v2 không được có `host`/`port`, v1 không được có `lan`/`pt` → `BadQr`. `encode` chỉ ra v2; `decode` đọc cả v1 (host → 1 `LanAddr`, thiếu port → 4320).
- Cỡ thật (test `v2_carries_every_lan_route_relay_and_ticket_and_stays_small`): 2 LAN + relay workers.dev + `pt` = **410 ký tự**; 1 LAN không relay = **263**. Không giữ được ≤ 300 khi có relay vì base64 của JSON; viết tắt key chỉ bớt ~40 ký tự, không đáng phá hình dạng v1. Desktop vẽ QR mức sửa lỗi `L` (≈ version 13 thay vì 15 ở mức M). Giới hạn đọc giữ 640.
- `pt` là 16 B ngẫu nhiên riêng (không suy ra từ `code`): lộ vé (log URL relay) không cho biết gì về PSK; relay chỉ thấy vé, không thấy mã.

### Vé ghép relay + control (relay `src/pair.ts`, `room.ts`, `index.ts`)
- Desktop → relay qua `connId = 0` trên socket `/m/` đã ký: `{t:"pair.open", h: base64url(SHA-256(ASCII pt)), exp}` (relay nhận khi `now < exp ≤ now + 300`, lưu `pair_h` / `pair_exp` trong meta SQL, 1 vé / máy) và `{t:"pair.close"}`. Socket desktop bị thay (`closing`) bị bỏ qua; desktop nối mới → relay xoá vé, desktop gửi lại vé hiện hành ngay khi nối (`Pairing.armed` là `watch`).
- Phone `/p/<id>?pt=<vé>` (không cần `t`): `pt` khớp `[A-Za-z0-9_-]{22}`, SHA-256 so hằng thời gian (`sameText`), `now < exp`. Đúng → kiểm desktop có mặt + chỗ trống → **xoá vé rồi mới accept** (mỗi vé đúng 1 kết nối; 2 socket cùng vé thì cái sau `4401`). Sai / hết hạn / đã dùng → `4401` (`bad ticket` / `expired`) + 1 lỗi cho IP (`Strikes`, RAM của DO, 20 / 10 phút → `4429`). Không có desktop → `4503`, vé giữ.
- `open` gửi desktop: `{t:"open", conn, ip?, country?, pairing?}` cho mọi phone (ip/country dùng cho hộp duyệt và cho WP-59 #27), `pairing:true` chỉ conn vé. Worker dựng header nội bộ mới `x-orrery-ip` / `x-orrery-country` từ `cf-connecting-ip` / `cf-ipcountry` (header client cùng tên không lọt vào DO); DO lọc lại bằng regex.
- Proto: `relay::Control` thêm `ip`, `country`, `pairing` (serde default, `skip_serializing_if` → `encode_control` cũ ra byte y hệt), `Control::new`; `encode_pair_open` / `encode_pair_close`. Bỏ `Copy` của `Control` (có `String`).

### Đóng sau `pair.ok` qua relay — làm cả 3 phía
- Chọn: **relay nhận control close `1000` nhưng chỉ cho conn `pairing`** (conn token: bỏ qua) + **phone vẫn tự đóng** sau `pair.ok` / `pair.denied`. Lý do: desktop chủ động dọn conn ghép (phone lỳ không treo socket ở relay), relay cũ chưa cập nhật vẫn đúng vì phone tự đóng; giới hạn `1000` vào conn ghép để không đổi hành vi các đường đóng `1000` khác của gateway trên relay.
- Desktop: `relay.rs` gửi `encode_close(conn, 1000)` khi `Conns.pairing(conn)`; LAN như cũ. Android: `finally socket.close()` có sẵn. Web/spike: kiểm `closed 1000 paired`.

### Desktop gateway (`apps/shell/src/remote`)
- `pair.rs`: `PAIR_TTL` 120 s; `Pairing::fail` đếm lỗi msg3 theo `serial` của QR đang hiện, lần thứ 5 → huỷ slot, `ended()="tries"` (status `pairEnded`), disarm vé. Đếm cả lỗi `BadPayload` msg3 (platform / tên sai dạng) vì đó cũng là msg3 hỏng. Lỗi trước msg3 (proto, timeout) không đếm.
- `Link.origin: Origin {ip, country, pairing}`; `session::serve`: conn `pairing` mà msg1 ≠ 50 B → `4400` trước mọi thứ (IK trên conn ghép bị chặn, kể cả thiết bị đã ghép). Conn token vẫn được phép XXpsk3 (thiết bị đã ghép ghép lại) — không siết thêm vì vẫn cần mã + duyệt.
- `try_key(&Link)`: relay có IP → `relay:<ip>` 5 / phút (trước là khoá chung `relay` 20 / phút, giờ chỉ còn khi relay không báo IP).
- Hộp duyệt: `Host::confirm_pair(&PairAsk {name, platform, ip, country, via})`, chữ ở `text.rs pair_title`: vi "Ghép «Pixel 9» (android) từ IP 203.0.113.7 [VN] qua relay?" / en "Pair “Pixel 9” (android) from IP 203.0.113.7 [VN] via relay?"; ip / country / platform sai dạng → bỏ ("IP không rõ"). Dùng `crate::hosts::confirm` có sẵn. Từ chối / 120 s → `pair.denied` + đóng 1000. Platform từ msg3 lưu vào thiết bị ngay.
- `pair_start` → `{qr, exp, relay, lan}`; status `pairing: {exp, relay} | null`, `pairEnded`.
- msg3 có `platform?` mới: `PairInitiator::start_on(.., platform, ..)` (giữ `start` cũ, vector cũ không đổi byte), `PairedDevice.platform`, `valid_platform` `[A-Za-z0-9]{1,16}`.

### Điện thoại Android (`apps/mobile`)
- FRB `QrInfo` **không đổi** (máy này không có `flutter_rust_bridge_codegen`; đổi struct là phải sinh lại + đổi content hash). Rust `decode_qr` vẫn kiểm toàn bộ QR (v1 / v2); Dart `readQrRoutes` (`pair_routes.dart`) chỉ đọc thêm `lan[]` + `pt` của v2 sau khi Rust đã nhận QR. `pair_start` Rust gọi `start_on(.., "android", ..)` (chỉ đổi thân hàm).
- `Pairing.pair(qr, onRoute:)`: báo `lan` → `openPairLan` thử song song mọi `lan` + mDNS, 3 s (`pairLanBudget`), socket đầu tiên thắng, socket mở muộn bị đóng. Hỏng **trước khi gửi msg 3** (không mở được, đóng, hết giờ, lỗi giao thức, `mkey` lệch) và QR có `relay` + `pt` → báo `relay`, bắt tay mới qua `wss://<relay>/p/<id>?pt=`. Hỏng sau msg 3 → không thử relay (desktop có thể đang hỏi duyệt / đã tiêu mã). Fallback sau `mkey` lệch an toàn vì XXpsk3 chỉ trộn PSK ở msg 3 mà điện thoại chưa gửi. Không `pt`: y như v1.
- Sau `pair.ok` lưu `relay` + `relayToken` như cũ; `lastHost` = LAN đã dùng hoặc LAN đầu trong QR.
- UI: `PairRouteRow` (Wi-Fi / mây + chữ `pairViaLan` / `pairViaRelay`, vi + en ARB), hiện khi đang ghép ở cả 2 tab.

### Desktop UI (`apps/web`)
- `RemotePairPanel.vue`: dòng "Có relay: điện thoại khác mạng vẫn ghép được" / "Chỉ LAN: điện thoại phải cùng Wi-Fi" (icon mây / Wi-Fi), đếm ngược từ `exp` (2:00), pha `confirming` khi status `pairing` về null (đang duyệt trên máy — trước đây đồng hồ có thể chạy hết trong lúc hộp duyệt đang mở), pha `tries` khi `pairEnded:"tries"` (có nút Mã mới). Text vi + en trong `i18n/remote-v97.ts`. `REMOTE_PAIR_MS` 2 phút.

### wasm + spike
- `core-remote-wasm`: chỉ **thêm** — `decodeQr` giữ `host`/`port` (= LAN đầu) và thêm `lan[]`, `pt`; `Pairing.fromQrOn(identity, qr, deviceId, deviceName, platform, now)`. `fromQr` / `fromParts` giữ nguyên chữ ký.
- Spike: peer phát QR v2 có `pt`, gửi `pair.open`, conn ghép chỉ XX, đóng 1000 sau `pair.ok`; trang ghép bằng `?pt=` (bỏ token mồi `pair-bootstrap`), kiểm vé dùng lại / vé sai → `4401`.

## File + symbol chính
- Proto: `src/pair/qr.rs` (`QrPayload {lan, ticket}`, `LanAddr`, `PairTicket`, `QR_VERSION=2`, `QR_V1`, `MAX_LAN`, `DEFAULT_LAN_PORT`), `src/pair/mod.rs` (`start_on`, `valid_platform`, `PairedDevice.platform`), `src/relay.rs` (`Control`, `encode_pair_open/close`), `src/vectors.rs` + `tests/vectors.json` (thêm `qrV2`, `qrV2Lan`, `qrV2Relay`, `ticket`, `ticketHash`; `pair.qr` giữ chuỗi v1), `tests/{pair,relay}.rs`.
- Relay: `src/pair.ts` (mới), `src/room.ts` (`connectPairing`, `acceptPhone`, `pairOpen`, `pairClose`, `desktopControl`), `src/index.ts` (`socketGet`), `src/envelope.ts` (`OpenInfo`), `src/close.ts` (`CLOSE_NORMAL`), `test/pair.test.ts` (mới), `test/helpers.ts`, `test/forward.test.ts`, `README.md`.
- Shell: `remote/{pair,session,relay,link,lan,ctx,status,text,host,mod}.rs`, tests `pair_tests.rs`, `pair_relay_tests.rs` (mới), `relay_tests.rs`, `liveness_tests.rs`, `unit_tests.rs`, `relay_setup_tests2.rs`, `test_util.rs`; `resources/relay/{worker.js,worker.json}` sinh lại (không web, hash `0a76fb78ff9faabe`).
- wasm: `core-remote-wasm/src/{lib,handshake}.rs`. Spike: `scripts/spike-v98/wasm/{peer/src/{main,conn}.rs, web/{spike,vectors}.mjs, run.mjs}`.
- Mobile: `lib/remote/{pair_routes (mới), pairing, crypto, crypto_rust, connection_config, remote_host, providers}.dart`, `lib/app/{pair_page, pair_route_row (mới)}.dart`, `lib/l10n/app_{en,vi}.arb` (+ generated), `rust/src/api/remote.rs`, `rust/tests/common/mod.rs`, tests `pair_routes_test.dart` (mới), `pairing_test.dart`, `pair_page_test.dart`, `integration_test.dart`, `wp29_shots_test.dart`, `support/fakes.dart`.
- Web / shared: `components/settings/remote/RemotePairPanel.vue` + `.test.ts` (mới), `i18n/remote-v97.ts`, `lib/remote.ts`, `packages/shared/src/remote.ts` (`RemotePairStart.relay/lan`, `RemoteStatus.pairing/pairEnded`).
- Docs: `docs/remote-integration.md` §1 (bảng control), §2 (QR v2, chọn đường, vé, hộp duyệt, 2 phút, sai 5 lần), §6 (`1000`, `4401` vé, `4429`), §7 (close `1000`, route `?pt=`, relay cũ); `docs/orrery-remote.md` §2; `.code_index/{shell,core,web,protocol,features,_meta}.md`, `apps/mobile/.code_index/mobile.md`. Số đo E2E: `docs/qa/v98/WP-52/{spike-results,wasm-sizes}.json` (file WP-50a giữ nguyên).

## Test chạy thật
| Lệnh | Kết quả |
|---|---|
| `cargo test -p core-remote-proto` (apps/core) | **46 pass / 0 fail** (trước 40; +4 pair, +2 relay); vector cũ không đổi byte |
| `cargo clippy -p core-remote-proto --all-targets --all-features -D warnings`; `cargo clippy -p core-remote-wasm --target wasm32-unknown-unknown --all-features -D warnings` | sạch |
| `cargo test -p core-remote-wasm` | 0 test (crate không có test Rust; vector chạy qua Node / trình duyệt) |
| `cd apps/relay && npm test` / `npm run typecheck` | **55 pass / 0 fail** (6 file, +10 test `pair.test.ts`) / sạch |
| `cd apps/shell && cargo test -j 4 --bin orrery-shell remote::` | **148 pass / 0 fail** (+6), chạy lại sau khi sinh bundle: 148 / 0 |
| `cd apps/mobile/rust && cargo test` | 13 + 2 pass / 0 fail |
| `node apps/mobile/scripts/check.mjs` | gen-l10n OK, format OK, **analyze sạch**, **276 pass / 0 fail** (266 + 10), ~63 skip như trước |
| `check.mjs --test=test/integration_test.dart` | 4 / 4 (thêm: QR gateway là v2; QR v1 đổi từ cùng mã ghép LAN thật bằng crypto Rust) |
| `npx vitest run remote` (apps/web) | **77 pass / 0 fail** (13 file, +3 `RemotePairPanel.test.ts`); `npm run typecheck -w @ccm/web` + `-w @ccm/shared` sạch |
| `node scripts/spike-v98/wasm/build.mjs` → `node vectors-node.mjs` | build OK; **33 / 33** vector (28 cũ + 5 QR v2) |
| `node scripts/spike-v98/wasm/run.mjs` (wrangler dev local 127.0.0.1:8787 với code relay mới + peer Rust + Playwright) | **WebKit 27.2: 14 / 14**, Chromium 157: 14 / 14, vector trong trang 33 / 33. Có: ghép bằng `?pt=`, desktop đóng conn ghép `1000 paired`, vé dùng lại `4401 bad ticket`, vé sai `4401 bad ticket`, IK + echo + 20 × 64 KiB + ping, key lạ `4401` |

wasm prod sau wasm-bindgen: 231.7 KiB thô / 87.0 KiB gzip (WP-50a: 222.9 / 83.9; thêm parse QR v2 + `fromQrOn`).

## Việc dở / chưa kiểm
- **Android thật**: chưa (không có máy). Đã kiểm bằng socket giả (`pair_routes_test.dart`) + integration với gateway Rust thật qua LAN localhost. Chưa kiểm: mDNS thật song song LAN 3 s, 4G thật → relay, camera quét QR 410 ký tự.
- **E2E dev app Tauri + `wrangler dev`**: không dựng — build Tauri đầy đủ nặng và agent WP-54 đang chạy cargo cùng lúc; phía desktop đã phủ bằng `pair_relay_tests.rs` (gateway thật + relay giả nói đúng envelope / control), phía relay + WebKit bằng spike. Lần chạy cargo shell cuối có trùng 1 build cargo của agent khác (tôi kiểm `tasklist` sau khi đã chạy) — xong bình thường.
- Relay của chủ nhân phải "Cập nhật relay" thì mới ghép qua relay được; relay cũ trả `4401` cho `?pt=` (phone báo lỗi mạng sau khi LAN hỏng); UI desktop không biết relay cũ hay mới nên vẫn ghi "Có relay".
- `cargo clippy` cho `apps/shell` chưa chạy.
- `apps/shell/permissions/autogenerated/*.toml` bị build sửa (chỉ xuống dòng, không đổi nội dung).

## Rủi ro bảo mật (cho review opus độc lập)
1. `pt` nằm trong query URL `/p/?pt=` → có thể vào log edge Cloudflare nếu chủ nhân bật Logpush / observability (wrangler đang tắt). Vé dùng 1 lần, ≤ 2 phút, relay chỉ lưu hash; có vé mà không có `code` thì không ghép được (PSK cần `code`), chỉ "đốt" được vé (từ chối dịch vụ ghép relay).
2. Ai có ảnh QR thì có cả `code` + `pt` → có thể ghép trước chủ nhân; chặn cuối là hộp duyệt (hiện IP / quốc gia / đường). Chủ nhân bấm Cho phép theo thói quen là rủi ro chính; cảnh báo "thiết bị mới" là WP-59.
3. IP / quốc gia tin vào relay (relay của chủ nhân, Worker ghi đè header). Ở `wrangler dev` client tự đặt `cf-connecting-ip` được — chỉ ảnh hưởng dev.
4. Desktop tin cờ `pairing` của relay chỉ để **siết** (conn ghép không được IK), không để cấp quyền; ghép vẫn cần mã + duyệt. Conn token vẫn được XXpsk3 (thiết bị đã ghép ghép lại) — có thể siết thêm nếu reviewer muốn.
5. Đếm "sai 5 lần huỷ QR" là theo QR, không theo IP: kẻ trong LAN (hoặc có token relay) gửi 5 msg3 rác là huỷ được QR đang hiện → chỉ phiền (bấm Mã mới), không lộ gì. LAN vẫn 5 / phút / IP.
6. `Strikes` của relay ở RAM DO: mất khi DO bị evict / hibernate, IPv6 đổi địa chỉ né được; brute force vé 128 bit vốn không khả thi, nên đây chỉ là chỗ móc, chưa phải chặn thật (WP-59).
7. Reason `expired` khác `bad ticket` cho biết vé từng đúng — chỉ có ý nghĩa với người đã có vé đúng.
8. Fallback relay sau khi LAN trả `mkey` lệch: an toàn vì msg 3 chưa gửi (PSK chưa dùng); kẻ giả mạo trên LAN chỉ thấy ephemeral.
9. QR v2 lộ mọi IP LAN nội bộ của máy (v1 lộ 1) cho ai thấy QR.
10. Platform trong msg3 do điện thoại tự khai (chỉ để hiển thị); desktop lọc `[A-Za-z0-9]{1,16}`, ip `[0-9a-fA-F:.]`, quốc gia `[A-Z0-9]{2}` trước khi đưa vào hộp duyệt.
11. Ghép qua relay vẫn qua mọi giới hạn cũ: 5 s bắt tay, inbox trước xác thực 4 frame / 64 KB, max 64 phone / room.

## Chỗ móc cho WP-59
- Relay: `apps/relay/src/pair.ts` `Strikes` (`add` / `blocked`, `STRIKE_MAX` 20, `STRIKE_WINDOW_MS` 10 phút) — thay bằng danh sách lưu ở DO + chặn tăng dần; điểm gọi `room.ts connectPairing` (vé sai / hết hạn). Thêm điểm cho token sai (`connectPhone` `verdict` sai) cùng chỗ.
- `open.ip` / `open.country` có trên **mọi** conn relay → `Link.origin` ở desktop (`link.rs`) dùng cho #27 "IP / quốc gia chưa thấy" (so với lịch sử thiết bị trong `devices.rs`) và nhật ký truy cập #24.
- Desktop: `pair.rs run` sau `devices.add(..)` = điểm báo "thiết bị mới ghép" (#27: desktop + Telegram + push tới điện thoại cũ); `Pairing::fail` / `session::try_key` / `Ctx::take_pair_try` = điểm đếm lỗi LAN theo IP (#23); `ctx.notify_unknown` có sẵn cho IK lạ.
- Control mới desktop → relay (vd. `{t:"ban", ip, until}`) đi cùng đường `connId = 0` như `pair.open`, relay đã chỉ đọc control từ socket desktop đã ký.

## Vòng 2 (sau review bảo mật độc lập)

### Đã sửa
1. **[VỪA] Hộp duyệt bị làm giả bằng tên** — `apps/shell/src/remote/text.rs`: `pair_title` đổi thứ tự, phần do gateway / relay đặt đứng trước, tên do điện thoại khai đứng cuối: vi "Ghép qua relay từ IP 203.0.113.7 [VN]: điện thoại android «Pixel 9»?" / en "Pair via relay from IP 203.0.113.7 [VN]: android phone “Pixel 9”?". Hàm mới `shown_name` (chỉ khi hiện, tên lưu không đổi) bỏ `«»“”‘’"'` và backtick, control (xuống dòng), bidi / zero-width (`pair::is_hidden`), gộp khoảng trắng, rỗng → `?`. Platform vẫn phải qua `valid_platform` (chữ số + chữ). Test `pair_tests.rs::a_hostile_phone_name_cannot_fake_the_route_or_ip_in_the_confirm_title` dùng đúng tên `A» từ IP 192.168.1.5 qua LAN? «B` (title bắt đầu bằng đường + IP thật, chỉ còn 1 cặp «») và một tên en có `”`, `"`, `'`, `\n`, U+202E.
2. **[THẤP] Vé đã tiêu được mở lại khi nối lại relay** — chọn cách đơn giản nhất: khi `relay.rs route` nhận `open` có `pairing:true`, gọi `Pairing::spend_relay()` (`pair.rs`): `armed = None` (vòng connection gửi `pair.close`, vô hại vì relay đã xoá) và `slot.relay = false` (status `pairing.relay` thành `false`). QR vẫn hiện và vẫn ghép được qua LAN; muốn ghép qua relay lần nữa phải bấm Mã mới. Lần nối relay sau đọc `armed` = `None` nên không gửi lại hash cũ. Giới hạn biết trước: nếu người dùng tạo QR mới đúng lúc một conn của vé cũ vừa được relay mở (cùng khoảng vài ms), `open` của vé cũ có thể làm desktop coi vé mới là đã tiêu → QR mới chỉ còn LAN (bấm Mã mới lần nữa); không mở thêm quyền nào. Test `pair_relay_tests.rs::a_ticket_the_relay_spent_is_not_armed_again_when_the_desktop_reconnects`: vé chưa dùng vẫn được gửi lại sau khi nối lại; sau `open pairing:true` desktop gửi `pair.close`, status `relay:false`, lần nối lại sau không có `pair.open` trong 1,5 s.
3. **[THẤP] Race 2 conn cùng vé** — `apps/relay/src/room.ts connectPairing`: băm vé (`await ticketHash`) **trước**, rồi mới đọc `pair_h` / `pair_exp`; từ lúc đọc tới `dropMeta` không còn `await` nên trong 1 DO chỉ 1 request đi qua được. Test `pair.test.ts` "two sockets racing with the same ticket: exactly one wins" (3 socket song song, 2 vòng): đúng 1 mở, 2 bị `4401`, desktop nhận đúng 1 `open`. Lưu ý trung thực: chạy thử thứ tự cũ thì test này vẫn pass (pool test của workerd không xen kẽ được các request ở chỗ `await` này), nên test là hàng rào hồi quy, còn chỗ sửa dựa trên đọc code.
4. **Mục 5 (đồng hồ lệch)** — sửa luôn vì rẻ: `pair.open` mang `ttl` (giây còn lại, 1–300) thay vì `exp` tuyệt đối; relay tính `exp = now(relay) + ttl` (`src/pair.ts parsePairOpen`), nên đồng hồ desktop lệch relay không còn làm vé bị bỏ âm thầm. Proto `relay::encode_pair_open(hash, ttl)` + `MAX_PAIR_TTL` (từ chối 0 / > 300); desktop `relay.rs arm_bytes` tính `exp - now` bằng `checked_sub`; peer spike gửi `ttl`. Shape cũ `{h, exp}` bị relay bỏ qua (có test). Hệ quả còn lại: đồng hồ điện thoại lệch vẫn có thể làm QR "hết hạn" phía điện thoại (`exp` trong QR là giờ desktop) — như v1.
5. **Mục 5 (web hiện "đang chờ duyệt" lúc hết hạn)** — `RemotePairPanel.vue`: khi status `pairing` về `null` mà mã đã quá `exp` thì sang pha `expired`, không phải `confirming`. Test mới trong `RemotePairPanel.test.ts`. Còn dở: pha `confirming` không có hạn riêng — nếu chủ nhân bấm Từ chối trên hộp native thì bảng vẫn ghi "Duyệt điện thoại…" cho tới khi đóng / Mã mới (desktop chưa báo `denied` qua status).

### Test chạy lại (vòng 2)
| Lệnh | Kết quả |
|---|---|
| `cargo test -p core-remote-proto` | 46 pass / 0 fail; clippy `--all-targets --all-features -D warnings` sạch |
| `cd apps/relay && npm test` / `npm run typecheck` | **56 pass / 0 fail** (+1 test race, test hạn đổi sang `ttl`) / sạch |
| `cd apps/shell && cargo test -j 4 --bin orrery-shell remote::` | **150 pass / 0 fail** (+2: tên độc, vé đã tiêu) |
| `cargo clippy -j 4 --bin orrery-shell --tests` (apps/shell, lần đầu chạy) | 0 error; 0 warning ở file WP-52 đụng (`pair`, `pair_tests`, `pair_relay_tests`, `relay`, `session`, `text`, `link`, `ctx`, `status`); 21 warning có sẵn ở file khác (`relay_setup_tests.rs` import thừa, `keys.rs`, `viewer.rs`, `bug_report.rs`, `preview_*`, `mod.rs::call`, `devices.rs::set_push`, `fcm.rs::set_http`, `vault.rs`, `host_extra.rs`, core-desk) |
| `npx vitest run remote` (apps/web) + `npm run typecheck` | **78 pass / 0 fail** / sạch |
| `node scripts/spike-v98/wasm/build.mjs` + `vectors-node.mjs` + `run.mjs` | vector 33 / 33; **WebKit 27.2 14 / 14**, Chromium 157 14 / 14 (ghép bằng vé `ttl`, close 1000, vé dùng lại / sai → 4401) |
| `node scripts/relay-bundle.mjs` (không web) | bundle mới `e2f923f56cfbd048` |

Lưu ý: 2 lần chạy cargo (spike build, clippy) có trùng lúc agent khác đang chạy cargo (thấy trong `tasklist` ngay trước đó); lần `cargo test` shell cuối đợi tới khi không còn cargo nào mới chạy.

### Thêm vào "Chỗ móc cho WP-59" (theo review)
- Huỷ QR bằng 5 msg3 rác (`Pairing::fail` đếm theo QR, không theo IP) → WP-59 nên đếm thêm theo IP và chỉ huỷ QR khi lỗi đến từ nhiều IP / hoặc khoá IP thay vì huỷ.
- `session::try_key` đổi sang `relay:<ip>` (5 / phút / IP) đã bỏ trần chung 20 / phút cho cả relay khi relay báo IP → WP-59 thêm lại trần chung (hoặc chặn ở relay) để nhiều IP không cộng dồn được.
