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

- URL: `https://orrery-relay.jinvu.workers.dev` (route, giới hạn, lệnh test / deploy: `apps/relay/README.md`).
- `POST /register` / `/revoke`: body có `ts` (giây), chữ ký Ed25519 trên `"orrery-register/1" ‖ machineId ‖ ts`. Desktop WS `/m/<id>?ts&sig` ký `"orrery-machine/1" ‖ machineId ‖ ts`; `ts` mỗi lần nối lại phải lớn hơn lần trước (không thì 409).
- Desktop mất kết nối → mọi điện thoại của máy đó bị đóng `4503`.
- Desktop đóng đúng 1 điện thoại: gửi envelope `connId = 0` với `{"t":"close","conn":n,"code":c}`, `c` ∈ `4400` / `4401` / `4426` / `4503` (mã khác, `conn` sai, JSON sai → bỏ qua); relay đóng phone `p:<n>` với mã đó và không báo ngược `close` cho desktop. Gateway dùng cho `4400` / `4401` / `4426` trên relay (trước đây chỉ ngừng trả lời).

### 7.1 Link mở pane từ Telegram / ntfy
- Tin `waiting` / `question` / `permission` / `done` của 1 pane có nút URL Telegram "Mở trên điện thoại" (hàng riêng dưới các nút trả lời) / ntfy `click` + action `view` → `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` (trang bounce của relay, mục trên). Tin `job` / `limit` / `disk` / `note` không có link.
- Core gọi host `remote.link {terminalId}` → vỏ trả `{url}` hoặc `{url: null}`. `url` chỉ có khi remote đang chạy, có ≥ 1 điện thoại đã ghép, Workspace summary mới nhất có pane với `terminalId` đó, relay `https`, `paneKey` khớp `[A-Za-z0-9_.:-]{1,128}`. Core kiểm lại (https, đúng dạng `/o/<machine>/<pane>`, không query) trước khi gắn; không hợp lệ → tin y như cũ.
- ntfy suy `terminalId` từ khoá episode (`w:<terminalId>|…`, `d:<terminalId>|…`) vì tin phone không mang `terminalId`.
- App: `orrery-remote://m/<machineId>/p/<paneKey>` (Android intent-filter, iOS `CFBundleURLTypes`) và `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` (Android App Links `autoVerify`, iOS Associated Domains `applinks:`). Máy không có trong danh sách đã ghép → snackbar "Chưa ghép máy này". Máy có rồi → chọn máy, mở Workspace, chờ summary (max 20 s), mở tab chứa pane; summary mới mà không có pane → "Pane này không còn mở".
- Deploy relay cần `ANDROID_CERT_SHA256` (keystore debug: `1C:73:97:…:D6:61`, trong `apps/relay/wrangler.jsonc`) và `APPLE_APP_ID` (để trống tới khi có Team ID); sau đó `GET /.well-known/assetlinks.json` trả package `dev.ccm.orrery.remote`.
