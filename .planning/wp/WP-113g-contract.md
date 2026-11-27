# WP-113g contract: desktop push payloads (v113 F5 + F6)

Nguồn sự thật cho WP-113g2 (phone + PWA). Khớp parser phone 113k: `apps/mobile/lib/push/waiting_update.dart` (`w`, `wl`, `k:"clear"`) và `push_service.dart::_register` (`pauseUntil`).

## Quy tắc chung

- FCM `data` toàn string. Số gửi dạng chuỗi số thập phân (`"w":"2"`).
- Ngoài seal (luôn đọc được, không nhạy cảm): `m` (machine), `k` (kind), `p` (pane, nếu có), `w` (số pane đang chờ, 0..999), `t0` (chỉ `running`: mốc bắt đầu, epoch ms).
- Trong seal (chỉ khi "Include content" bật VÀ seal bật; JSON plaintext của `e`): `t`, `b`, `p`, và thêm `wl`, `menu`. Content tắt hoặc seal tắt: KHÔNG có `wl`, KHÔNG có `menu`.
- Web Push (đã mã hoá E2E theo RFC 8291): cùng khoá như seal, nằm ngay trong JSON: `title`, `body`, `m`, `k`, `tag`, `p`, `w`, `t0`, và `wl`, `menu` chỉ khi content bật.
- Nhãn (`wl[]`, `menu.items[].label`): đi qua `mask_text`, bỏ ký tự điều khiển / bidi, gộp khoảng trắng, cắt <= 24 ký tự (có `…`).
- Tag thay thế: `"<m>:<p>"` nếu có pane, không thì `"<m>:<k>"`. FCM notification có `android.notification.tag` cùng giá trị cho push có pane (hệ thống tự thay); phone tự dựng local notification dùng đúng tag này (`PushNotice.tag`).
- Quota theo thiết bị: push hỏi (`attention`/`question`/`permission`/`error`) `PER_HOUR=30`; push thông tin (`done`/`running`/`open`/`clear`) `PER_HOUR_INFO=20` riêng, không chiếm chỗ của push hỏi.
- `pauseUntil` (xem cuối) mute mọi group tới mốc; không chặn `security`, `test`, `open`, `clear`.

## Kind

| `k` | Khi nào | Có notification (FCM plain) | Kênh Android | Web Push |
|---|---|---|---|---|
| `attention` / `question` / `permission` | session working sang waiting | có | `attention` | có |
| `error` | working / waiting sang error | có | `attention` | có |
| `done` | working sang idle | có | `attention` | có |
| `running` | session working liên tục >= 120s (1 lần / lượt) | có | `running` (thấp, không rung) | có |
| `open` | chủ nhân bấm "Mở trên điện thoại" ở pane head | có | `attention` | có |
| `clear` | số pane chờ về 0 sau khi phone đã nhận `w > 0` | KHÔNG (data-only, `priority: normal`) | - | KHÔNG gửi |
| `security`, `test` | như cũ | có | `attention` | như cũ |

Phone phải: `clear` không hiện notification, ghi widget về 0 (parser có sẵn). Notification `running` nên đặt `timeoutAfter` ~ 6h phòng khi push `done` thay thế không tới (hết quota / session bị đóng).

## `w` và `wl`

- Mọi push trừ `test` và `security` có `w`: số terminal đang `waiting` lúc GỬI (tính lại trước khi gửi, không lấy lúc xếp hàng). `w` có thể là `"0"` (vd push `done`).
- `wl` (trong seal / Web Push payload): <= 2 nhãn, theo thứ tự pane chờ lâu nhất trước, dạng mảng JSON `["Fix login","orrery"]` (phone `parseWaitingLabels` đọc được mảng JSON). Nhãn = tên session, thiếu thì tab, thiếu thì project, thiếu thì tên agent. Không có `wl` khi `w = 0`.
- `clear`: `w = "0"`. Chỉ gửi tới điện thoại FCM đã từng nhận push có `w > 0` kể từ lần clear trước. Nếu lúc gửi số chờ lại > 0 thì desktop bỏ push clear.

## `menu` (P1, trong seal / Web Push payload)

```json
"menu": { "sig": "1a2b3c4d", "items": [ { "index": 1, "label": "Yes" }, { "index": 2, "label": "Yes, and don't ask again" }, { "index": 3, "label": "No" } ] }
```

- Chỉ có trên push `attention` / `question` / `permission` của pane đang hiện menu trong workspace summary (cùng nguồn đang cấp `ws.menu` cho chat), có `content` bật và (FCM) seal bật.
- <= 3 mục, bỏ mục `other` / `submit`; bỏ cả menu nếu là menu chọn nhiều ô (có `checked`).
- Phone bấm action: mở app, qua khoá app, rồi gửi `ws.menu {paneKey: p, sig, index}`. Sig lệch thì desktop trả lỗi (hiện là `busy`; `stale` ghi ở báo cáo WP-113g1, chưa đổi vì file nằm ngoài phạm vi).

## `running` (P3)

FCM plain:

```json
{ "message": { "token": "...", "notification": { "title": "Agent is working", "body": "A session has been running for a while. Tap to open it." }, "data": { "m": "M1", "k": "running", "p": "pane-1", "w": "0", "t0": "1764150000000" }, "android": { "priority": "high", "notification": { "channel_id": "running", "tag": "M1:pane-1" } } } }
```

Seal (content + seal bật) data-only: `{ "v":"2", "m":"M1", "k":"running", "e":"<sealed>", "w":"0", "t0":"1764150000000" }`, plaintext `{ "t":"Claude, Fix login, orrery", "b":"Running for 2m", "p":"pane-1", "wl":[...] }`.

- `t0` = mốc desktop thấy session bắt đầu working (epoch ms); phone dùng làm chronometer. Session đã chạy sẵn khi Orrery khởi động: mốc = lúc Orrery thấy.
- Xong: push `done` (hoặc `attention` / `error`) cùng tag, phone thay notification. Tối đa 2 push mỗi lượt (`running` + kết thúc).

## `open` (P6)

```json
{ "message": { "token": "...", "notification": { "title": "Opened from your computer", "body": "Tap to open this pane." }, "data": { "m": "M1", "k": "open", "p": "pane-1", "w": "1" }, "android": { "priority": "high", "notification": { "channel_id": "attention", "tag": "M1:pane-1" } } } }
```

- Link mở: `orrery-remote://m/<m>/p/<p>` dựng bằng `pushLink` có sẵn từ `m`, `p`.
- Gửi tới mọi điện thoại bật push, kể cả khi đang kết nối (hành động chủ nhân chọn).

## `clear`

```json
{ "message": { "token": "...", "data": { "m": "M1", "k": "clear", "w": "0" }, "android": { "priority": "normal" } } }
```

Không có `notification`, không có `p`, không seal (không chứa nội dung).

## `push.register`

Thêm field tuỳ chọn `pauseUntil` (epoch ms, số nguyên). Desktop nhận nếu `now < pauseUntil <= now + 24h`, ngược lại coi như không tạm dừng (gửi register không có `pauseUntil` = bật lại). Lưu theo thiết bị trong RAM (phone gửi lại ở mỗi lần kết nối), kiểm lại lúc gửi từng push. Trong lúc tạm dừng desktop không xếp / không gửi push nhóm `waiting`, `done`, `error`, `running`; vẫn gửi `security`, `test`, `open`, `clear`. Hết mốc tự bật lại, không cần phone gửi gì.

## `ccm:remote-send-pane`

Arg `{ "paneKey": "<key>" }` (hoặc chuỗi key). Reply `{ "ok": true, "sent": <số phone xếp hàng> }` hoặc `{ "ok": false, "error": "off" | "invalid" | "notFound" | "none" | "limit" }`.
