# DECISIONS

- 2026-11-04 · MCP `telegram_receive on` phải qua hộp xác nhận native trên máy (agent không tự bật nhận lệnh từ xa được); `off` / `status` và `telegram_notify` không cần · lý do: review WP-1 (prompt injection có thể tự bật nhận lệnh). Main quyết, báo chủ nhân.
- 2026-11-04 · Chủ nhân cho phép tự test Telegram bằng bản dev (`npm run build` web + desktop, electron stage với user-data-dir / CCM_DATA_DIR tạm, cổng 4319, CDP 9333) · lý do: chủ nhân nói 'con tự test đi nhé'.
- 2026-11-05 · Người lạ tag / trả lời bot: bot trả câu cố định "Xin lỗi bạn, tôi chỉ trả lời chủ nhân <tên>, gửi thêm tiền để thương lượng 🐮" (tối đa 5 câu/phút, không bao giờ chuyển vào session, không tốn token) — thay cho "bỏ qua im lặng" của 2026-11-04 · lý do: chủ nhân yêu cầu.
- 2026-11-05 · Kết nối Telegram theo session (menu chuột phải session ở rail → "Kết nối Telegram…", chỉ session này hoặc cả project) · tuyến session gắn theo Claude session id (giữ qua resume), trang đẩy bảng pane ↔ session ↔ terminal sang main (`ccm:telegram-panes`) · lý do: chủ nhân yêu cầu.
- 2026-11-05 · Thành viên Telegram (WP-D, main chốt theo đề xuất, chủ nhân đổi được): mức `read` chỉ status / last / tail / help + nút 💬 📜 📄; mức `full` mọi lệnh + nút trừ Cho phép / Từ chối quyền tool, nối topic, lệnh lúc app tắt, ghép bot (chỉ chủ nhân); prompt của thành viên mang tên đã lưu trong app, không lấy tên Telegram (chống mạo danh).

- 2026-11-09 · Office (Habblaud) mặc định TẮT, bật trong Settings · chủ nhân hầu như không dùng; tắt thì collector không chạy OfficeHub/timelapse (tiết kiệm CPU/disk), web không load Habblaud.
- 2026-11-09 · PLAN-v87 lõi Rust: đồng ý cài Rust + CI macOS; bỏ blur trên toolbar/sidebar (GPU); chuyển MCP orrery từ PTY host sang Electron main ở bước 2; giảm cả size cài đặt (bước 1b, mạnh nhất ở bước 6 Tauri).
- 2026-11-09 · Tauri không Node sidecar, port hết sang Rust rồi mới bỏ Electron · chủ nhân muốn size bộ cài nhỏ nhất (~20–35 MB) · .planning/PLAN-v87-tauri-options.md

## v97 — App Orrery mobile (2026-11-12)
- 2026-11-12 · App Orrery riêng, không dùng `claude remote-control` · chủ nhân muốn Workspace như desktop
- 2026-11-12 · Kết nối LAN trước, khác mạng qua relay Cloudflare Workers + Durable Objects
- 2026-11-12 · 1 Workspace chung desktop ↔ điện thoại; cỡ terminal: bên nào đang gõ thì cỡ bên đó
- 2026-11-12 · Client Flutter; chỗ nào Flutter không đủ thì được viết native (Jetpack Compose / SwiftUI), chỉ khi cần
- 2026-11-12 · Phạm vi làm cả: terminal, Workspace, Explorer, file, Changes, Git Log, Run, Monitor dạng list
- 2026-11-12 · Cho phép nội dung session ra điện thoại (mã hoá đầu-cuối); đổi trên máy thì điện thoại hiện theo, chậm chút được
- 2026-11-12 · Thông báo: a) Telegram / ntfy có sẵn trước, b) push native APNs / FCM ở bước cuối
- 2026-11-12 · Chỉ cài APK + TestFlight, chưa lên store
- 2026-11-12 · Branch `feat/v97-mobile`, commit từng phần khi gói nghiệm thu xong, không push khi chưa được bảo
- 2026-11-12 · Hỏi chủ nhân bằng AskUserQuestion; phân tích spec kỹ trước khi code
- 2026-11-12 · Thao tác cần xác nhận làm từ điện thoại → hỏi trên điện thoại; ghép thiết bị mới vẫn xác nhận trên máy tính
- 2026-11-12 · Switch "giữ máy thức khi bật điều khiển từ xa", mặc định bật
- 2026-11-12 · 1 app quản lý nhiều máy tính (Windows + Mac)
- 2026-11-12 · Android + iOS cùng lúc
- 2026-11-12 · Không khoá app bằng Face ID / vân tay (dựa vào khoá màn hình điện thoại)
- 2026-11-12 · Relay trên tài khoản Cloudflare của chủ nhân, `*.workers.dev`
- 2026-11-12 · App mobile theo theme + ngôn ngữ của desktop đang kết nối, sáng / tối theo điện thoại
- 2026-11-12 · Desktop: chip 📱 + log; notification chỉ khi thiết bị lạ / lần đầu trong ngày
- 2026-11-12 · Tablet / iPad / máy gập: chia pane như desktop khi màn ≥ 700 dp, máy nhỏ 1 pane + vuốt
- 2026-11-12 · Terminal trên điện thoại: mặc định khung nhập dòng + thanh phím phụ, có nút chuyển sang gõ thẳng
- 2026-11-12 · Máy offline: hiện trạng thái cuối (cache trên điện thoại) đánh dấu cũ + giờ, không gõ được, tự nối lại
- 2026-11-12 · Tên app "Orrery Remote", package `dev.ccm.orrery.remote`, code ở `apps/mobile`
- 2026-11-12 · Điện thoại và desktop focus riêng: lệnh từ điện thoại không đổi tab / pane đang xem trên desktop
- 2026-11-12 · Mã hoá đầu-cuối: 1 crate Rust (snow) dùng chung máy + điện thoại qua flutter_rust_bridge
- 2026-11-12 · Firebase project mới `orrery-remote` (chủ nhân tạo khi tới bước push)
- 2026-11-12 · Cloudflare: Free trước, vượt thì báo chủ nhân lên Paid 5 USD/tháng
- 2026-11-12 · Test: Android cắm USB vào máy Windows này trước; iOS (iPhone + Simulator) chạy trên MacBook khi chủ nhân báo
- 2026-11-12 · Vỏ chỉ còn Tauri (4.0.0 bỏ Electron) ⇒ gateway nằm trong `apps/shell`, không làm cho Electron
- 2026-11-12 · Chạy liên tục qua đêm: xong lượt → commit + handoff + tự làm lượt tiếp; push branch `feat/v97-mobile` cuối mỗi lượt
- 2026-11-12 · Cổng duyệt cảm quan (terminal sau WP-1, UI sau WP-11): con tự chọn theo số đo, ảnh gom `docs/qa/v97/` chủ nhân xem sau
- 2026-11-12 · Relay: code + test local (miniflare) rồi deploy; chủ nhân đã `wrangler login` (2026-11-12, wrangler 4.160.1)
- 2026-11-12 · Relay + push do người dùng tự dựng (mỗi người server riêng): Settings → Điều khiển từ xa có mục dựng relay (dán Cloudflare API token, Orrery tự deploy relay đóng gói sẵn qua Cloudflare API, không Node / wrangler) và mục thông báo đẩy (kéo thả `google-services.json` + service account JSON)
- 2026-11-12 · Push do desktop gửi thẳng FCM HTTP v1 (service account khoá bằng keychain trên máy); relay không giữ secret Firebase (đã xoá secret `FCM_SERVICE_ACCOUNT` khỏi relay jinvu); điện thoại nhận `FirebaseOptions` qua phiên Noise, init Firebase lúc chạy
- 2026-11-12 · Điện thoại chỉ init 1 Firebase (plugin chỉ hỗ trợ app mặc định): dùng project của máy đầu tiên gửi `push.config`; máy khác cùng projectId vẫn nhận push, khác projectId ⇒ hiện "Máy này dùng Firebase khác, không nhận thông báo" (con tự quyết, đa số người dùng 1 project)
- 2026-11-12 · Chưa dựng relay ⇒ chỉ LAN; relay `orrery-relay.jinvu.workers.dev` là relay riêng của chủ nhân, không phải mặc định chung
- 2026-11-12 · App điện thoại khoá bằng sinh trắc học / PIN máy (local_auth), bật mặc định, xác thực lần đầu rồi lại sau mỗi 1 giờ (chỉnh được trong cài đặt, đổi cài đặt phải xác thực); khoá = đóng mọi kết nối · chủ nhân hỏi 2FA, chọn giữa sinh trắc và TOTP → con chọn sinh trắc (không cần app thứ 2, TOTP trên cùng máy thêm ít bảo vệ)
- 2026-11-15 - Test app của v100/v101 chỉ chạy bản dev tách biệt (cổng riêng, userData/CCM_DATA_DIR/APPDATA tạm), không ảnh hưởng Orrery đang chạy - chủ nhân yêu cầu; công thức trong PROGRESS-2026-11-15-click-disk.md.

## v104 - 7 đề xuất UX + terminal điện thoại (2026-11-20)
- 2026-11-20 - Gói đầu tiên: terminal điện thoại mất dòng khi resize (ConPTY `PSEUDOCONSOLE_RESIZE_QUIRK` vẽ lại theo cỡ mới, điện thoại resize muộn). Làm cả 2 hướng: điện thoại xử lý output + event đúng thứ tự, flush rồi resize ngay (Android + PWA); desktop giữ terminal ảo Rust, gửi `pty.snapshot` (scrollback max 2000 dòng) sau mỗi resize và khi nối lại - chủ nhân chốt; `.planning/PLAN-v104-ux-proposals.md` WP-104a1/a2
- 2026-11-20 - Inbox gộp cho desktop + web app + Android, cùng nhóm cùng thứ tự; desktop gửi thêm nhóm menu chờ trả lời và đụng file qua remote; Inbox không trả lời menu (ngoại lệ 6 giữ nguyên) - chủ nhân chốt
- 2026-11-20 - UI classic: E2E sang next ngay; 4.0.8 đóng băng + changelog báo trước; xoá hẳn ở 4.1.0 - chủ nhân chốt (thay mốc "sau 1-2 bản")
- 2026-11-20 - Electron: v104 chỉ làm bảng so tính năng Tauri vs Electron + E2E mặc định Tauri; xoá `apps/desktop/src` sau khi `.planning/QA-mac-checklist.md` đạt, giữ tag `last-electron` - chủ nhân chốt
- 2026-11-20 - Cảnh báo đụng file: 1 switch chung `ccm.notify.clash` (Settings, Thông báo) cho chip rail + header pane + nhóm Inbox + toast, mặc định TẮT; bật thì mỗi cặp (file, 2 session) báo 1 lần / lượt, có "Bỏ qua file này" + "Xem diff", chỉ cảnh báo không chặn - chủ nhân chốt
- 2026-11-20 - Settings: sửa search + test mọi entry có dòng thật, `fileIcons` / `dust` chỉ ở trang Workspace, gộp `reminders` vào `notify` (map `ccm.settings.page` cũ) - chủ nhân chốt
- 2026-11-20 - Changes theo session: luôn hiện nhóm "Không rõ", giữ tập file lượt cuối tới khi đánh dấu đã xem, notify `done` có nút "Xem thay đổi" - chủ nhân chốt
- 2026-11-20 - DeepSeek không xoá, tách phần web thành module lazy `apps/web/src/features/deepseek/` - chủ nhân chốt
- 2026-11-20 - Triển khai v104 chế độ manager; mỗi gói nghiệm thu xong commit + push lên `feat/v98-pwa`, không trailer attribution - chủ nhân cho phép riêng đợt này
