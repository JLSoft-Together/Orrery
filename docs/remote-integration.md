# Orrery Remote — giao thức điện thoại ↔ desktop (proto 1)

Hợp đồng chung cho gateway desktop (`apps/shell/src/remote/`), crate `apps/core/crates/core-remote-proto`,
relay (`apps/relay`) và app `apps/mobile`. Thiết kế + lý do: `.planning/PLAN-v97-mobile.md`.
Đổi file này = tăng `proto` nếu phá tương thích.

## 1. Lớp kết nối

| Đường | Socket | Bên ngoài Noise |
|---|---|---|
| LAN | `ws://<ip LAN>:4320/remote` (mặc định port 4320, `CCM_REMOTE_PORT`) | mỗi WS binary message = 1 Noise message |
| Relay | desktop `wss://<relay>/m/<machineId>`, điện thoại `wss://<relay>/p/<machineId>?t=<token>` | desktop: `[u16 connId BE][noise msg]`; `connId = 0` = control JSON của DO `{t:"open"\|"close", conn}`; điện thoại: không có envelope (DO thêm / bỏ) |

- LAN: không nhận frame nào trong 45 s (kể cả ping WS / kind 3) → gateway đóng phiên; điện thoại gửi ping ≤ 30 s / lần.
- Keepalive relay: text frame `"ping"` → DO tự trả `"pong"` (`setWebSocketAutoResponse`), ngoài Noise, mỗi 30 s.
- Trước khi vào Noise không có dữ liệu ứng dụng nào. Handshake chưa xong trong 5 s → đóng. Frame đầu sau bắt tay IK phải là `hello` giải mã được trong 5 s, chỉ từ đó phiên mới được tính (`lastSeen`, thông báo, max 16 phiên); trước đó inbox max 4 frame / 64 KB.
- mDNS `_orrery._tcp`, TXT `{v, k, n}`: `k` = base64url 8 byte đầu SHA-256(key tĩnh Noise của máy), `n` = tên máy; không có machineId.
- Ghép sai: không huỷ QR; max 5 lần thử / phút / IP (LAN) hoặc / connId (relay) → `4429`.

## 2. Noise

- Prologue: `b"orrery-remote/" ‖ proto (u16 BE)`. Msg 1 của cả 2 bắt tay gửi dạng `proto (u16 BE) ‖ noise msg1`: responder đọc proto trước để dựng prologue, nhờ đó trả được `ProtoTooOld {min}` / `ProtoTooNew {max}` thay vì lỗi giải mã. Payload msg 2 IK = `{proto}`; msg 1 / msg 2 ghép payload rỗng. Vector cố định: `apps/core/crates/core-remote-proto/tests/vectors.json`. Bên nhận gặp lỗi giải mã 1 lần → đóng kết nối.
- **Ghép**: `Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s`, điện thoại initiator. PSK = HKDF-SHA256(ikm = `code` 16 B, salt = `"orrery-pair"`, info = `machineId`). Sau msg 2 điện thoại so key tĩnh desktop với `mkey` trong QR, khác → dừng (không gửi msg 3). Payload msg 3 (JSON): `{deviceId, name, platform}`. Desktop hỏi xác nhận native, lưu thiết bị, gửi frame `pair.ok {machine, name, relay?, relayToken?}` hoặc `pair.denied`.
- **Phiên**: `Noise_IK_25519_ChaChaPoly_BLAKE2s`, điện thoại initiator. Payload msg 1: `{deviceId, proto}`. Desktop kiểm `deviceId` + key tĩnh trong `devices.json`, sai → đóng (`4401`); `proto < minProto` → đóng `4426` ngay (không payload). Gateway phân biệt ghép / phiên theo độ dài msg 1 (ghép luôn 50 B = 2 B proto + 48 B Noise, IK dài hơn). Sau `pair.ok` gateway đóng `1000`, điện thoại nối lại bằng IK. Qua relay desktop chưa đóng được 1 điện thoại: `4401` / `4426` qua relay = ngừng trả lời (cần control close từ desktop ở relay — việc tồn).
- QR (base64url JSON, ≤ 300 B): `{v:1, machine, name, host?, port?, relay?, mkey, code, exp}`; hết hạn 5 phút, dùng 1 lần, chỉ hợp lệ khi Settings đang hiện QR.

## 3. Frame trong Noise

`[u8 kind][u8 flags][u32 stream BE][payload]`, flags bit 0 = `more` (`FLAG_MORE`), bit 1 = `claim` (`FLAG_CLAIM`, `Frame.claim` trong crate `core-remote-proto`; chỉ kind 2, kind khác mang bit này → `BadFrame`; `Message.claim` = có ít nhất 1 khúc mang bit; phím người dùng thật gõ / dán → gateway gửi `claim` cho hub; phím tự trả lời query của terminal không bật bit này). Payload mỗi khúc ≤ 60 KiB, ghép lại max 1 MiB.

| kind | Tên | stream | payload |
|---|---|---|---|
| 0 | json | 0 | UTF-8 JSON (§4) |
| 1 | pty_out | id attach (do điện thoại chọn, u32) | byte terminal |
| 2 | pty_in | id attach | byte gõ (UTF-8 / escape) |
| 3 | ping | 0 | rỗng; bên nhận trả ping |

Gộp frame (≤ 30 ms) ở gateway / điện thoại, không ở relay.

## 4. JSON

Lệnh: `{id, t, ...}` (`id` chuỗi ≤ 32 ký tự, duy nhất trong phiên). Trả lời: `{re: id, ok: true, ...}` hoặc
`{re: id, ok: false, error: <mã>, detail?}`. Event: `{t, ...}` không `id`.

Mã lỗi: `denied` · `busy` · `limit` · `notFound` · `invalid` · `timeout` · `offline` (renderer chưa lên) · `rate` · `proto` · `internal`.

### 4.1 Phiên
| Lệnh | Trả lời / event |
|---|---|
| `hello {proto, app, platform, caps[]}` | `{ok, machine, name, version, minProto, theme{mode, palette{token: color}}, locale}` + event `snapshot` + `ws.snapshot` |
| `resync` | event `snapshot` |
| — | event `machine.restarting`, `device.revoked`, `remote.off` (gửi trước khi đóng socket), `relay.token {token}` (khi token còn < 7 ngày, token mới hạn 30 ngày) |

### 4.2 Dashboard
- Event `snapshot {seq, data}` và `events {seq, events[]}`: đúng `MonitorSnapshot` / `ServerMessage` của `/ws` (`packages/shared/src/types.ts`), đã bỏ field chỉ desktop dùng. Hụt `seq` → điện thoại gửi `resync`.
- Lệnh cho phép: `terminal.alias`, `terminal.note`, `terminal.snooze`, `terminal.dismiss`, `history.get`, `tool-spans.get` (shape như `ClientMessage`). Cấm: `client.presence`, `terminal.open`, `terminal.focusWindow`, `folder.pick`, `favorite.open`, `notify.*`, `sensitive.*`, `office.*`.

### 4.3 Workspace
| Lệnh | Ý nghĩa |
|---|---|
| `ws.get` | trả `ws.snapshot` hiện tại |
| `ws.open {cwd, preset: "claude"\|"codex"\|"shell", prompt?, submit?, resume?}` | mở pane trong tab project, không đổi focus desktop → `{paneKey}` |
| `ws.close {paneKey}` | pane bận / file chưa lưu → event `confirm.ask` cho chính điện thoại; không hiện gì trên desktop |
| `ws.rename {tabKey, name\|null}` · `ws.move {paneKey, tabKey}` · `ws.reopen` · `ws.compact {paneKey}` | như desktop |
| `ws.send {paneKey, text}` | ≤ 2000 ký tự; pane idle → gửi luôn, không idle → chỉ gõ sẵn → `{submitted}` |
| `ws.keys {paneKey, keys, since}` | nút trả lời xin quyền; `since` lệch `statusSince` → `busy` |

`timeout` của lệnh `ws.*` = kết quả chưa rõ (renderer có thể vẫn đã làm): client KHÔNG tự retry lệnh có tác dụng phụ. Gateway gửi kèm `rid` (theo phiên + `id` lệnh) cho renderer; renderer bỏ lệnh trùng `rid` (50 gần nhất, 60 s) và trả lại kết quả lần đầu.

Event `ws.snapshot {rev, tabs[]}` / `ws.patch {base, rev, ops[]}` (`ops` = JSON Patch RFC 6902 trên snapshot; chỉ áp khi `base` = rev đang có, lệch → `ws.get`).
`tabs[i] = {key, name, project, tree, zoom, folded, panes[]}`; `tree` = `{pane} | {dir, ratio, a, b}`;
`panes[j] = {key, kind, preset, agent, title, cwd, branch, state, rail, waitText, attention, subagents, sessionId, terminalId, dirty, busy}`.

### 4.4 PTY
| Lệnh | Ý nghĩa |
|---|---|
| `pty.attach {stream, sessionId, cols, rows}` | xem pane (viewer); output về kind 1 cùng `stream`; event `pty.ready {stream, cols, rows}` rồi replay |
| `pty.detach {stream}` | thôi xem |
| `pty.resize {stream, cols, rows}` | lưu cỡ của điện thoại; chỉ áp khi điện thoại đang gõ |
| kind 2 trên `stream` | gõ (ghi vào PTY); gateway gửi `claim` cho hub khi đó là phím người dùng thật, điện thoại thành người gõ, cỡ của nó áp vào PTY |

Reply `pty.attach` = `{re, ok, stream}`; `sessionId` phải là pane trong Workspace summary (không thì `notFound`). Event: `pty.resized {stream, cols, rows}`, `pty.exit {stream, code}` (`code:null` = mất hub), `pty.overflow {stream}` (attach lại để lấy replay). Max 4 attach / thiết bị (thứ 5 → `limit`). Ngay sau khi điện thoại nối (hello), summary được coi là cũ tới lần `publish` kế (max 10 s): `pty.attach` với pane chưa có trong summary chờ summary mới tối đa 2 s rồi mới trả `notFound`.

### 4.5 Viewer
Bảng cố định (lệnh ngoài bảng → `denied`); path luôn tuyệt đối, phải nằm trong root pane đang sống hoặc grant của desktop (full-access của desktop KHÔNG áp cho điện thoại); không có `grant-folder`, chọn thư mục / file, mở path, reveal.

| Lệnh | Args | Kênh | Reply `data` |
|---|---|---|---|
| `fs.list` | `{root, dir}` | `ccm:list-dir` | kết quả list-dir |
| `fs.files` | `{root}` | `ccm:list-files` | danh sách file |
| `fs.read` | `{path}` | `ccm:read-file` | `{kind: text|binary|image|too-large|missing, path, size, mtimeMs, text?, eol?, bom?}` (desk đọc max 2 MiB, nhị phân / ảnh không có nội dung) |
| `fs.write` | `{path, text, base?}` (`base` = `mtimeMs` đã đọc → `conflict`) | `ccm:file-write` | `{ok, …}` hoặc `{ok:false, error}` (chỉ ghi đè file có sẵn, 20 / phút) |
| `fs.watch` / `fs.unwatch` | `{root}` | `ccm:watch` / `unwatch` | `true` / `false` |
| `git.status` · `git.diff` · `git.file` | `{root}` · `{root, path, opts?}` · `{path}` | `ccm:changes-status` / `changes-diff` / `changes-file` | như desktop |
| `git.watch` / `git.unwatch` | `{root}` | `ccm:changes-watch` / `changes-unwatch` | `true` / `false` |
| `git.stage` · `git.unstage` · `git.commit` | `{root, paths[]|hunk}` · `{root, paths[]}` · `{root, message, amend?}` | `ccm:git-stage` / `git-unstage` / `git-commit` | `{ok, …}`; amend hỏi xác nhận |
| `git.discard` | `{root, path, hunk?}` | `ccm:changes-discard` | luôn hỏi xác nhận trước (gateway) |
| `log.refs` · `log.page` · `log.commit` · `log.diff` | `{root}` · `{root, query}` · `{root, hash}` · `{root, hash, path}` | `ccm:log-refs` / `log-page` / `log-commit` / `log-diff` | như desktop |
| `branch.create` · `branch.update` · `branch.fetch` · `branch.push-preview` | `{root, …}` | `ccm:git-branch-create` / `git-branch-update` / `git-fetch` / `git-push-preview` | timeout 60 s với update / fetch |
| `branch.delete` | `{root, ref}` | `ccm:git-branch-delete` | hỏi xác nhận (kể cả force khi chưa merge) |
| `branch.push` | `{root, ref, remote?, branch?, setUpstream?}` | `ccm:git-push` | luôn hỏi xác nhận trước (gateway), 60 s |
| `run.configs` · `run.devices` · `run.sessions` | `{root}` · — · — | `ccm:run-configs` / `run-devices` / `run-sessions` | list |
| `run.start` | `{root, configId, deviceId?}` | `ccm:run-start` | config chưa trust → hỏi xác nhận |
| `run.stop` · `run.reload` | `{sessionId}` · `{sessionId, hot?}` | `ccm:run-stop` / `run-reload` | |

Reply thành công `{re, ok:true, data}` (`data` là kết quả nguyên của desk; lỗi nghiệp vụ nằm trong `data.ok === false` + `data.error`: `conflict`, `not-merged`, `failed`…); từ chối / ngoài root → `{ok:false, error:"denied"}`; thiếu / sai field → `invalid`; reply quá ~900 000 byte → `limit` (proto 1 MiB / message); desktop không phản hồi → `timeout` (15 s, 60 s với fetch / push / run.start) hoặc `offline`. Max 120 lệnh viewer / phút / thiết bị, `fs.write` 20 / phút, 8 watch / phiên (thứ 9 → `limit`), 4 lệnh chờ / phiên (`busy`).
Event (chỉ tới phiên đã đăng ký, gom 100 ms): `fs.changed {root, paths[], more?}` (sau `fs.watch`; mỗi event max 64 KB hoặc 200 path, bị cắt thì `more: true`, điện thoại tự `fs.list` lại), `git.changed {root}` (sau `git.watch`), `run.event {event}` (`event` = payload `ccm:run-event`; tự đăng ký khi phiên gọi `run.sessions` / `run.start`). Phiên đóng → mọi watch nhả.
Xác nhận (4.6) dùng cho `branch.delete`, `git.discard`, `branch.push`, `git.commit` amend, `run.start` config lạ: không hộp native nào hiện trên desktop; có thể có 2 lượt `confirm.ask` liên tiếp (vd xoá nhánh chưa merge → xoá cưỡng bức). `run.start` mà build Android cần gỡ bản cũ khác key (`reinstall`) chạy ở thread runner: host nhận `origin` = owner của phiên điện thoại (`run.origin`) nên gửi event `confirm.ask {re:"run.reinstall"}` tới đúng điện thoại đó, không hộp native; `confirm.answer` đánh thức runner, không trả lời trong 60 s / phiên đóng → coi như huỷ. Từ desktop vẫn hộp native.

### 4.6 Xác nhận
Lệnh cần xác nhận trả ngay `{re, ok:true, confirm: cid}`; event `confirm.ask {cid, re, title, detail, ok, cancel}` chỉ tới thiết bị gửi lệnh; lệnh `confirm.answer {cid, ok}` (`cid` dùng 1 lần); từ chối / quá 60 s / phiên đóng → lệnh gốc kết thúc `{cancelled:true}`. `confirmed` chỉ gateway đặt.

## 5. Giới hạn
`ws.*` 30 / phút · `fs.write` 20 / phút · JSON 1 MiB · text prompt 2000 · max 8 kết nối chưa xác thực / IP trên LAN · thiết bị không dùng 30 ngày tự hết hạn.

## 6. Mã đóng WS
`4400` handshake lỗi · `4401` thiết bị không hợp lệ / bị thu hồi · `4426` proto cũ · `4429` rate · `4503` gateway tắt / desktop vắng trên relay · `4409` desktop bị thay bởi kết nối desktop mới (relay).

## 7. Relay (bản deploy)

- URL: mỗi người một relay riêng trên Cloudflare của mình (`https://orrery-relay.<subdomain>.workers.dev`), `relay.url` rỗng = chỉ LAN, QR không có relay. Route, giới hạn, lệnh test / deploy: `apps/relay/README.md`; dựng từ Settings: §7.3. Relay `orrery-relay.jinvu.workers.dev` là relay riêng của chủ nhân (config cũ giữ nguyên url), không phải mặc định chung.
- `POST /register` / `/revoke`: body có `ts` (giây), chữ ký Ed25519 trên `"orrery-register/1" ‖ machineId ‖ ts`. Desktop WS `/m/<id>?ts&sig` ký `"orrery-machine/1" ‖ machineId ‖ ts`; `ts` mỗi lần nối lại phải lớn hơn lần trước (không thì 409).
- Desktop mất kết nối → mọi điện thoại của máy đó bị đóng `4503`.
- Desktop đóng đúng 1 điện thoại: gửi envelope `connId = 0` với `{"t":"close","conn":n,"code":c}`, `c` ∈ `4400` / `4401` / `4426` / `4503` (mã khác, `conn` sai, JSON sai → bỏ qua); relay đóng phone `p:<n>` với mã đó và không báo ngược `close` cho desktop. Gateway dùng cho `4400` / `4401` / `4426` trên relay (trước đây chỉ ngừng trả lời).

### 7.1 Link mở pane từ Telegram / ntfy
- Tin `waiting` / `question` / `permission` / `done` của 1 pane có nút URL Telegram "Mở trên điện thoại" (hàng riêng dưới các nút trả lời) / ntfy `click` + action `view` → `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` (trang bounce của relay, mục trên). Tin `job` / `limit` / `disk` / `note` không có link.
- Core gọi host `remote.link {terminalId}` → vỏ trả `{url}` hoặc `{url: null}`. `url` chỉ có khi remote đang chạy, có ≥ 1 điện thoại đã ghép, Workspace summary mới nhất có pane với `terminalId` đó, relay `https`, `paneKey` khớp `[A-Za-z0-9_.:-]{1,128}`. Core kiểm lại (https, đúng dạng `/o/<machine>/<pane>`, không query) trước khi gắn; không hợp lệ → tin y như cũ.
- ntfy suy `terminalId` từ khoá episode (`w:<terminalId>|…`, `d:<terminalId>|…`) vì tin phone không mang `terminalId`.
- App: `orrery-remote://m/<machineId>/p/<paneKey>` (Android intent-filter, iOS `CFBundleURLTypes`) và `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` (Android App Links `autoVerify`, iOS Associated Domains `applinks:`). Máy không có trong danh sách đã ghép → snackbar "Chưa ghép máy này". Máy có rồi → chọn máy, mở Workspace, chờ summary (max 20 s), mở tab chứa pane; summary mới mà không có pane → "Pane này không còn mở".
- Deploy relay cần `ANDROID_CERT_SHA256` (keystore debug: `1C:73:97:…:D6:61`, trong `apps/relay/wrangler.jsonc`) và `APPLE_APP_ID` (để trống tới khi có Team ID); sau đó `GET /.well-known/assetlinks.json` trả package `dev.ccm.orrery.remote`.

### 7.2 Push native FCM (phía điện thoại, WP-14)

- Firebase do người dùng tự thiết lập; app không có `google-services.json` và không dùng Gradle plugin `com.google.gms.google-services`. Firebase khởi tạo lúc chạy: `Firebase.initializeApp(options: FirebaseOptions(apiKey, appId, messagingSenderId, projectId))` bằng options nhận qua phiên Noise.
- Gateway → phone, sau `hello`: event `{t:"push.config", apiKey, appId, senderId, projectId}` hoặc `{t:"push.config"}` (không có field) khi người dùng chưa thiết lập. Chưa thiết lập → app ẩn switch thông báo, hiện dòng "chưa thiết lập trên máy tính". Giá trị phải khớp `[A-Za-z0-9:_.\-]{1,256}`, `projectId` khớp `[a-z0-9-]{1,63}`; không hợp lệ coi như chưa thiết lập.
- Phone → gateway: `push.register {token, platform: "android"|"ios", locale: "vi"|"en"}` (token `[A-Za-z0-9:_.-]{16,512}`) khi bật switch, mỗi lần phiên ready, khi nhận `push.config` mới, khi token đổi (`onTokenRefresh`); `push.unregister` khi tắt switch hoặc khi xoá máy. Trả lời `{re, ok:true}`, lỗi `invalid` (token / locale sai) hoặc `denied` (thiết bị không còn ghép).
- Options được lưu trên điện thoại (`SecureKv` khoá `push.configs`) để mở app từ notification lúc app bị kill vẫn khởi tạo được Firebase và đọc `getInitialMessage`.
- Giới hạn plugin: `firebase_messaging` chỉ hỗ trợ app Firebase mặc định ("Messaging does not yet support multiple Firebase Apps"). Mỗi lần chạy app dùng project của máy đầu tiên có config; máy có `projectId` khác không đăng ký push được (điện thoại không gửi `push.register` cho máy đó).
- Payload FCM `data` chỉ có `{m: machineId, p: paneKey, k: done|question|permission|attention}`; chạm notification (cả khi app bị kill) → deep link `orrery-remote://m/<m>/p/<p>` (§7.1). Android channel `attention` (tạo trong `MainActivity`, tên vi / en trong `strings.xml`), quyền `POST_NOTIFICATIONS` xin khi bật switch. iOS: chỉ có code chung, chưa có APNs key / `GoogleService-Info.plist`.
- Gateway quyết định khi nào push (thiết bị không có phiên mở, max 1 push / pane / 30 s, max 30 / giờ / thiết bị) và gửi thẳng tới FCM HTTP v1 (§7.4). Event `push.config` gửi ngay sau `welcome` và mỗi khi người dùng đổi / gỡ cấu hình push trên desktop. `data.k` của thông báo thử là `test` và không có `p` (chạm không mở pane).

### 7.3 Desktop tự dựng relay (WP-19)
Desktop (Rust, `remote/{cf_http,cf_api,relay_setup}.rs`) deploy bundle đóng sẵn `apps/shell/resources/relay/{worker.js,worker.json}` (sinh bằng `node scripts/relay-bundle.mjs`, `worker.json` = `{hash, mainModule, compatibilityDate, migrationTag, sqliteClasses, bindings}`) lên Cloudflare của người dùng bằng API token. Không cần Node / wrangler lúc chạy.

Token tạo ở `https://dash.cloudflare.com/profile/api-tokens` → Create Custom Token: **Account · Workers Scripts · Edit** (gồm subdomain workers.dev + bật route) và **Account · Account Settings · Read** (để `GET /accounts`). Chọn "Include · tài khoản cần dùng". Token cất trong keychain (`orrery-cf-token`), không bao giờ gửi lại trang / log / điện thoại.

Chuỗi gọi (Bearer token, base `https://api.cloudflare.com/client/v4`):
1. `GET /user/tokens/verify` (`result.status == active`); token account-owned không qua được → thử `GET /accounts?per_page=1`.
2. `GET /accounts?per_page=50&page=n` theo `result_info.total_pages` (max 10 trang) → `{id,name}` (id phải 32 hex).
3. `GET /accounts/{id}/workers/subdomain`; chưa có (404 / lỗi 10007) → `PUT` cùng đường dẫn `{"subdomain":"orrery-<6 hex>"}` (thử lại tối đa 3 tên). Subdomain trả về phải khớp `[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?` mới được ghép vào url, sai → lỗi `subdomain`.
4. `GET /accounts/{id}/workers/scripts` → `migration_tag` của script `orrery-relay`.
5. `PUT /accounts/{id}/workers/scripts/orrery-relay` multipart: part `metadata` (JSON: `main_module`, `compatibility_date`, `bindings` = `durable_object_namespace` ROOM/Room, `ratelimit` LIMITER ns 9701 30 / 60 s, `plain_text` ANDROID_CERT_SHA256, APPLE_APP_ID; `migrations` chỉ khi tag trên Cloudflare khác tag cuối của bundle (`worker.json.migrations` giữ đủ danh sách): chưa có + 1 migration → `{new_tag,new_sqlite_classes}`, chưa có + nhiều → `{new_tag,steps:[tất cả]}`, tag cũ nằm trong danh sách → `{old_tag,new_tag,steps:[các bước sau old_tag]}`, tag lạ → lỗi `migration` (không ghi đè), trùng → bỏ) + part `worker.js` (`application/javascript+module`).
6. `POST /accounts/{id}/workers/scripts/orrery-relay/subdomain {"enabled":true,"previews_enabled":false}`.
7. `GET <url>/health` (không gửi header Authorization, không theo redirect, timeout 10 s / request) mỗi 3 s, max 60 s, cần `200 ok`. Hết hạn mà chưa phản hồi (DNS workers.dev mới tạo có thể chậm): vẫn lưu url + hash, deploy trả `{ok:true,url,warning:"health"}`, `remoteRelayStatus` có `state:"ready"` + `warning:"health"` (UI hiện "chưa phản hồi", relay tự thử nối lại với backoff); warning xoá ở lần deploy / gỡ kế tiếp.
8. Ghi config: `relay.url`, `accountId`, `accountName`, `deployedHash`; url đổi → `registered=false`; gateway đang chạy thì restart (điện thoại đang nối bị ngắt, tự nối lại).

IPC (page → shell, `window.ccm`): `remoteRelayStatus()` → `{state: none|ready|deploying|error, url?, account?, updateAvailable, hasToken, error?}`; `remoteRelayToken(token)` → `{ok:true, accounts:[{id,name}]}` | `{ok:false, error}` (đúng token mới lưu); `remoteRelayDeploy(accountId)` → `{ok:true,url}` | `{ok:false,error}`; `remoteRelayRemove(deleteWorker)` → `{ok, workerDeleted}` (xoá url + token keychain + account, ngắt relay; `deleteWorker` mới gọi `DELETE …/scripts/orrery-relay?force=true`, và shell tự hiện confirm native trước khi xoá — lệnh từ page một mình không xoá được; huỷ → `{ok:false,error:"cancelled"}`; xoá lỗi thì giữ nguyên cấu hình; relay cũ không có `accountId` → `workerDeleted:false`; keychain không xoá được token → `{ok:false,error:"keyring"}` sau khi đã gỡ cấu hình). Gỡ khi đang deploy: đặt cờ huỷ, deploy dừng ở bước kế (`cancelled`, không ghi url lại), rồi gỡ chạy; gỡ giữ `busy` suốt lúc gọi API nên deploy / `remoteRelayToken` song song bị từ chối `busy`. Event `ccm:remote-relay-progress {step: verify|subdomain|upload|enable|health|done}`; `ccm:remote-changed` bắn lúc bắt đầu / xong. Deploy lỗi khi relay đã có url: `state:"ready"` + `error` riêng (relay cũ vẫn chạy). Deploy xong mà url đổi: gateway chỉ restart nếu người dùng vẫn đang bật. `updateAvailable` = `deployedHash` khác hash bundle trong app (chỉ relay do Orrery dựng, `accountId` không rỗng); nút "Cập nhật relay" = gọi lại deploy (tag trùng → không gửi migrations, deploy lặp an toàn). Mã lỗi: `token_invalid` `token_scope` `rate_limited` `no_account` `no_token` `keyring` `offline` `subdomain` `upload` `enable` `health` `bundle` `busy` `cancelled` `migration` `rejected` (kiểu: `packages/shared/src/remote.ts` `RelaySetupStatus` …).

### 7.4 Desktop gửi push FCM thẳng (WP-20)
Desktop (Rust, `remote/{fcm_files,fcm_jwt,fcm,push,push_setup}.rs`) giữ service account của người dùng và gửi FCM HTTP v1, không qua relay. Relay không giữ secret Firebase.

Cài đặt bằng 2 file (người dùng thả vào Settings → Điều khiển từ xa → Thông báo đẩy): `google-services.json` (Firebase console → Project settings → app Android `dev.ccm.orrery.remote`) và service account JSON (Project settings → Service accounts → Generate new private key). Desktop kiểm: cả hai là JSON object ≤ 100 KB; `project_info.project_id` (`[a-z0-9-]{4,64}`) và `project_number` (chỉ số) hợp lệ; service account `type=service_account`, có `client_email`, `private_key` là PEM PKCS#8 mà ring đọc được, `project_id` trùng; `client[]` có package `dev.ccm.orrery.remote` (lấy `mobilesdk_app_id` + `api_key[].current_key`). FirebaseOptions lưu ở `config.json` `firebase {projectId, apiKey, appId, senderId, clientEmail}` (senderId = `project_number`); service account lưu trong keychain `orrery-fcm-sa` dưới dạng `{client_email, private_key}`, không bao giờ vào config / status / log / điện thoại. `token_uri` của file bị bỏ qua, luôn dùng `https://oauth2.googleapis.com/token`. Đổi sang project khác → xoá mọi token đã đăng ký (token thuộc project cũ); cùng project mà đổi service account → giữ token.

Gửi: JWT RS256 `{iss: client_email, scope: https://www.googleapis.com/auth/firebase.messaging, aud: token url, iat, exp: iat + 3600}` ký bằng ring `RSA_PKCS1_SHA256` → `POST https://oauth2.googleapis.com/token` (`grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer`) → access token cache 55 phút trong RAM → `POST https://fcm.googleapis.com/v1/projects/<projectId>/messages:send` body `{message: {token, notification: {title, body}, data: {m, p, k}, android: {priority: high, notification: {channel_id: attention}}}}`. Nguồn kích hoạt là chính gateway (`push_watch.rs`), không phụ thuộc relay hay Telegram: một subscription `StoreTap` giữ suốt lúc remote chạy, theo dõi `terminal.status`; `working → waiting` = `k: attention`, `working → idle` = `k: done`; terminal -> pane qua workspace summary (không có pane thì bỏ). Job xếp hàng được kiểm lại `is_live` ngay trước khi gửi. Chữ title / body cố định theo `k` và locale đã đăng ký (vi / en), không chứa nội dung phiên.

Kết quả: 2xx = đã gửi. 401 → bỏ access token, lấy lại 1 lần. 404 hoặc `errorCode UNREGISTERED` → xoá token của thiết bị (chỉ cái đó). 429 / 5xx / mất mạng → thử lại tối đa 3 lần (backoff 2 s, 4 s), vẫn lỗi thì giữ token, `last_error` = `quota` / `unavailable` / `offline`. 403 → `denied` (API FCM chưa bật hoặc service account thiếu quyền), 400 khác → `rejected`, OAuth lỗi → `auth`. Thu hồi thiết bị → token biến mất cùng bản ghi thiết bị.

IPC (`window.ccm`): `remotePushStatus()` → `{configured, projectId?, tokens, error?}`; `remotePushSetup(googleServices, serviceAccount)` (nội dung file, trang đọc bằng File API) → `{ok:true, projectId}` | `{ok:false, error}` với `bad_json` `bad_service_account` `mismatch_project` `no_android_client` `keyring`; `remotePushTest(deviceId?)` → `{ok:true, sent}` | `{ok:false, error}` (`not_configured` `no_device` `unregistered` `auth` `denied` `rejected` `quota` `unavailable` `offline`); `remotePushRemove()` → `{ok:true}` (xoá options, xoá mục keychain,  mọi token; gửi `push.config` rỗng). Kiểu: `packages/shared/src/remote.ts` `PushStatus` / `PushSetupResult` / `PushTestResult`. File thả sai chỗ: file google-services ở ô service account, hoặc file service account không phải JSON → `bad_service_account`; service account ở ô google-services → `bad_json`.
