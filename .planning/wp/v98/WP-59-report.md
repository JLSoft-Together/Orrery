# WP-59 — báo cáo: chặn IP tăng dần (relay + LAN), danh sách / gỡ / chặn tay / tin cậy / nhật ký, siết relay, cảnh báo

**Kết luận: XONG code + test trên máy này.** Chưa deploy relay thật, chưa kiểm trên điện thoại thật (không có máy), chưa E2E với app Tauri thật + `wrangler dev` (phía relay đã chạy thật qua spike `run.mjs`, phía desktop qua relay giả trong `pair_relay_tests.rs`).

## Thiết kế đã chốt + lý do

### Luật chung (LAN desktop = relay)
- Khoá đếm / chặn: IPv4 nguyên địa chỉ (IPv4-mapped → IPv4), **IPv6 theo /64** — một máy IPv6 thường có cả /64, đếm theo địa chỉ thì đổi địa chỉ là né. Đánh đổi: người khác cùng /64 với kẻ dò cũng bị chặn ghép (token hợp lệ vẫn qua, xem dưới).
- 20 lỗi / 10 phút → 1 giờ; bị chặn lại trong 7 ngày sau `until` → 24 giờ → 7 ngày (giữ bậc 3); 7 ngày sạch → về 1 giờ. Đang chặn thì không cộng.
- Sự kiện lỗi. LAN: bắt tay Noise sai, quá 5 s, IK thiết bị lạ, msg 3 sai mã, ghép khi không có QR, không hello, `4429` của `Gate` / trần thử ghép, nguồn đã bị chặn với QR (`tries`). Relay: token sai / hết hạn / thu hồi, vé sai / hết hạn, text frame, frame > 65 535 B, frame đầu > 5 s, quá 4 socket / IP, `429` Worker, và `strike` desktop gửi. **Không tính** proto cũ (`4426`) — app cũ hợp lệ.
- Không bao giờ tự chặn: IP trong danh sách tin cậy; IP có phiên hợp lệ đang mở hoặc trong 10 phút qua (LAN: phiên IK đã `hello`; relay: socket đã qua token HMAC). Thêm: trên relay **token hợp lệ đi qua lệnh chặn tự động** — thiết bị đã ghép dùng chung CGNAT / 4G với kẻ dò không bị khoá ngoài; chặn tay vẫn áp cho mọi người (quyết định rõ của chủ nhân).
- Chặn chỉ áp cho `/p/` và cổng LAN; `/m/` của desktop không bao giờ bị chặn. Gỡ chặn là thao tác ngay trên desktop (IPC cục bộ), không cần điện thoại nối được → "luôn có đường gỡ".
- CIDR (chặn tay / tin cậy): IPv4 ≥ /8, IPv6 ≥ /16, max 32 mỗi loại; desktop từ chối chặn tay dải đang có phiên sống (`live`) để chủ nhân không tự khoá phiên đang dùng.

### Relay (`apps/relay/src/{cidr,guard,guard-control}.ts`, `room.ts`, `index.ts`)
- Bền: bảng SQLite `guard_block (k, level, until, at, n, why)` + `guard_rule (trust|manual)`, tạo trong constructor (`IF NOT EXISTS`, không cần migration). Max 1024 dòng; dọn dòng quá `until + 7 ngày`, rồi dòng hết sớm nhất. Bộ đếm 10 phút ở **RAM** (1024 khoá): ghi SQLite mỗi lỗi sẽ cho kẻ dò đốt quota ghi dòng / ngày của Free plan; chỉ ghi khi tạo lệnh chặn (≤ 1 / IP / kỳ chặn). Mất bộ đếm khi DO evict là chấp nhận được, lệnh chặn + bậc không mất (test `new Room(state, env)` trên cùng storage).
- 4 socket phone / IP: tag `ip:<key>` khi accept, đếm socket chưa `closing` (tin cậy không giới hạn) → `4429 too many` + lỗi.
- 5 s: **không dùng DO alarm** (mỗi alarm là 1 request tính tiền). Relay kiểm khi frame đầu tới (`att.at`, `att.fed`) → `4400 timeout` + lỗi; socket im lặng bị desktop cắt ở 5 s (deadline bắt tay có sẵn → control `close 4400` + `strike timeout`); desktop vắng thì phone bị `4503` như cũ.
- `429` Worker: `index.ts noteRate` gọi DO `POST /strike` nội bộ, max 1 lần / phút / IP / isolate (Map 1024) để 429 không biến thành 1 request DO mỗi lần. Đường `/strike` chỉ tới được từ Worker (adminPost chỉ chuyển `/register`, `/revoke`).
- Khi tạo lệnh chặn: notice `{t:"blocked", row}` tới desktop + đóng mọi socket chưa-token của IP đó `4429 blocked`.
- Control desktop: giới hạn đọc control nâng 256 → 4096 B (chỉ socket desktop đã ký). `strike {conn, why}` dùng IP trong **attachment của conn** (field `ip` bị bỏ qua) → desktop không bị lừa chặn IP tuỳ ý. `block.list` → notice `block.list {items ≤ 200, total}`; `block.remove {c|all}` gỡ dòng chồng lấn + bộ đếm rồi trả list; `block.set {list:[{c, ttl?}]}` **thay toàn bộ** chặn tay (thay vì `block.add` lẻ: hội tụ khi desktop đổi lúc relay offline; `ttl` tương đối như `pair.open`) và đóng socket trong dải; `trust.set` thay danh sách + xoá dòng chồng lấn.

### Desktop (`apps/shell/src/remote/`)
- `guard.rs` `Guard` trong `Ctx`: file `<remote dir>/guard.json` (trust, manual, dòng LAN; max 256 dòng LAN), RAM: bộ đếm, `good`, mirror relay, nhật ký 200, hàng `block.remove` chờ, cờ `dirty`. `admits` gọi đầu `lan::accept_loop` (TCP bị đóng trước bắt tay WS). Đồng bộ lên relay mỗi lần nối (`trust.set`, `block.set`, removes, `block.list`) và ngay khi đổi (`wake` → `take_ops`).
- `Out::Strike(why)` mới trong `link.rs`: `Ctx::refuse` = ghi nhật ký + (LAN: đếm cục bộ | relay: gửi `strike` trước `close`). Các điểm `return link.out.close(...)` lỗi ở `session.rs` / `pair.rs` đổi sang `refuse`.
- Review WP-52 mục 3:
  - **Huỷ QR**: đổi sang luật theo nguồn — 5 lỗi msg 3 từ 1 nguồn (`lan:<key>` / `relay:<key>`) → nguồn đó `4429 tries` với QR này, QR vẫn dùng được cho nguồn khác; **20** lỗi tổng / QR → huỷ (`pairEnded:"tries"`). Không chọn "chỉ huỷ khi lỗi từ conn đã qua vé" vì LAN không có bước vé (sẽ thành không bao giờ huỷ trên LAN). Mã 16 B không đoán được, trần tổng chỉ chặn đoán phân tán.
  - **Trần chung relay**: `try_keys` = `[relay:<key> 5/phút, relay 20/phút]`; `take_pair_try` kiểm tất cả rồi mới lấy slot (không tiêu slot khoá đầu khi khoá sau đã đầy).
- Nhật ký 200 dòng RAM `{at, ip, via, country?, device?, result, why}` — `why` là chuỗi tĩnh, `device` qua `shown_name`; không mã / vé / token / key / payload (test `pair_tests` kiểm state không chứa mã base64). Không ghi TCP bị bỏ do IP đang chặn (tránh đẩy hết 200 dòng). Không lưu đĩa (giữ đơn giản, đổi ở WP sau nếu cần).
- `guardRev` trong status: tăng khi danh sách / nhật ký đổi; `ctx.changed()` từ nhật ký max 1 lần / giây.
- Cảnh báo `alerts.rs` `Ctx::raise` = `Host::notify` + `Host::telegram` (mới, mặc định no-op; `AppHost` gọi `desk:tg-alert {kind:"remote", detail:{text}}` trên thread riêng; core-desk thêm nhánh `remote`: lọc control, 600 ký tự, escape HTML, gửi mọi chat có route khi Telegram bật + bật cảnh báo) + `push_security(except)` (FCM `k:"security"`, không `p`, title / body cố định không IP / tên, bỏ qua giới hạn pane, gửi cả máy đang nối; app Android không có `p` thì không mở link — đã kiểm `pushLink`).
  - Thiết bị mới ghép: sau `pair.ok` (`Ctx::paired`).
  - Có IP bị chặn (LAN hoặc notice relay): max 1 / giờ, số lần bị gộp đếm vào lần sau ("và N IP khác"). Không có timer gửi bù (lần gộp cuối chỉ hiện ở lần cảnh báo kế tiếp và trong danh sách).
  - Nơi lạ: `Device.nets` (/24 · /48, max 16) + `countries` (max 8) trong `devices.json`; tập rỗng chỉ ghi nhớ (thiết bị cũ không báo nhầm lần đầu); **chỉ phiên relay báo** — IP LAN nội bộ đổi theo DHCP sẽ báo nhảm; max 1 / thiết bị / giờ. So theo mạng thay vì từng IP để 4G đổi IP trong cùng dải không báo liên tục (4G nhảy dải vẫn báo — xem rủi ro).

### UI (`apps/web/src/components/settings/remote/`)
- `RemoteGuardBlocked.vue` (IP / dải, chip nơi chặn icon + chữ, lý do, số lỗi, lúc chặn, hết hạn / Vĩnh viễn, bậc; Gỡ; Gỡ hết hỏi lại inline; form chặn tay IP / CIDR + 1 giờ / 1 ngày / 7 ngày / 30 ngày / vĩnh viễn, lỗi tại chỗ), `RemoteGuardTrust.vue`, `RemoteAccessLog.vue` (kết quả = icon + chữ + màu). Dạng danh sách 2 dòng thay vì bảng 6 cột để không cuộn ngang ở cửa sổ hẹp. State `composables/useRemoteGuard.ts` (tải khi mở + khi `guardRev` đổi), IPC 1 kênh `ccm:remote-guard {op}` (preload `remoteGuard`, shim sinh lại). i18n `remote-v98.ts` vi + en, mục tìm kiếm Settings mới. Màu chỉ dùng `success` / `warning` / ink (không màu trạng thái lifecycle), theo COLOR-SYSTEM.
- Skill: đã nạp `ui-ux-pro-max:ui-ux-pro-max` (script search cần Python, máy không có → theo design system sẵn của repo); skill `frontend-design:frontend-design` **không có** trong danh sách skill của phiên này.

## File + symbol
- Proto `core-remote-proto/src/relay.rs`: `encode_strike`, `encode_block_list`, `encode_block_remove`, `encode_block_set` (`BlockRule`), `encode_trust_set`, `valid_rule_text`, `decode_notice` → `Notice::{Blocked, BlockList}`, `BlockRow`, `STRIKE_REASONS`, `MAX_GUARD_CONTROL` 4096, `MAX_NOTICE` 32 KiB, `MAX_GUARD_RULES` 32; test `tests/relay.rs` (+2).
- Relay: `src/cidr.ts` (`parseIp`, `parseCidr`, `inCidr`, `cidrText`, `ipKey`), `src/guard.ts` (`Guard`, `parseRules`, `parseManual`, hằng), `src/guard-control.ts` (`guardControl`), `src/room.ts` (`full`, `strike`, `strikeConn`, `sendList`, `closeMatching`, `rateStrike`, `liveGood`, `Attachment.{key,ip,at,fed,good}`, `MAX_PER_IP`, `FIRST_FRAME_MS`), `src/index.ts` (`noteRate`), `src/envelope.ts` (`encodeNotice`, `GUARD_CONTROL_MAX`), `src/pair.ts` (bỏ `Strikes`); test `test/guard.test.ts` (mới), `test/pair.test.ts`; README.
- Shell: `remote/guard.rs`, `guard_ip.rs`, `guard_hooks.rs`, `guard_ops.rs`, `alerts.rs` (mới); `ctx.rs` (`guard`, `Host::telegram`, `take_pair_try(&[..])`, `guardRev`), `session.rs` (`try_keys`, `refuse`), `pair.rs` (`Pairing::{fail(ticket, source), barred}`, `source`, `MAX_QR_FAILS`), `lan.rs`, `link.rs` (`Out::Strike`), `relay.rs` (sync, notice, `encode_strike`), `devices.rs` (`nets`, `countries`, `note_place`), `push.rs` (`push_security`, kind `security`), `host.rs` (`telegram`), `mod.rs` (kênh `ccm:remote-guard`), `ccm.rs` LOCAL; test `guard_tests.rs` (mới), `pair_relay_tests.rs`, `relay_tests.rs`, `pair_tests.rs`, `test_util.rs`; `resources/relay/{worker.js,worker.json}` sinh lại.
- core-desk: `telegram/{episodes,text_act,tests}.rs` (`remote_alert`, nhánh `kind == "remote"`).
- Shared / web / desktop: `packages/shared/src/remote.ts` (`RemoteGuard*`, `RemoteStatus.guardRev`), `apps/desktop/src/preload.ts` (`remoteGuard`) → `apps/shell/src/shim.js` (sinh bằng `scripts/gen-desk-shim.mjs`), `apps/web/src/{lib/desktop.ts, lib/remoteGuard.ts, composables/useRemoteGuard.ts, i18n/remote-v98.ts, i18n/{en,vi}.ts, components/settings/{SettingsRemote.vue,index.ts}, components/settings/remote/{RemoteGuardBlocked,RemoteGuardTrust,RemoteAccessLog}.vue, RemoteGuard.test.ts}`.
- Docs: `docs/remote-integration.md` §1 (bảng control), §2 (luật ghép sai mới), §5.1 / §5.2 (mới), §6, §7; `docs/orrery-remote.md` §2, §8, §12 (mục bảo mật cho người dùng; sửa luôn "Mã QR 5 phút" cũ → 2 phút); `.code_index/{shell,core,web,protocol,_meta}.md`; `docs/qa/v98/WP-59/` (ảnh + `shots.mjs` + `spike-results.json`).

## Test chạy thật
| Lệnh | Kết quả |
|---|---|
| `cargo test -j 4 -p core-remote-proto` (apps/core) | **48 pass / 0 fail** (trước 46) |
| `cargo clippy -p core-remote-proto --all-targets --all-features -- -D warnings` | sạch |
| `cargo check -p core-remote-wasm --target wasm32-unknown-unknown` | OK |
| `cargo test -j 4 -p core-desk --lib telegram::` | **35 pass / 0 fail** (+1 `remote_alerts_post_escaped_text_to_every_routed_chat`) |
| `cd apps/relay && npm test` / `npm run typecheck` | **63 pass / 0 fail** (7 file; trước 56, +7 `guard.test.ts`) / sạch |
| `cd apps/shell && cargo test -j 4 --bin orrery-shell remote::` | **164 pass / 0 fail** (trước 150; chạy lại sau khi sinh bundle: 164 / 0) |
| `cargo test -j 4 --bin orrery-shell` (toàn bộ) | **358 pass / 0 fail** |
| `cargo clippy -j 4 --bin orrery-shell --tests` | 0 warning ở file WP-59 đụng; warning còn lại có sẵn (`relay_setup_tests.rs` import thừa, `mod.rs::call`, `devices.rs::set_push`, `keys.rs`, `fcm.rs`, `viewer.rs`, `vault.rs`, `preview_*`, `bug_report.rs`, `host_extra.rs`, core-desk `space.rs`, `moves.rs`) |
| `cd apps/web && npx vitest run remote` | **85 pass / 0 fail** (14 file; +7 `RemoteGuard.test.ts`) |
| `npx vitest run src/i18n src/components/settings` | 44 / 0 (gồm i18n compile mọi message vi + en) |
| `npm run typecheck -w @ccm/web` / `-w @ccm/shared` | sạch |
| `node scripts/spike-v98/wasm/run.mjs` (wrangler dev 127.0.0.1:8787 với relay mới + peer Rust + Playwright) | vector node 33 / 33; **WebKit 27.2: 14 / 14**, Chromium 157: 14 / 14 (số đo `docs/qa/v98/WP-59/spike-results.json`; `docs/qa/v98/WP-50a/results.json` bị script ghi đè đã trả lại) |
| `node scripts/relay-bundle.mjs` (với `CCM_PWA_DIST` = đường không tồn tại) | bundle **`055bd4866c1c7704`**, 48 713 B, không web |

Bundle: `apps/pwa/dist` hiện có (WP-55a đang dựng) nên chạy mặc định sẽ nhúng web dở vào bundle commit; tôi trỏ `CCM_PWA_DIST` ra đường không tồn tại để giữ bundle không web như WP-52 (`web-assets.generated.ts` không đổi).

Ảnh UI (Vite dev cổng 5199 + `window.ccmDesktop` giả, Chrome headless, vi): `docs/qa/v98/WP-59/remote{Blocked,Trusted,Access}-{light,dark}.png` (light = `orrery-light`, dark = `galaxy`); dựng lại: `node docs/qa/v98/WP-59/shots.mjs`.

Mọi cargo chạy khi `tasklist` không có cargo khác; không đụng cổng 4320–4322; tiến trình nền (wrangler, Vite, Chrome) đã tắt.

## Việc dở / chưa kiểm
- Chưa kiểm: relay thật trên Cloudflare (hibernation thật, IP thật từ `cf-connecting-ip`, `LIMITER` 429 thật → `/strike`), Android / iPhone thật, E2E app Tauri thật.
- Relay của chủ nhân phải "Cập nhật relay" (hash mới) thì mới có chặn / 4 socket / IP ở relay; relay cũ bỏ qua control mới (LAN vẫn chặn).
- Nhật ký truy cập và hàng `block.remove` chờ chỉ ở RAM (mất khi tắt app); mirror danh sách relay chỉ có sau khi relay nối.
- Cảnh báo "IP bị chặn" gộp không có timer gửi bù.
- Web Push (WP-56) chưa có: chỗ móc là `Ctx::raise` (`remote/alerts.rs`) — thêm 1 dòng gọi kênh Web Push với cùng `AlertText` hoặc chỉ kind `security`.

## Rủi ro bảo mật (cho review opus độc lập)
1. **Tự khoá trên LAN**: điện thoại của chủ nhân giữ cặp khoá cũ (desktop đổi key / thiết bị bị thu hồi) và tự nối lại liên tục → IK `device` mỗi lần → IP LAN bị chặn 1 giờ → ghép lại từ máy đó bị TCP từ chối. Đường gỡ: UI desktop (Gỡ / Gỡ hết) hoặc thêm mạng nhà vào Tin cậy. Không miễn trừ IP private mặc định (spec yêu cầu cổng LAN cùng luật).
2. Token hợp lệ đi qua chặn tự động ở relay: token bị lộ thì kẻ có token không bị chặn tự động (vẫn cần khoá Noise của thiết bị để làm gì); thu hồi thiết bị hoặc chặn tay vẫn hiệu lực.
3. IPv6 /64: kẻ dò cùng /64 với chủ nhân làm chủ nhân bị chặn ghép relay từ /64 đó (phiên token vẫn qua). Kẻ có nhiều /64 (thuê VPS) vẫn né được — chặn chỉ giảm nhiễu, an toàn thật dựa vào mã 128 bit + vé + hộp duyệt.
4. Bộ đếm relay trong RAM: DO evict giữa chừng làm mất đếm (không mất lệnh chặn). Isolate Worker nhiều nơi → `noteRate` mỗi isolate 1 / phút.
5. `strike` desktop chỉ theo `conn`; nhưng desktop vẫn là chủ room — desktop bị chiếm thì chặn được IP bất kỳ đang nối (vô nghĩa khi desktop đã bị chiếm).
6. Danh sách 4 socket / IP tính cả socket token: một văn phòng NAT > 4 điện thoại đã ghép phải thêm vào Tin cậy.
7. IP / quốc gia tin relay (Worker đặt header nội bộ); `wrangler dev` client tự đặt được — chỉ ảnh hưởng dev.
8. Telegram nhận IP + tên thiết bị (chat của chủ nhân, qua Telegram server); FCM chỉ nhận chữ cố định. Nhật ký giữ IP + tên thiết bị trong RAM, `devices.json` giữ dải /24 · /48 + quốc gia của điện thoại (cục bộ).
9. Cảnh báo nơi lạ chỉ cho relay, so theo /24 · /48: 4G nhảy dải sẽ báo (max 1 / giờ / thiết bị); kẻ trong cùng /24 với chủ nhân không gây cảnh báo.
10. `block.set` rỗng từ desktop xoá mọi chặn tay trên relay — đúng ý (desktop là nguồn sự thật), nhưng nếu `guard.json` hỏng / bị xoá, lần nối sau relay mất chặn tay (chặn tự động giữ nguyên).
11. Chặn tay không thể bao dải có phiên sống, nhưng có thể bao IP của điện thoại đang offline — chủ nhân tự khoá điện thoại đó qua relay (gỡ được từ desktop).
12. Đọc control desktop nâng lên 4096 B (chỉ từ socket `/m/` đã ký Ed25519, socket bị thay bị bỏ qua).

## Chỗ móc cho WP-56 (Web Push)
- `apps/shell/src/remote/alerts.rs` `Ctx::raise(text, except)`: thêm gửi Web Push cạnh `push_security(except)` (cùng luật: không IP / tên trong payload, trừ thiết bị `except`).
- Kind push `security` (`push.rs title_body`) và `RemotePushKind` phía web app có thể thêm `security` (không có pane, chạm mở danh sách máy).

## Vòng 2 (sau review bảo mật độc lập: SỬA, 0 chặn)

Luật huỷ QR theo nguồn (5 sai / nguồn loại nguồn, 20 tổng huỷ QR) được chủ nhân duyệt, giữ nguyên.

### Đã sửa
1. **[VỪA] Trần chung `Gate` đầy làm chặn IP chủ nhân** — `limits.rs` `Gate::admit` → `Result<Pending, Refused::{Total, Ip}>` (kiểm trần riêng IP trước, trần chung 64 sau; slot mở / phút được trả lại khi bị từ chối vì trần chung). `lan.rs accept_loop` chỉ `strike_lan` khi `Refused::Ip`. Test `guard_tests::a_full_shared_pending_pool_never_counts_as_a_failure_for_the_next_ip`, `unit_tests::the_gate_counts_pending_connections_per_ip`.
2. **[VỪA] Strike mất khi phone đóng ngay** — desktop `relay.rs` writer gửi `Out::Strike` cả khi conn đã đóng (chỉ bỏ khi `conn` đã có entry gen khác = mở lại cho phone khác; `Conns::has`). Relay `room.ts`: `markClosing` ghi `recent` (conn → `{key, ip, at}`, max 64, dùng trong 30 s, RAM); `strikeConn` dùng socket còn sống, không có thì `recent`. Test relay "a strike for a phone that just left still counts…", shell `pair_relay_tests::a_strike_still_reaches_the_relay_when_the_phone_left_…`. Lưu ý trung thực: cả hai test có thể pass với code cũ nếu thứ tự xử lý rơi đúng (socket đã đóng còn trong `getWebSockets`, task phiên chạy trước control close) — chỗ sửa dựa trên đọc code, test là hàng rào hồi quy.
3. **[VỪA] `4429` không tới relay** — desktop chuyển `CLOSE_RATE` trong `encode_close`; relay `DESKTOP_CLOSE` thêm `[4429, "rate"]`. Test relay (phone nhận `4429 rate`), shell (close `4429` cho conn bị chặn tay ở mục 5).
4. **[THẤP–VỪA] Token lộ được miễn chặn** — relay không còn `markGood` / `good` lúc nhận token. Desktop gửi control mới `{t:"good", conn}` (`Out::Good` sau `admit_live` trong `session::serve`, proto `encode_good`); relay `markConnGood` đặt `att.good` + `Guard.markGood(ipk, deviceId)`. Token hợp lệ chỉ đi qua chặn tự động khi IP được bảo lãnh trong 10 phút hoặc deviceId trong 24 giờ (`Guard.vouched`, RAM). Hướng an toàn khi lệch bản: desktop cũ không gửi `good` → không ai được miễn trên relay (chặn nhiều hơn, gỡ từ desktop); relay cũ bỏ qua `good`. Test relay "…a token alone does not pass a block" (token chưa bảo lãnh bị đóng `4429 blocked` cùng IP đó; thiết bị đã bảo lãnh qua được), shell `a_paired_phone_on_the_relay_…` kiểm `good` chỉ tới sau hello.
5. **[THẤP–VỪA] Relay cũ / tự host bỏ qua chặn tay** — `relay.rs route` `Open`: `Guard::manual_blocked(origin.ip)` → không mở phiên, ghi nhật ký `blocked/manual`, đóng `4429 blocked`. Test shell `…manual_blocks_apply_on_an_old_relay` (relay giả không hiểu `block.set`).
6. **[THẤP] Gỡ 1 IP xoá luật rộng hơn** — `Guard::unblock(Some)` chỉ xoá luật chặn tay trùng đúng chuỗi (sau chuẩn hoá `Cidr::text`); dòng tự động chồng lấn vẫn gỡ. Test `unblocking_one_address_leaves_a_wider_manual_rule_in_place`.
7. **[THẤP] Telegram + "mạng mới"** — core-desk không phân biệt được chat riêng của chủ nhân với nhóm (routes là chat / topic dự án) nên `AlertText.tg` riêng: chỉ loại cảnh báo + đường + quốc gia + lời nhắc mở desktop, không IP / tên (desktop notification giữ chi tiết). Cảnh báo nơi lạ **chỉ khi quốc gia mới**: CGNAT di động đổi cả /16, /8 thường xuyên nên /16 vẫn báo nhảm; IP từng lần nối có trong nhật ký. Bỏ `Device.nets` + `guard_ip::net` (không giữ dữ liệu không dùng); `Devices::note_place(id, country) -> bool`. Test texts (không `203.0.113`, không tên trong `tg`), LAN / ghép mới (Telegram không có `127.0.0.1` / tên).
8. Docs §5.1 / §5.2 / §1 / §2 + `orrery-remote.md` §12: huỷ QR vẫn rẻ trên LAN (4 IPv4 × 5 = 20 → chỉ hỏng lượt ghép, bấm Mã mới), miễn trừ theo NAT / IPv6 /64, khoá IPv6 /64, control `good`, chặn tay tự áp ở desktop, `4429` chuyển cho relay, gỡ đúng chuỗi, Telegram không chi tiết. README relay cập nhật.
9. `devices.rs known_platform` thêm `"web"`; shared `RemoteDevice.platform` thêm `'web'`; `RemoteDeviceRow.vue` + `remoteV97.devices.platform.web` (vi / en "Web app"); test `RemoteDeviceRow.test.ts` + `guard_tests`.

### Test chạy lại (vòng 2)
| Lệnh | Kết quả |
|---|---|
| `cargo test -j 4 -p core-remote-proto` + clippy `-D warnings` | **48 pass / 0 fail** (thêm assert `encode_good` vào test có sẵn) / sạch |
| `cd apps/relay && npm test` / `npm run typecheck` | **64 pass / 0 fail** / sạch |
| `node scripts/relay-bundle.mjs` (`CCM_PWA_DIST` = đường không tồn tại) | bundle **`983a54112caa3232`** (50 455 B, không web) |
| `cargo test -j 4 --bin orrery-shell remote::` (sau khi sinh bundle) | **167 pass / 0 fail**; toàn bộ binary **361 / 0** |
| `cargo clippy -j 4 --bin orrery-shell --tests` | không warning mới (duy nhất `devices.rs::set_push` có sẵn ở file đụng) |
| `npx vitest run remote` (apps/web) / `vue-tsc` / `@ccm/shared` typecheck | **86 pass / 0 fail** / sạch / sạch |
| `node scripts/spike-v98/wasm/run.mjs` | vector 33 / 33; **WebKit 27.2: 14 / 14**, Chromium 157: 14 / 14 (số đo `docs/qa/v98/WP-59/spike-results.json`, `WP-50a/results.json` đã trả lại) |

### Rủi ro còn lại (bổ sung)
- Bảo lãnh deviceId 24 giờ nằm trong RAM của DO: DO evict → điện thoại đã ghép ở IP đang bị chặn phải chờ hết chặn hoặc chủ nhân gỡ từ desktop.
- `recent` conn vừa đóng chỉ ở RAM, 30 s; strike tới muộn hơn bị bỏ.
- Miễn trừ theo IP vẫn áp cho cả NAT / /64 của điện thoại đã bảo lãnh trong 10 phút.
