# Orrery Remote — giao thức điện thoại ↔ desktop (proto 1)

Hợp đồng chung cho gateway desktop (`apps/shell/src/remote/`), crate `apps/core/crates/core-remote-proto`,
relay (`apps/relay`) và app `apps/mobile`. Thiết kế + lý do: `.planning/PLAN-v97-mobile.md`.
Đổi file này = tăng `proto` nếu phá tương thích.

## 1. Lớp kết nối

| Đường | Socket | Bên ngoài Noise |
|---|---|---|
| LAN | `ws://<ip LAN>:4320/remote` (mặc định port 4320, `CCM_REMOTE_PORT`) | mỗi WS binary message = 1 Noise message |
| Relay | desktop `wss://<relay>/m/<machineId>`, điện thoại `wss://<relay>/p/<machineId>?t=<token>` (đã ghép) hoặc `?pt=<vé ghép>` (đang ghép, WP-52) | desktop: `[u16 connId BE][noise msg]`; `connId = 0` = control JSON (bảng dưới); điện thoại: không có envelope (DO thêm / bỏ) |

Control `connId = 0` (JSON, ≤ 256 B):

| Chiều | JSON | Ý nghĩa |
|---|---|---|
| relay → desktop | `{t:"open", conn, ip?, country?, pairing?}` | điện thoại mới; `ip` = `cf-connecting-ip` (chỉ ký tự `[0-9A-Fa-f:.]`, ≤ 45), `country` = `cf-ipcountry` (`[A-Z0-9]{2}`), `pairing: true` chỉ khi nối bằng vé `pt`. Desktop cũ bỏ qua field lạ |
| relay → desktop | `{t:"close", conn}` | điện thoại đã đi |
| desktop → relay | `{t:"close", conn, code}` | đóng đúng 1 điện thoại (§7) |
| desktop → relay | `{t:"pair.open", h, ttl}` | mở vé ghép: `h` = base64url(SHA-256(ASCII `pt`)) 43 ký tự, `ttl` = số giây còn lại của QR (nguyên 1–300; tương đối để đồng hồ desktop lệch relay không làm vé bị bỏ âm thầm, vòng 2); relay lưu `h` + `exp = now(relay) + ttl` (1 vé / máy, vé mới thay vé cũ); sai dạng → bỏ qua |
| desktop → relay | `{t:"pair.close"}` | xoá vé (ghép xong, huỷ, sai 5 lần, hoặc ngay khi desktop nhận `open` có `pairing:true` = relay đã tiêu vé). Desktop nối lại relay → relay tự xoá vé, desktop chỉ gửi lại `pair.open` nếu QR còn hiện **và** vé chưa bị tiêu (vòng 2, `Pairing::spend_relay`; QR vẫn ghép được qua LAN, muốn ghép qua relay lần nữa thì bấm Mã mới) |

Control chỉ đọc từ socket desktop đã ký (`/m/`) và chưa bị thay; byte điện thoại gửi luôn là data của `conn` của nó, không bao giờ thành control.

- LAN: không nhận frame nào trong 45 s (kể cả ping WS / kind 3) → gateway đóng phiên; điện thoại gửi ping ≤ 30 s / lần.
- Keepalive relay: text frame `"ping"` → DO tự trả `"pong"` (`setWebSocketAutoResponse`), ngoài Noise, mỗi 30 s.
- Keepalive phiên relay (WP-30): DO không biết phone chết khi TCP phone đứt không đóng (text `"ping"` được DO tự trả, không tới desktop) nên desktop gửi Noise ping (kind 3) mỗi 15 s; mọi frame từ phone (kể cả ping trả lại) reset timer; im lặng ≥ 45 s → gateway đóng phiên (close 4400 `idle` + control close conn cho relay), `live` bỏ khỏi `remoteStatus`. Phone không đổi: nó đã trả ping khi không có ping đang chờ. Desktop chỉ ping trên relay (LAN đã có timeout 45 s, tránh hai bên ping chéo).
- Trước khi vào Noise không có dữ liệu ứng dụng nào. Handshake chưa xong trong 5 s → đóng. Frame đầu sau bắt tay IK phải là `hello` giải mã được trong 5 s, chỉ từ đó phiên mới được tính (`lastSeen`, thông báo, max 16 phiên); trước đó inbox max 4 frame / 64 KB.
- mDNS `_orrery._tcp`, TXT `{v, k, n}`: `k` = base64url 8 byte đầu SHA-256(key tĩnh Noise của máy), `n` = tên máy; không có machineId.
- Ghép sai: mỗi lần sai mã (bắt tay XXpsk3 hỏng ở msg 3) tính vào QR đang hiện; sai **5 lần** → huỷ QR (status `pairing: null`, `pairEnded: "tries"`, relay nhận `pair.close`). Ngoài ra max 5 lần thử / phút / IP (LAN, hoặc IP relay báo trong `open`; relay không báo IP → 20 / phút chung) → `4429`.

## 2. Noise

- Prologue: `b"orrery-remote/" ‖ proto (u16 BE)`. Msg 1 của cả 2 bắt tay gửi dạng `proto (u16 BE) ‖ noise msg1`: responder đọc proto trước để dựng prologue, nhờ đó trả được `ProtoTooOld {min}` / `ProtoTooNew {max}` thay vì lỗi giải mã. Payload msg 2 IK = `{proto}`; msg 1 / msg 2 ghép payload rỗng. Vector cố định: `apps/core/crates/core-remote-proto/tests/vectors.json`. Bên nhận gặp lỗi giải mã 1 lần → đóng kết nối.
- **Ghép**: `Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s`, điện thoại initiator. PSK = HKDF-SHA256(ikm = `code` 16 B, salt = `"orrery-pair"`, info = `machineId`). Sau msg 2 điện thoại so key tĩnh desktop với `mkey` trong QR, khác → dừng (không gửi msg 3). Payload msg 3 (JSON): `{deviceId, name, platform?}` (`platform` `[A-Za-z0-9]{1,16}`, Android gửi `android`, web app gửi qua `Pairing.fromQrOn`; field sai dạng → desktop đóng `4400`). Desktop hỏi xác nhận native (mục hộp duyệt dưới), lưu thiết bị, gửi frame `pair.ok {machine, name, relay?, relayToken?}` hoặc `pair.denied` rồi đóng `1000` (cả LAN lẫn relay, §7).
- **Phiên**: `Noise_IK_25519_ChaChaPoly_BLAKE2s`, điện thoại initiator. Payload msg 1: `{deviceId, proto}`. Desktop kiểm `deviceId` + key tĩnh trong `devices.json`, sai → đóng (`4401`); `proto < minProto` → đóng `4426` ngay (không payload). Gateway phân biệt ghép / phiên theo độ dài msg 1 (ghép luôn 50 B = 2 B proto + 48 B Noise, IK dài hơn). Sau `pair.ok` / `pair.denied` gateway đóng `1000`, điện thoại nối lại bằng IK. Qua relay desktop gửi control close `1000` cho conn ghép; relay chỉ nhận `1000` cho conn `pairing` (conn token: bỏ qua). Điện thoại vẫn tự đóng socket sau khi đọc `pair.ok` / `pair.denied` (relay cũ chưa nhận `1000` thì vẫn đóng được). Conn `pairing` chỉ nhận bắt tay ghép: msg 1 không phải 50 B → gateway đóng `4400` (IK trên conn ghép bị từ chối kể cả thiết bị đã ghép).
- **QR v2** (desktop chỉ phát v2, WP-52): base64url(JSON) `{v:2, machine, name, lan?: ["<ip>:<port>", …], relay?, pt?, mkey, code, exp}`. `lan` = mọi IP LAN gateway đang nghe (max 4, IPv4 / tên máy, không có `:` trong host, port 1–65535); `relay` = `relay.url` khi có; `pt` = vé ghép relay 16 B ngẫu nhiên base64url (22 ký tự), chỉ có khi relay đang `online` lúc bấm Thêm điện thoại (có `pt` thì phải có `relay`). v2 không được có `host` / `port`; v1 không được có `lan` / `pt`. Cỡ thật (`tests/pair.rs`): 2 LAN + relay workers.dev + `pt` = **410 ký tự**, 1 LAN không relay = **263**; > 300 vì base64 của JSON, desktop vẽ QR mức sửa lỗi `L` (≈ version 13). Giới hạn đọc `MAX_QR_LEN` 640.
- **QR v1** (điện thoại vẫn đọc): `{v:1, machine, name, host?, port?, relay?, mkey, code, exp}`; `host` → 1 đường LAN, thiếu `port` → 4320.
- Mã ghép sống **2 phút** (`PAIR_TTL` 120 s, trước là 5 phút), dùng 1 lần, chỉ hợp lệ khi Settings đang hiện QR; sai 5 lần huỷ (§1). `ccm:remote-pair-start` → `{qr, exp, relay, lan}` (`relay` = QR có `pt`, `lan` = số đường LAN); status có `pairing: {exp, relay} | null`, `pairEnded: "tries" | null`.
- **Chọn đường khi ghép (điện thoại)**: thử song song mọi `lan` + mDNS trong **3 s** (`pairLanBudget`); socket LAN đầu tiên mở được thì ghép trên đó. Không mở được, hoặc hỏng **trước khi gửi msg 3** (đóng, hết giờ, lỗi giao thức, `mkey` lệch) và QR có `relay` + `pt` → ghép lại từ đầu (bắt tay mới) qua `wss://<relay>/p/<machineId>?pt=<pt>`. Hỏng sau msg 3 → không thử relay (desktop có thể đang hỏi duyệt). Không có `pt`: như v1 (chỉ LAN; không đường nào → `noRoute`, mở không được → `network`). UI ghép hiện đường đang dùng (icon Wi-Fi / mây + chữ). Sau `pair.ok` lưu `relay` + `relayToken` như cũ; `lastHost` = đường LAN đã dùng, hoặc đường LAN đầu trong QR nếu ghép qua relay.
- **Vé ghép relay**: desktop gửi `pair.open {h, ttl}` (§1) khi tạo QR có `pt`, `pair.close` khi QR dùng xong / huỷ / sai 5 lần. Phone nối `/p/<id>?pt=` không cần `t`; relay kiểm `pt` khớp `[A-Za-z0-9_-]{22}`, SHA-256 so hằng thời gian với `h`, `now < exp`; đúng → xoá vé (mỗi vé đúng 1 kết nối, kể cả 2 socket cùng lúc), gửi `open {…, pairing:true}`. Sai / hết hạn / đã dùng → đóng `4401` (`bad ticket` / `expired`) và tính 1 lỗi cho IP; 20 lỗi / 10 phút / IP → `4429` cho vé từ IP đó (bộ đếm trong RAM của DO, chỗ móc WP-59). Không có desktop → `4503`, vé giữ nguyên.
- **Hộp duyệt**: mọi lần ghép (LAN và relay) gateway hỏi native: vi "Ghép qua <LAN \| relay> từ IP <ip> [<quốc gia>]: điện thoại <platform> «<tên>»?" / en "Pair via <LAN \| relay> from IP <ip> [<country>]: <platform> phone “<name>”?" (`remote/text.rs` `pair_title`). Đường + IP + quốc gia do gateway / relay đặt luôn đứng trước; tên do điện thoại khai đứng cuối và qua `shown_name` (bỏ ngoặc kép / ngoặc «» “” ‘’, nháy đơn, backtick, control, bidi / zero-width, gộp khoảng trắng) để tên không giả được câu hỏi (vòng 2); IP LAN = địa chỉ TCP, IP relay = `ip` trong `open`; platform / IP / quốc gia sai dạng thì bỏ ("IP không rõ"). Từ chối / hết 120 s → `pair.denied`. Cảnh báo "thiết bị mới ghép" (#27) để WP-59.

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
`1000` ghép xong (`pair.ok` / `pair.denied`; relay: reason `paired`, chỉ conn `pairing`) · `4400` handshake lỗi (gồm IK trên conn ghép, sai mã ghép) · `4401` thiết bị không hợp lệ / bị thu hồi; trên relay còn: token sai, vé ghép sai / đã dùng (`bad ticket`) / hết hạn (`expired`) · `4426` proto cũ · `4429` rate (relay: còn 20 vé sai / 10 phút / IP) · `4503` gateway tắt / desktop vắng trên relay · `4409` desktop bị thay bởi kết nối desktop mới (relay).

## 7. Relay (bản deploy)

- URL: mỗi người một relay riêng trên Cloudflare của mình (`https://orrery-relay.<subdomain>.workers.dev`), `relay.url` rỗng = chỉ LAN, QR không có relay. Route, giới hạn, lệnh test / deploy: `apps/relay/README.md`; dựng từ Settings: §7.3. Relay `orrery-relay.jinvu.workers.dev` là relay riêng của chủ nhân (config cũ giữ nguyên url), không phải mặc định chung.
- `POST /register` / `/revoke`: body có `ts` (giây), chữ ký Ed25519 trên `"orrery-register/1" ‖ machineId ‖ ts`. Desktop WS `/m/<id>?ts&sig` ký `"orrery-machine/1" ‖ machineId ‖ ts`; `ts` mỗi lần nối lại phải lớn hơn lần trước (không thì 409).
- Desktop mất kết nối → mọi điện thoại của máy đó bị đóng `4503`.
- Desktop đóng đúng 1 điện thoại: gửi envelope `connId = 0` với `{"t":"close","conn":n,"code":c}`, `c` ∈ `4400` / `4401` / `4426` / `4503`, hoặc `1000` khi `conn` là conn ghép (`pairing`) (mã khác, `conn` sai, JSON sai → bỏ qua); relay đóng phone `p:<n>` với mã đó và không báo ngược `close` cho desktop. Gateway dùng cho `4400` / `4401` / `4426` trên relay (trước đây chỉ ngừng trả lời) và `1000` sau `pair.ok` / `pair.denied` trên conn ghép.
- Ghép qua relay (WP-52): `GET /p/<machineId>?pt=<vé>` (WS) không cần `t`; Worker chuyển `cf-connecting-ip` / `cf-ipcountry` vào DO bằng header nội bộ `x-orrery-ip` / `x-orrery-country` (header cùng tên từ client bị bỏ, Worker dựng header mới). Vé, `pair.open` / `pair.close`, đếm lỗi: §1, §2. Relay cũ (chưa "Cập nhật relay") không biết `pt` → phone bị `4401`, desktop vẫn ghép được qua LAN.

### 7.1 Link mở pane từ Telegram / ntfy
- Tin `waiting` / `question` / `permission` / `done` của 1 pane có nút URL Telegram "Mở trên điện thoại" (hàng riêng dưới các nút trả lời) / ntfy `click` + action `view` → `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` (trang bounce của relay, mục trên). Tin `job` / `limit` / `disk` / `note` không có link.
- Core gọi host `remote.link {terminalId}` → vỏ trả `{url}` hoặc `{url: null}`. `url` chỉ có khi remote đang chạy, có ≥ 1 điện thoại đã ghép, Workspace summary mới nhất có pane với `terminalId` đó, relay `https`, `paneKey` khớp `[A-Za-z0-9_.:-]{1,128}`. Core kiểm lại (https, đúng dạng `/o/<machine>/<pane>`, không query) trước khi gắn; không hợp lệ → tin y như cũ.
- ntfy suy `terminalId` từ khoá episode (`w:<terminalId>|…`, `d:<terminalId>|…`) vì tin phone không mang `terminalId`.
- App: `orrery-remote://m/<machineId>/p/<paneKey>` (Android intent-filter; iPhone dùng web app, cách mở link sẽ bổ sung ở v98) và `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` (Android App Links `autoVerify`; không còn iOS Associated Domains vì không có app iOS native). Máy không có trong danh sách đã ghép → snackbar "Chưa ghép máy này". Máy có rồi → chọn máy, mở Workspace, chờ summary (max 20 s), mở tab chứa pane; summary mới mà không có pane → "Pane này không còn mở".
- Deploy relay cần `ANDROID_CERT_SHA256` (keystore debug: `1C:73:97:…:D6:61`, trong `apps/relay/wrangler.jsonc`) và `APPLE_APP_ID` (để trống; không còn dùng vì không có app iOS native, relay sẽ dọn sau); sau đó `GET /.well-known/assetlinks.json` trả package `dev.ccm.orrery.remote`.

### 7.2 Push native FCM (phía điện thoại, WP-14)

- Firebase do người dùng tự thiết lập; app không có `google-services.json` và không dùng Gradle plugin `com.google.gms.google-services`. Firebase khởi tạo lúc chạy: `Firebase.initializeApp(options: FirebaseOptions(apiKey, appId, messagingSenderId, projectId))` bằng options nhận qua phiên Noise.
- Gateway → phone, sau `hello`: event `{t:"push.config", apiKey, appId, senderId, projectId}` hoặc `{t:"push.config"}` (không có field) khi người dùng chưa thiết lập. Chưa thiết lập → app ẩn switch thông báo, hiện dòng "chưa thiết lập trên máy tính". Giá trị phải khớp `[A-Za-z0-9:_.\-]{1,256}`, `projectId` khớp `[a-z0-9-]{1,63}`; không hợp lệ coi như chưa thiết lập.
- Phone → gateway: `push.register {token, platform: "android"|"ios" (app Flutter chỉ còn gửi `android`; `ios` giữ trong hợp đồng, iPhone dùng kênh Web Push riêng của web app, v98), locale: "vi"|"en"}` (token `[A-Za-z0-9:_.-]{16,512}`) khi bật switch, mỗi lần phiên ready, khi nhận `push.config` mới, khi token đổi (`onTokenRefresh`); `push.unregister` khi tắt switch hoặc khi xoá máy. Trả lời `{re, ok:true}`, lỗi `invalid` (token / locale sai) hoặc `denied` (thiết bị không còn ghép).
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

### 7.5 Web app `/app/*` (WP-53)
Relay phục vụ web app iPhone (`apps/pwa`, WP-54) tại `https://<relay>/app/`. File web nhúng thẳng vào `worker.js` (base64 trong `apps/relay/src/web-assets.generated.ts`, `WebBundle {hash, files{path → {type, cache, etag, data}}}`), nên luồng deploy §7.3 không đổi: "Cập nhật relay" đẩy cả bản web. Cloudflare Workers (Free và Paid) giới hạn 64 MiB bundle không nén, không có giới hạn gzip; `relay-bundle.mjs` fail khi `worker.js` vượt 80% (51,2 MiB).
- Dựng: `node scripts/relay-bundle.mjs` đọc `CCM_PWA_DIST` (mặc định `apps/pwa/dist`, phải có `index.html`); không có thư mục thì `/app/*` trả 404 `web app not bundled`. `web-assets.generated.ts` được trả về bản stub sau mỗi lần chạy (nội dung web không vào `src/`), chỉ nằm trong `apps/shell/resources/relay/worker.js`. `worker.json.hash` gồm cả web nên `updateAvailable` bật khi web đổi.
- Route (GET / HEAD, method khác 405): `/app` → 308 `/app/`; `/app/` = `index.html`; đường không có đuôi file và không có trong bundle → `index.html` (SPA); đường có đuôi mà không có → 404; segment `..`, `.`, chứa `\`, NUL, hoặc `//` sau giải mã → 404; chỉ tra đúng khoá trong map.
- `content-type` theo đuôi (`.wasm` = `application/wasm`, `.webmanifest` = `application/manifest+json`, `.js` = `text/javascript`); đuôi lạ làm bước dựng fail. `ETag` + `If-None-Match` → 304. `Cache-Control`: `assets/<tên>-<hash 8+ ký tự>.<đuôi>` = `public, max-age=31536000, immutable`, còn lại `no-cache`. Worker không tự nén; Cloudflare nén phía edge theo `accept-encoding`.
- Header mọi phản hồi `/app/*`: CSP `default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self' wss://<host relay> wss://*:4320 https://*:4320; worker-src 'self'; manifest-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'` (nguồn LAN là hằng `LAN_SOURCES` trong `apps/relay/src/app.ts`; WebKit có nhận wildcard IP hay không chưa kiểm trên iPhone thật), `Strict-Transport-Security: max-age=31536000`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Permissions-Policy: camera=(self)`; `sw.js` thêm `Service-Worker-Allowed: /app/`. Web app không được có script / style inline.
- `GET /app/version.json` (`no-store`) = `{web, proto, minProto}`: `web` = hash bản web (16 hex), `proto` / `minProto` = `PROTO` / `MIN_PROTO` trong `apps/relay/src/app.ts` (phải đổi cùng `core-remote-proto`).

### 7.4 Desktop gửi push FCM thẳng (WP-20)
Desktop (Rust, `remote/{fcm_files,fcm_jwt,fcm,push,push_setup}.rs`) giữ service account của người dùng và gửi FCM HTTP v1, không qua relay. Relay không giữ secret Firebase.

Cài đặt bằng 2 file (người dùng thả vào Settings → Điều khiển từ xa → Thông báo đẩy): `google-services.json` (Firebase console → Project settings → app Android `dev.ccm.orrery.remote`) và service account JSON (Project settings → Service accounts → Generate new private key). Desktop kiểm: cả hai là JSON object ≤ 100 KB; `project_info.project_id` (`[a-z0-9-]{4,64}`) và `project_number` (chỉ số) hợp lệ; service account `type=service_account`, có `client_email`, `private_key` là PEM PKCS#8 mà ring đọc được, `project_id` trùng; `client[]` có package `dev.ccm.orrery.remote` (lấy `mobilesdk_app_id` + `api_key[].current_key`). FirebaseOptions lưu ở `config.json` `firebase {projectId, apiKey, appId, senderId, clientEmail}` (senderId = `project_number`); service account niêm phong AES-GCM (`o1:`, master key 32 byte trong keychain `orrery-fcm-master`) vào file `<remote dir>/fcm-sa.sealed` (Credential Manager Windows max 2560 byte, `private_key` ~1.8k ký tự vượt khi gộp; mục keychain cũ `orrery-fcm-sa` vẫn đọc được và xoá khi cài lại), không bao giờ vào config / status / log / điện thoại. `token_uri` của file bị bỏ qua, luôn dùng `https://oauth2.googleapis.com/token`. Đổi sang project khác → xoá mọi token đã đăng ký (token thuộc project cũ); cùng project mà đổi service account → giữ token.

Gửi: JWT RS256 `{iss: client_email, scope: https://www.googleapis.com/auth/firebase.messaging, aud: token url, iat, exp: iat + 3600}` ký bằng ring `RSA_PKCS1_SHA256` → `POST https://oauth2.googleapis.com/token` (`grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer`) → access token cache 55 phút trong RAM → `POST https://fcm.googleapis.com/v1/projects/<projectId>/messages:send` body `{message: {token, notification: {title, body}, data: {m, p, k}, android: {priority: high, notification: {channel_id: attention}}}}`. Nguồn kích hoạt là chính gateway (`push_watch.rs`), không phụ thuộc relay hay Telegram: một subscription `StoreTap` giữ suốt lúc remote chạy, theo dõi `terminal.status`; `working → waiting` = `k: attention`, `working → idle` = `k: done`; terminal -> pane qua workspace summary (không có pane thì bỏ). Job xếp hàng được kiểm lại `is_live` ngay trước khi gửi. Chữ title / body cố định theo `k` và locale đã đăng ký (vi / en), không chứa nội dung phiên.

Kết quả: 2xx = đã gửi. 401 → bỏ access token, lấy lại 1 lần. 404 hoặc `errorCode UNREGISTERED` → xoá token của thiết bị (chỉ cái đó). 429 / 5xx / mất mạng → thử lại tối đa 3 lần (backoff 2 s, 4 s), vẫn lỗi thì giữ token, `last_error` = `quota` / `unavailable` / `offline`. 403 → `denied` (API FCM chưa bật hoặc service account thiếu quyền), 400 khác → `rejected`, OAuth lỗi → `auth`. Thu hồi thiết bị → token biến mất cùng bản ghi thiết bị.

IPC (`window.ccm`): `remotePushStatus()` → `{configured, projectId?, tokens, error?}`; `remotePushSetup(googleServices, serviceAccount)` (nội dung file, trang đọc bằng File API) → `{ok:true, projectId}` | `{ok:false, error}` với `bad_json` `bad_service_account` `mismatch_project` `no_android_client` `keyring`; `remotePushTest(deviceId?)` → `{ok:true, sent}` | `{ok:false, error}` (`not_configured` `no_device` `unregistered` `auth` `denied` `rejected` `quota` `unavailable` `offline`); `remotePushRemove()` → `{ok:true}` (xoá options, xoá mục keychain,  mọi token; gửi `push.config` rỗng). Kiểu: `packages/shared/src/remote.ts` `PushStatus` / `PushSetupResult` / `PushTestResult`. File thả sai chỗ: file google-services ở ô service account, hoặc file service account không phải JSON → `bad_service_account`; service account ở ô google-services → `bad_json`.
