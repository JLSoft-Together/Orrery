# Orrery Remote — dùng điện thoại điều khiển Orrery trên máy tính

Orrery Remote là app điện thoại (Flutter) để xem và điều khiển Workspace của Orrery trên máy tính: terminal, pane, file, Changes, Git Log, Run, danh sách session. Mọi pane vẫn chạy trên máy tính; điện thoại chỉ là màn hình + bàn phím thứ 2. Máy tính phải đang chạy Orrery (ẩn ở tray vẫn được).

Hợp đồng kỹ thuật: `docs/remote-integration.md`. Ngoại lệ rule bảo mật: `docs/claude-code-integration.md` §11as.

## 1. Tình trạng hiện tại

| Phần | Trạng thái |
|---|---|
| Gateway trên máy (Windows, Tauri) | Có, đã test tự động |
| Relay Cloudflare (`https://orrery-relay.jinvu.workers.dev`) | Có, đã deploy; bản có "đóng đúng 1 điện thoại" chưa deploy |
| App Android | Có, build APK được; chạy được trên 1 máy thật (Galaxy A21s) |
| iPhone / iPad | Web app (PWA) tải từ relay, đang làm ở v98. Không có app iOS native, không cần tài khoản Apple Developer |
| Push thông báo native (APNs / FCM) | Phía điện thoại có (switch trong danh sách máy, Firebase do người dùng tự thiết lập trên máy tính, options gửi qua `push.config`); chạm thông báo mở thẳng pane. iPhone dùng Web Push riêng của web app (v98, §13, mã hoá đầu cuối, có nội dung). Chưa kiểm trên iPhone thật. Dùng thêm Telegram / ntfy |
| Quét QR bằng camera thật, mDNS thật, hiệu năng terminal trên máy thật | Chưa kiểm xong |

## 2. Bật và ghép điện thoại

1. Orrery → Settings → **Điều khiển từ xa** → bật công tắc. Mặc định tắt.
2. Bấm **Thêm điện thoại**: hiện QR, đếm ngược **2 phút**, dùng **1 lần**. Mã chỉ có hiệu lực khi QR đang hiện. Dưới QR ghi QR có relay hay không: relay đang online lúc bấm thì QR mang thêm vé ghép relay (điện thoại khác mạng vẫn ghép được), không thì "Chỉ LAN".
3. Mở Orrery Remote trên điện thoại → thêm máy → quét QR. App thử mọi IP LAN trong QR (và mDNS) khoảng **3 giây**; không được (khác mạng, 4G) thì tự ghép qua relay bằng vé trong QR. Màn ghép hiện đường đang dùng (Wi-Fi / relay). QR cũ (v1, trước v98) vẫn quét được nhưng chỉ ghép qua LAN.
4. Orrery trên máy hỏi xác nhận "Ghép «tên điện thoại» (android) từ IP … [quốc gia] qua LAN / relay?" → chọn Cho phép. Xem kỹ IP: IP lạ hoặc bạn không vừa quét QR thì chọn Huỷ. Từ chối hoặc hết 2 phút thì quét lại QR mới.
5. Một điện thoại / IP sai mã **5 lần** thì IP đó không dùng được QR này nữa (QR vẫn dùng được cho máy khác); tổng 20 lần sai trên 1 QR thì QR bị huỷ (bấm Mã mới). Ngoài ra sai quá 5 lần / phút (theo IP) thì bị chặn tạm, và quá nhiều lỗi thì IP bị chặn hẳn (mục 12).

Một app quản lý nhiều máy tính (Windows + Mac). Tên máy hiện trong app là tên máy đã đặt ở Orrery.

## 3. Kết nối: LAN và relay

| | LAN trực tiếp | Relay |
|---|---|---|
| Khi nào | Cùng Wi-Fi | Khác mạng (4G, Wi-Fi khác) |
| Đường đi | `ws://<IP máy>:4320/remote`, tìm máy bằng mDNS `_orrery._tcp` | Máy và điện thoại đều gọi ra Cloudflare Worker; không mở port router |
| Mã hoá | Noise đầu-cuối | Noise đầu-cuối (relay chỉ thấy byte mã hoá + `machineId`) |

App thử LAN trước, **1,5 giây** không được thì sang relay. Đổi mạng giữa chừng (Wi-Fi sang 4G) thì app nối lại qua relay, attach lại pane, terminal lấy lại output cũ (replay 256 KiB). Nối lại có backoff 1 / 2 / 4 / 8 / 16 / 30 giây.

Trạng thái relay hiện ở Settings: off / connecting / online / error (icon + chữ + màu).

Port LAN mặc định **4320**; đổi bằng biến môi trường `CCM_REMOTE_PORT`.

## 4. Firewall

Lần đầu bật, hệ điều hành có thể hỏi cho phép Orrery nhận kết nối.

- **Windows**: chọn **Mạng riêng tư**, cho phép. Lỡ chọn sai: Windows Security → Firewall → Allow an app → Orrery → tick Private. Cần mở TCP **4320**.
- **macOS**: hộp "cho phép nhận kết nối mạng đến" → Allow.
- Settings → Điều khiển từ xa có dòng nhắc firewall kèm port đang dùng. Không vào được qua LAN thì relay vẫn dùng được (máy cần có Internet).

## 5. Thu hồi, hết hạn, ngắt

- Settings → **Điện thoại đã ghép nối**: xoá từng thiết bị (hỏi xác nhận ngay trên dòng) hoặc **Ngắt mọi điện thoại**.
- Thiết bị **30 ngày** không dùng tự hết hạn, phải ghép lại.
- Token relay hạn 30 ngày, tự gia hạn khi còn dưới 7 ngày và điện thoại kết nối.
- Thu hồi xong điện thoại bị đóng kết nối ngay (mã `4401`).
- Tối đa **16 phiên** cùng lúc, tối đa **32 thiết bị** đã ghép, mỗi điện thoại attach tối đa **4 pane** cùng lúc.

## 6. Giữ máy thức

Settings có công tắc **Giữ máy thức khi bật điều khiển từ xa**, mặc định **bật**. Máy ngủ thì điện thoại mất kết nối; màn hình vẫn tắt được. Giữ thức chạy cả khi dùng pin. Tắt remote hoặc thoát Orrery thì thôi giữ.

Chưa xử lý: macOS App Nap. Tham số giảm tốc nền của cửa sổ chỉ áp khi remote đã bật lúc mở app (bật giữa chừng cần khởi động lại Orrery; Settings chưa có chữ báo).

## 7. Mở pane từ Telegram / ntfy

Tin của 1 pane (chờ, hỏi, xin quyền, xong việc) có nút **Mở trên điện thoại** (Telegram) hoặc bấm vào thông báo (ntfy). Link dạng `https://orrery-relay.jinvu.workers.dev/o/<machineId>/<paneKey>` hoặc `orrery-remote://m/<máy>/p/<pane>`; app mở thẳng pane đó.

- Link chỉ có khi remote đang chạy, đã ghép ít nhất 1 điện thoại, và pane còn mở. Tin `job` / `limit` / `disk` / `note` không có link.
- Máy chưa ghép trong app: báo "Chưa ghép máy này". Pane đã đóng: báo pane không còn mở.
- Android App Links tự mở app cần relay trả `assetlinks.json`; chưa kiểm trên máy thật, và lệnh deploy relay kèm cert debug chưa chạy. iPhone dùng web app nên không dùng App Links.

## 8. Giới hạn

| Mục | Giá trị |
|---|---|
| Port LAN | 4320 |
| Mã QR ghép | 2 phút, 1 lần |
| Lỗi trước khi bị chặn IP | 20 / 10 phút → chặn 1 giờ, tái phạm 24 giờ, rồi 7 ngày |
| Kết nối relay cùng 1 IP | max 4 |
| Thiết bị không dùng | 30 ngày thì hết hạn |
| Phiên cùng lúc | max 16 |
| Attach pane / điện thoại | max 4 |
| Xác nhận trên điện thoại | 60 giây, max 4 xác nhận chờ |
| Prompt gửi từ điện thoại | max 2000 ký tự; pane không idle thì chỉ gõ sẵn, không submit |
| Lệnh `ws.*` | 30 / phút (hợp đồng); gateway đếm chung max 240 lệnh / phút / phiên |
| Ghi file | 20 / phút, chỉ ghi đè file có sẵn, báo conflict nếu file đổi trên đĩa |
| Kích thước tin | 1 MiB |
| Không có dữ liệu trên LAN | 45 giây thì máy đóng phiên (app gửi ping ≤ 30 giây) |
| Relay Free plan | 100k request Durable Object / ngày; vượt thì lên Paid 5 USD / tháng |

Cỡ terminal: bên nào **đang gõ** thì cỡ bên đó áp vào terminal. Điện thoại chỉ xem thì giữ cỡ máy tính và cuộn ngang trong khung terminal. Điện thoại và máy tính focus riêng: lệnh từ điện thoại không đổi tab / pane đang xem trên máy. Không khoá app bằng vân tay; dựa vào khoá màn hình điện thoại.

Thao tác nguy hiểm (xoá branch, bỏ thay đổi, push, commit amend, chạy config lạ, cài đặt lại bản Android) hỏi xác nhận **trên điện thoại**, không hiện hộp trên máy. Ghép thiết bị mới vẫn xác nhận trên máy.

## 9. Máy tính thấy gì

- Chip điện thoại trên pane điện thoại vừa ra lệnh hoặc đang giữ cỡ (60 giây).
- Settings → "Lệnh từ điện thoại": 50 dòng gần nhất; thêm ghi chú trong inbox của session.
- Nút Ngắt mọi điện thoại.

## 10. Lỗi thường gặp

| Triệu chứng | Nguyên nhân hay gặp | Cách xử |
|---|---|---|
| Quét QR báo hết hạn | Quá 5 phút hoặc đã dùng | Bấm Thêm điện thoại lấy QR mới |
| Ghép không thấy máy | Khác Wi-Fi, hoặc router chặn mDNS (client isolation) | Cùng Wi-Fi, tắt AP isolation; QR có sẵn IP máy |
| LAN không vào được | Firewall chặn 4320 | Mục 4; hoặc dùng relay |
| Relay `error` | Máy không ra Internet, hoặc relay giới hạn | Kiểm mạng; `https://orrery-relay.jinvu.workers.dev/health` trả `ok` |
| Báo "máy đang khởi động lại" | Orrery thoát / reload renderer | Chờ, app tự nối lại |
| Settings báo lỗi keyring | OS không cho lưu key | Remote không bật được; mở khoá keyring / Credential Manager rồi thử lại |
| Điện thoại bị đóng `4401` | Thiết bị bị thu hồi hoặc hết 30 ngày | Ghép lại |
| Điện thoại bị đóng `4426` | App quá cũ so với máy | Cài bản app mới |
| Sửa file báo conflict | File đổi trên máy sau khi mở | Tải lại, không ghi đè |
| Gõ tiếng Việt lỗi trong terminal | Bàn phím gửi composition | Dùng khung nhập dòng (mặc định), gửi cả dòng |

## 11. Cài app

**Android (APK, chưa lên store)**: `cd apps/mobile && flutter build apk` (hoặc `--profile` để đo hiệu năng), file ở `apps/mobile/build/app/outputs/flutter-apk/`; cài bằng `adb install -r <file>.apk`. Package `dev.ccm.orrery.remote`, minSdk 24. Cần Rust toolchain (cargokit build 3 ABI).

**iPhone / iPad (web app, đang làm ở v98)**: dùng web app tải từ relay của bạn, không có app iOS native, không cần Mac hay tài khoản Apple Developer. Các bước cài web app lên màn hình chính và cài CA cho LAN sẽ bổ sung ở v98.

### Web app iPhone và relay: đánh đổi

- Web app được relay phát: code chạy trên iPhone đến từ relay Cloudflare của chính bạn, không đến từ store.
- Ai kiểm soát tài khoản Cloudflare đó có thể sửa code web app. Bật 2FA cho tài khoản Cloudflare, giữ API token kín.
- Mã hoá đầu cuối vẫn là Noise giữa iPhone và máy tính. Nhưng code chạy Noise trên iPhone do relay phát, nên tin relay = tin code đó.

## 12. Dữ liệu và bảo mật

- Trên máy: `<data>/remote/config.json` + `devices.json` (key bí mật nằm trong keyring OS, không nằm trong 2 file này).
- Trên điện thoại: key và token relay trong Keychain / Keystore (flutter_secure_storage); cache snapshot trong thư mục cache của app, không backup.
- Relay không log, không đọc nội dung.
- Env, secret, app key, token PTY không bao giờ ra điện thoại.

### Chặn IP, danh sách tin cậy, nhật ký, cảnh báo (v98)

- **Tự chặn**: một IP gây 20 lỗi trong 10 phút (bắt tay sai, token sai / hết hạn / bị thu hồi, mã hoặc vé ghép sai, thử quá nhanh, gói tin sai / quá cỡ) bị chặn **1 giờ**; bị chặn lại trong 7 ngày sau đó thì **24 giờ**, rồi **7 ngày**. Áp ở cả cổng LAN của máy và relay. IPv6 bị chặn theo cả dải /64.
- **Không bao giờ tự chặn**: IP trong **danh sách tin cậy**, và IP đang có (hoặc vừa có trong 10 phút) phiên hợp lệ của điện thoại đã ghép — tức là cả các máy khác sau cùng router / NAT (IPv4) hoặc cùng dải /64 (IPv6) với điện thoại đó. Trên relay, điện thoại đã ghép vẫn vào được dù IP của nó bị tự chặn nếu nó vừa mở phiên thành công trong 24 giờ qua (ví dụ dùng chung IP 4G với kẻ dò); chỉ có token mà không có khoá của điện thoại thì không được miễn.
- **Xem và gỡ**: Settings → Điều khiển từ xa → **IP bị chặn**: IP / dải — IPv6 theo /64, nơi chặn (LAN / relay / chặn tay), lý do, số lỗi, lúc chặn, hết hạn, bậc. **Gỡ** từng IP hoặc **Gỡ hết** (hỏi lại). Máy tính luôn gỡ được vì đây là thao tác trên chính máy, không cần điện thoại nối được; chặn chỉ áp cho điện thoại, không bao giờ áp cho kết nối relay của máy tính.
- Huỷ mã ghép: một IP sai mã 5 lần thì IP đó không dùng được mã đang hiện; tổng 20 lần sai thì mã bị huỷ. Trong cùng Wi-Fi, 4 máy là đủ huỷ mã của bạn — chỉ làm hỏng lượt ghép đó, bấm Mã mới.
- **Chặn tay**: nhập IP hoặc dải CIDR (IPv4 tối thiểu /8, IPv6 tối thiểu /16), chọn 1 giờ / 1 ngày / 7 ngày / 30 ngày / vĩnh viễn. Không chặn được dải đang có điện thoại nối (thu hồi điện thoại thay vào đó). Chặn tay áp cho mọi kết nối, kể cả điện thoại đã ghép, và máy tính tự áp cả khi relay chưa cập nhật. Gỡ 1 IP không gỡ dải chặn tay rộng hơn chứa nó.
- **Tin cậy**: thêm mạng nhà / văn phòng (IP hoặc CIDR, max 32). Danh sách tin cậy và chặn tay lưu trên máy (`<data>/remote/guard.json`) và tự đồng bộ lên relay mỗi lần nối.
- **Nhật ký truy cập**: 200 lần nối / lỗi gần nhất (giờ, IP, quốc gia nếu qua relay, đường, thiết bị, kết quả, lý do). Không ghi mã ghép, vé, token, khoá. Nhật ký chỉ nằm trong bộ nhớ, mất khi tắt Orrery.
- **Cảnh báo** (thông báo trên máy + Telegram nếu đã bật Telegram và bật cảnh báo + thông báo đẩy tới các điện thoại khác đã bật push): thiết bị mới vừa ghép; có IP bị chặn (gộp, tối đa 1 lần / giờ); điện thoại đã ghép nối qua relay từ quốc gia chưa thấy (tối đa 1 lần / giờ / thiết bị). Tin Telegram (có thể vào nhóm có người khác) và thông báo đẩy không chứa IP hay tên thiết bị; chi tiết xem trên máy tính.
- Relay dựng trước v98: bấm **Cập nhật relay** để có chặn IP và giới hạn 4 kết nối / IP ở relay; chưa cập nhật thì chỉ cổng LAN chặn.

## 13. Dựng relay và thông báo đẩy trong Settings

Settings → Điều khiển từ xa có hai khối, không cần đọc docs hay chạy lệnh.

**Relay (ngoài mạng nhà)** — dùng tài khoản Cloudflare miễn phí của bạn:

1. Bấm "Mở trang tạo token", bấm Create Token → Start from scratch (Custom), tick `Account · Workers Scripts · Edit` và `Account · Account Settings · Read` trên account muốn dùng.
2. Dán token vào ô (ẩn ký tự) rồi "Kiểm tra". Token được cất trong Keychain / Credential Manager của máy, không hiện lại trên màn hình. Mỗi lần mở Settings, nếu muốn chọn account hoặc cập nhật relay thì dán lại token.
3. Chọn account (chỉ có một thì đã chọn sẵn) rồi "Dựng relay". Tiến độ từng bước hiện ngay: kiểm token, địa chỉ workers.dev, tải relay, bật địa chỉ, chờ relay phản hồi. Rời trang giữa chừng không dừng việc dựng; mở lại sẽ thấy tiến độ.
4. Xong: địa chỉ relay (nút Sao chép), account, và "Cập nhật relay" khi bản kèm app mới hơn. Nếu relay chưa phản hồi ngay, đó là DNS đang lan, chờ vài phút.

"Gỡ" hỏi hai kiểu: chỉ gỡ khỏi máy này, hoặc xoá luôn Worker trên Cloudflare (app hỏi xác nhận thêm một lần). Chưa xoá Worker thì nó vẫn nằm trong account của bạn.

Điện thoại ghép nối trước khi có relay cần ghép lại để dùng relay; điện thoại đã ghép với relay này không cần làm gì thêm.

**Thông báo đẩy** — dùng project Firebase của bạn:

1. Firebase console → Add app → Android, package `dev.ccm.orrery.remote` → tải `google-services.json`.
2. Project settings → Service accounts → Generate new private key → tải file khoá.
3. Thả cả hai file vào vùng thả (hoặc "chọn file"). App nhận diện theo nội dung nên không cần đúng tên file; mỗi loại hiện dấu ✓ kèm tên project. Hai file khác project, file quá 64 KB hay không phải JSON Firebase đều bị báo ngay trên màn hình.
4. "Cài thông báo đẩy". Xong: project id, số điện thoại đã đăng ký, "Gửi thử" (chọn điện thoại khi có nhiều máy), "Gỡ".

Nội dung hai file chỉ nằm trong bộ nhớ của trang đến lúc gửi cho app, rồi app cất khoá trong Keychain; không lưu vào localStorage hay log.

**Thông báo trên iPhone / iPad (web app, v98)** — không cần Firebase. Mỗi máy tính tự tạo một khoá VAPID cất trong Keychain / Credential Manager và gửi Web Push thẳng tới Apple (hoặc Edge / Chrome / Firefox nếu mở web app trên máy khác), không qua relay:

1. iPhone / iPad: mở web app từ relay, Chia sẻ → Thêm vào Màn hình chính, rồi mở từ biểu tượng đó (Safari chỉ cho bật thông báo khi chạy từ Màn hình chính).
2. Cài đặt → Thông báo → dòng "Thông báo đẩy" → "Bật" → cho phép. Mỗi máy tính đang nối được đăng ký riêng ("Đang bật · 2 máy tính").
3. Chuông trên thẻ từng máy ở Home tắt / bật thông báo từ máy đó; 3 nhóm (chờ / xong / lỗi) và "Kèm nội dung" áp cho mọi máy.

Thông báo có nội dung (ngoại lệ thứ 7 trong `CLAUDE.md`): tiêu đề `agent · session · dự án`, thân là câu hỏi / quyền đang chờ, "Xong sau 12m" hoặc lỗi; secret bị che, cắt 120 ký tự, hiện cả trên màn khoá. Tắt "Kèm nội dung" thì về chữ cố định. Nội dung Web Push được mã hoá đầu cuối (RFC 8291): Apple / Microsoft / Google chỉ thấy gói mã hoá. Thông báo Android (FCM) vẫn là chữ cố định cho tới khi có mã hoá bằng khoá thiết bị. Không push khi app đang mở trên điện thoại; chạm thông báo mở thẳng pane.

Trên máy tính: Settings → Điều khiển từ xa → khối thiết bị → nút **Thông báo điện thoại** (chuông, "Bật 2 / 3") mở danh sách điện thoại đã ghép; mỗi dòng có trạng thái (biểu tượng + chữ + màu: đang bật · Web Push / FCM · kèm nội dung, tắt trên máy tính này, chưa bật trên điện thoại) và công tắc bật / tắt push cho riêng điện thoại đó, không cần huỷ ghép. Chi tiết giao thức: `docs/remote-integration.md` §7.6.

Đổi tên hiển thị: desktop sửa tên điện thoại đã ghép ngay trong list (alias lưu trong `devices.json`, giữ tên gốc điện thoại gửi), điện thoại đổi tên máy tính cục bộ; hai bên không đồng bộ tên. Điện thoại đổi tên session bằng lệnh `terminal.alias` có sẵn, cùng alias với desktop.

## 14. Khoá app điện thoại

App Orrery Remote khoá bằng vân tay / khuôn mặt hoặc PIN / mẫu hình / mật khẩu khoá màn hình của máy (`local_auth`), lớp 2 cạnh khoá ghép Noise. Bật mặc định; lần đầu mở app phải xác thực, sau đó phiên mở khoá có hạn mặc định 1 giờ (Mỗi lần mở · 15 phút · 1 giờ · 4 giờ, chỉnh ở nút khoá trên màn danh sách máy; đổi hoặc tắt phải xác thực trước).

Khi khoá app đóng mọi kết nối tới máy tính (LAN và relay), không mở kết nối mới và không hiện session / terminal; tap thông báo hoặc deep link lúc đang khoá được giữ lại, xác thực xong mới mở. Android đặt `FLAG_SECURE` nên recent apps không lộ nội dung. Mốc mở khoá chỉ nằm trong RAM (đồng hồ đơn điệu + giờ máy, lấy số lớn hơn nên chỉnh giờ lùi không kéo dài phiên); process chết hoặc reboot thì khoá lại. Máy chưa đặt khoá màn hình: màn khoá báo cần đặt, có nút mở cài đặt bảo mật; chỉ ở trạng thái đó mới có nút "Tắt khoá app" kèm cảnh báo.
