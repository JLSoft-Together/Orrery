# Orrery Remote — dùng điện thoại điều khiển Orrery trên máy tính

Orrery Remote là app điện thoại (Flutter) để xem và điều khiển Workspace của Orrery trên máy tính: terminal, pane, file, Changes, Git Log, Run, danh sách session. Mọi pane vẫn chạy trên máy tính; điện thoại chỉ là màn hình + bàn phím thứ 2. Máy tính phải đang chạy Orrery (ẩn ở tray vẫn được).

Hợp đồng kỹ thuật: `docs/remote-integration.md`. Ngoại lệ rule bảo mật: `docs/claude-code-integration.md` §11as.

## 1. Tình trạng hiện tại

| Phần | Trạng thái |
|---|---|
| Gateway trên máy (Windows, Tauri) | Có, đã test tự động |
| Relay Cloudflare (`https://orrery-relay.jinvu.workers.dev`) | Có, đã deploy; bản có "đóng đúng 1 điện thoại" chưa deploy |
| App Android | Có, build APK được; chạy được trên 1 máy thật (Galaxy A21s) |
| App iOS | **Chưa build**, chưa chạy thử (cần Mac + TestFlight) |
| Push thông báo native (APNs / FCM) | **Chưa có** (chờ Firebase project). Dùng Telegram / ntfy |
| Quét QR bằng camera thật, mDNS thật, hiệu năng terminal trên máy thật | Chưa kiểm xong |

## 2. Bật và ghép điện thoại

1. Orrery → Settings → **Điều khiển từ xa** → bật công tắc. Mặc định tắt.
2. Bấm **Thêm điện thoại**: hiện QR, đếm ngược **5 phút**, dùng **1 lần**. Mã chỉ có hiệu lực khi QR đang hiện.
3. Mở Orrery Remote trên điện thoại → thêm máy → quét QR. Điện thoại và máy phải cùng Wi-Fi khi ghép (ghép chỉ đi qua LAN).
4. Orrery trên máy hỏi xác nhận "cho phép điện thoại này điều khiển?" → chọn Cho phép. Từ chối hoặc hết 5 phút thì quét lại QR mới.
5. Sai mã quá 5 lần / phút (theo IP) thì bị chặn tạm.

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
- **macOS**: hộp "cho phép nhận kết nối mạng đến" → Allow. App iOS cũng sẽ hỏi quyền **Local Network** khi tìm máy.
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
- Android App Links tự mở app cần relay trả `assetlinks.json`; chưa kiểm trên máy thật, và lệnh deploy relay kèm cert debug chưa chạy. iOS: chưa làm.

## 8. Giới hạn

| Mục | Giá trị |
|---|---|
| Port LAN | 4320 |
| Mã QR ghép | 5 phút, 1 lần |
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

**iOS (TestFlight, chưa làm)**: cần Mac, tài khoản Apple Developer, bật capability Associated Domains; quyền Local Network (`NSBonjourServices`) đã khai trong project. Điền `APPLE_APP_ID` (Team ID) vào `apps/relay/wrangler.jsonc` rồi deploy relay. Chưa có bản build iOS nào chạy thử. App chỉ cài qua APK + TestFlight, chưa lên store.

## 12. Dữ liệu và bảo mật

- Trên máy: `<data>/remote/config.json` + `devices.json` (key bí mật nằm trong keyring OS, không nằm trong 2 file này).
- Trên điện thoại: key và token relay trong Keychain / Keystore (flutter_secure_storage); cache snapshot trong thư mục cache của app, không backup.
- Relay không log, không đọc nội dung.
- Env, secret, app key, token PTY không bao giờ ra điện thoại.
