# Orrery Remote — giao thức điện thoại ↔ desktop (proto 1)

Hợp đồng chung cho gateway desktop (`apps/shell/src/remote/`), crate `apps/core/crates/core-remote-proto`,
relay (`apps/relay`) và app `apps/mobile`. Thiết kế + lý do: `.planning/PLAN-v97-mobile.md`.
Đổi file này = tăng `proto` nếu phá tương thích.

## 1. Lớp kết nối

| Đường | Socket | Bên ngoài Noise |
|---|---|---|
| LAN | `ws://<ip LAN>:4320/remote` (mặc định port 4320, `CCM_REMOTE_PORT`) | mỗi WS binary message = 1 Noise message |
| Relay | desktop `wss://<relay>/m/<machineId>`, điện thoại `wss://<relay>/p/<machineId>?t=<token>` | desktop: `[u16 connId BE][noise msg]`; `connId = 0` = control JSON của DO `{t:"open"\|"close", conn}`; điện thoại: không có envelope (DO thêm / bỏ) |

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

`[u8 kind][u8 flags][u32 stream BE][payload]`, flags bit 0 = `more`, bit 1 = `claim` (chỉ kind 2: phím người dùng thật gõ / dán → gateway gửi `claim` cho hub; phím tự trả lời query của terminal không bật bit này). Payload mỗi khúc ≤ 60 KiB, ghép lại max 1 MiB.

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

Event `ws.snapshot {rev, tabs[]}` / `ws.patch {rev, ops[]}` (`ops` = JSON Patch RFC 6902 trên snapshot; lệch `rev` → `ws.get`).
`tabs[i] = {key, name, project, tree, zoom, folded, panes[]}`; `tree` = `{pane} | {dir, ratio, a, b}`;
`panes[j] = {key, kind, preset, agent, title, cwd, branch, state, rail, waitText, attention, subagents, sessionId, terminalId, dirty, busy}`.

### 4.4 PTY
| Lệnh | Ý nghĩa |
|---|---|
| `pty.attach {stream, sessionId, cols, rows}` | xem pane (viewer); output về kind 1 cùng `stream`; event `pty.ready {stream, cols, rows}` rồi replay |
| `pty.detach {stream}` | thôi xem |
| `pty.resize {stream, cols, rows}` | lưu cỡ của điện thoại; chỉ áp khi điện thoại đang gõ |
| kind 2 trên `stream` | gõ (ghi vào PTY); gateway gửi `claim` cho hub khi đó là phím người dùng thật, điện thoại thành người gõ, cỡ của nó áp vào PTY |

Event: `pty.resized {stream, cols, rows}`, `pty.exit {stream, code}`, `pty.overflow {stream}` (attach lại để lấy replay). Max 4 attach / thiết bị.

### 4.5 Viewer
`fs.list {root, dir}` · `fs.files {root}` · `fs.read {path}` · `fs.write {path, text, base?}` · `fs.watch {root}` / `fs.unwatch` ·
`git.status {root}` · `git.diff {root, path}` · `git.stage|unstage {root, paths[]}` · `git.commit {root, message, amend?}` · `git.discard {root, path}` ·
`log.refs {root}` · `log.page {root, query}` · `log.commit {root, hash}` · `log.diff {root, hash, path}` ·
`branch.create|delete|update|fetch|push {root, ...}` · `run.configs {root}` · `run.start|stop|reload {…}` · `run.sessions`.
Map 1–1 sang kênh `ccm:*` của core-desk (`.planning/research/v97-rust-side.md` §5), cùng Gate preview.
Event: `fs.changed {root, paths[]}`, `git.changed {root}`, `run.event {...}`.

### 4.6 Xác nhận
Event `confirm.ask {cid, title, detail, ok, cancel}` chỉ tới thiết bị gửi lệnh; lệnh `confirm.answer {cid, ok}`; quá 60 s → coi như huỷ.

## 5. Giới hạn
`ws.*` 30 / phút · `fs.write` 20 / phút · JSON 1 MiB · text prompt 2000 · max 8 kết nối chưa xác thực / IP trên LAN · thiết bị không dùng 30 ngày tự hết hạn.

## 6. Mã đóng WS
`4400` handshake lỗi · `4401` thiết bị không hợp lệ / bị thu hồi · `4426` proto cũ · `4429` rate · `4503` gateway tắt / desktop vắng trên relay · `4409` desktop bị thay bởi kết nối desktop mới (relay).

## 7. Relay (bản deploy)

- URL: `https://orrery-relay.jinvu.workers.dev` (route, giới hạn, lệnh test / deploy: `apps/relay/README.md`).
- `POST /register` / `/revoke`: body có `ts` (giây), chữ ký Ed25519 trên `"orrery-register/1" ‖ machineId ‖ ts`. Desktop WS `/m/<id>?ts&sig` ký `"orrery-machine/1" ‖ machineId ‖ ts`; `ts` mỗi lần nối lại phải lớn hơn lần trước (không thì 409).
- Desktop mất kết nối → mọi điện thoại của máy đó bị đóng `4503`.
