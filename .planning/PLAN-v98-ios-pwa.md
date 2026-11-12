# PLAN v98 — Orrery Remote web app (PWA) cho iPhone / iPad + ghép 2 đường + push có nội dung

Chốt qua grill với chủ nhân 2026-11-12. Dùng nội bộ: không store, không ký, không Apple Developer (memory `internal-distribution`).

## Quyết định đã chốt

| # | Chủ đề | Quyết định |
|---|---|---|
| 1 | Công nghệ | **Vue 3 + Vite + Pinia + vue-i18n**, workspace mới `apps/pwa`; dùng `packages/shared` (protocol), xterm.js như desktop, token màu Orrery; UI + motion theo mockup `docs/design/v97-mobile-ui` |
| 2 | Mã hoá | Noise = **Rust `core-remote-proto` → wasm** (wasm-bindgen), cùng code với desktop + Android |
| 3 | Host | Web app nằm trên relay của chủ nhân `https://<relay>/app/`; **chấp nhận** relay chủ nhân tin được, nhắc bật 2FA Cloudflare, ghi đánh đổi vào docs |
| 4 | Relay | PC + Mac **dùng chung 1 relay** (`orrery-relay.jlst.workers.dev`) |
| 5 | LAN iPhone | Web app tải từ relay; **LAN qua CA riêng**: mỗi máy 1 CA (khoá trong keychain máy đó), Settings → Điện thoại có QR tải hồ sơ `.mobileconfig`; desktop mở `wss://` LAN bằng chứng chỉ ký từ CA; mất LAN tự về relay. Cần spike Safari |
| 6 | Chọn đường | Mỗi máy có select **"Kết nối: Tự động / Chỉ LAN / Chỉ relay"** (mặc định Tự động). IP LAN desktop tự báo qua relay mỗi lần nối, người dùng không nhập gì |
| 7 | Ghép máy | **Cả 2 đường**: QR mới chứa LAN + relay + mã 1 lần; thử LAN ~3 s, lỗi (khác mạng / chưa cài CA) thì ghép qua relay. **Android dùng luồng mới cùng đợt**, QR cũ vẫn đọc được |
| 8 | Khoá app | Face ID / Touch ID qua **WebAuthn**, mặc định 1 giờ; máy không hỗ trợ thì báo và bỏ khoá |
| 9 | Phạm vi | **Đủ tính năng 1 lượt**, ngang Android: ghép, Home nhiều máy, Monitor A/B, Workspace A/B, terminal (Ctrl/Alt dính), viewer file/diff/git/run, Cài đặt, khoá, push, đổi tên |
| 10 | Quyền | **Ngang Android** (điều khiển từ xa mặc định tắt trên desktop, thao tác nguy hiểm xác nhận trên điện thoại) |
| 11 | Cache | Lưu snapshot cuối trong IndexedDB (sau khoá), thu hồi thiết bị thì xoá sạch |
| 12 | iPad | Hỗ trợ, ≥ 768 px chia đôi như tablet Android |
| 13 | iOS native | **Xoá `apps/mobile/ios/`**; tối nay chủ nhân chỉ test Mac desktop + A21s |
| 14 | Cập nhật | Bản web đóng gói trong app desktop, "Cập nhật relay" đẩy luôn web mới; web app thấy bản mới / giao thức lệch thì hiện thanh "Có bản mới — tải lại", không tự reload |
| 15 | Push iPhone | **Web Push (VAPID)** từ desktop, khoá VAPID mỗi máy cất keychain; 404 / 410 = xoá subscription |
| 16 | Push có nội dung | **Cả iPhone + Android**: agent · tên session · project; thân = câu hỏi / quyền đang chờ, "Xong sau 12m" hoặc lỗi; lọc secret, cắt 120 ký tự. **Hiện đủ trên màn khoá** |
| 17 | Android FCM | **Mã hoá bằng khoá thiết bị**, gửi data message; app giải mã rồi tự hiện thông báo; giải mã lỗi thì hiện bản không nội dung. Google chỉ thấy gói mã hoá |
| 18 | Ngoại lệ | Push có nội dung = **ngoại lệ thứ 7** của quy tắc không lộ nội dung session → ghi vào `CLAUDE.md` + docs; Cài đặt → Thông báo có công tắc **"Kèm nội dung"** (mặc định bật), tắt thì về thông báo cố định |
| 19 | Bật / tắt nhanh | **Chuông trên thẻ từng máy ở Home điện thoại** (tắt push từ máy đó) + **nút chuông trên desktop** mở danh sách điện thoại, bật / tắt từng cái |
| 20 | Giữ nguyên | 3 nhóm (chờ / xong / lỗi), 1 pane 30 s, 30 lần / giờ, không push khi app đang mở, chạm mở thẳng pane, Telegram giữ nguyên |
| 21 | Test | Vitest + **Playwright WebKit** (E2E với dev app + relay thật) + chủ nhân test iPhone / iPad thật cuối đợt |
| 22 | Cách làm | Như v97: chia gói giao subagent, main kiểm + review, **commit + push dần**, nhánh `feat/v98-pwa` từ `feat/v97-mobile`, PROGRESS + HANDOFF |
| 23 | Chặn IP tự động | Relay + cổng LAN desktop: **20 lỗi / 10 phút** (bắt tay Noise sai, token sai, mã ghép sai, bị 429, khung sai) → chặn **1 giờ → 24 giờ → 7 ngày** nếu tái phạm. IP thiết bị đã ghép đang nối hợp lệ và IP trong danh sách tin cậy không bao giờ bị chặn tự động |
| 24 | Danh sách IP | Desktop Settings → Điều khiển từ xa: xem (IP, nguồn relay / LAN, lý do, số lần, lúc chặn, hết hạn), **gỡ từng / gỡ hết** (xác nhận), **chặn tay IP / CIDR** (có hạn hoặc vĩnh viễn), **danh sách tin cậy**, **nhật ký truy cập** 200 lần nối / lỗi gần nhất |
| 25 | Siết relay | Mỗi IP max 4 kết nối đồng thời, chưa bắt tay xong trong 5 s thì cắt, khung quá cỡ thì cắt + tính lỗi; web app CSP chặt (không script lạ / inline, chỉ nối relay + LAN) + HSTS |
| 26 | Ghép qua relay | Mã ghép sống 2 phút, sai 5 lần là huỷ; **desktop hiện hộp duyệt** "Ghép <tên máy> từ IP <ip>?" mới cho ghép |
| 27 | Cảnh báo | Desktop + Telegram + push tới điện thoại cũ khi: **thiết bị mới ghép**; **có IP bị chặn** (gộp, max 1 lần / giờ); **thiết bị đã ghép nối từ IP / quốc gia chưa thấy** (quốc gia từ `cf-ipcountry` của relay) |
| 28 | Dev không cần Mac | Cu em làm trên Windows + iPhone: Vite dev + Cloudflare quick tunnel (https thật trên iPhone), Eruda console chỉ bản dev + log gửi về dev server, Playwright WebKit trên Windows; không debug breakpoint Safari |
