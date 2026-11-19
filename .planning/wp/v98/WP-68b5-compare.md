# WP-68b5 mục 2: so ảnh app thật với mockup WP-68a

Đã so 14 cặp (Chromium): home, monitor A, monitor B, detail, limit, pair-code, pair-scan, lock, term, ws-b, monitor iPad, rail iPad; mỗi màn 1 cặp sáng hoặc tối. WebKit không mở.

## Đã sửa

| Lệch | Sửa tại |
|---|---|
| Tên session, tên agent và cả hàng bị tô màu trạng thái (vàng, xanh) vì class `tone-*` scoped của `MonitorView` rò xuống root của `SessionRow` và `LinkChip` | `apps/pwa/src/views/MonitorView.vue` đổi `tone-*` thành `gtone-*` (template dòng 128, style từ dòng 387) |
| Chip trạng thái Idle trong chi tiết session bị tô xanh Done, cùng nguyên nhân với `StatusChip` | `apps/pwa/src/views/SessionView.vue` đổi `tone-*` thành `subtone-*` (dòng 108, 311-324) |
| Ô chữ cái project tone 4 (vd "web") mất nền và chữ đen vì dùng biến không tồn tại `--ccm-success` | `apps/pwa/src/styles/base.css:176`, `apps/pwa/src/components/IpadRail.vue:353,435` đổi sang `--ccm-done` |
| Chip Relay / LAN viền đặc, mockup là chip tô nền nhạt không viền | `apps/pwa/src/components/LinkChip.vue` (`.chip`: `border: 0`, nền `color-mix 12%`) |
| Màn "Bật khoá" nút bám ngay dưới thẻ, mockup đẩy nút xuống đáy (spacer không có tác dụng vì `.view` không phải flex) | `apps/pwa/src/App.vue` `.view:not(.fill)` thành flex column, con `flex: 1 0 auto` |

## Bảng so

| Màn | Thiết bị | Khớp / lệch | Lệch còn lại / đã sửa |
|---|---|---|---|
| home | phone sáng + tối | khớp | demo chỉ 1 máy (khác dữ liệu); chip Relay đã sửa; viền chuông bật dùng accent, giữ vì là trạng thái bật |
| pair code | phone sáng | khớp | không có |
| pair scan (đang ghép) | phone sáng | khớp | mockup chụp khung camera, app chụp trạng thái đang ghép (khác kịch bản) |
| lock | phone tối | lệch, đã sửa | nút đẩy xuống đáy (App.vue); tiêu đề "Bật khoá màn hình" do UA test không phải iOS |
| monitor A | phone sáng | lệch, đã sửa | màu tên session / agent (MonitorView); ô "W" (base.css). Còn: mockup có thanh context nhỏ bên phải hàng session và tên agent trung tính, app không có thanh context (thiếu dữ liệu trong SessionRow, không sửa) |
| monitor B | phone tối | lệch, đã sửa | tô màu cả hàng (như trên) |
| detail | phone sáng | khớp | app đặt nút "Mở terminal" cuối trang, mockup là nút icon ở header; "Hoạt động gần đây" không nằm trong thẻ khi rỗng. Thiết kế khác nhau, không sửa |
| limit | phone sáng | lệch nhỏ | mockup ẩn segmented và filter khi có banner limit và thêm dòng "Dữ liệu lúc", app giữ cả hai. Không sửa |
| terminal | phone sáng | lệch nhỏ | key dock 11 phím cuộn ngang (mockup 7 phím vừa khung), cuộn ngang trong dock là chủ ý, không tràn trang. Terminal chụp dữ liệu 64 KiB nên khác nội dung |
| ws B (nhóm project) | phone sáng | khác thiết kế | b4 chọn nhóm project thu gọn 1 cột, mockup lưới 2 cột. Ô "I" mất nền nay đã có nền |
| monitor | iPad sáng | lệch lớn, chưa sửa | mockup có khung "Tổng quan" dạng lưới thẻ session ở cột phải và tab Project / Trạng thái trong rail; app hiện danh sách kiểu phone ở cột phải và không có tab con trong rail. Cần gói riêng, không phải sửa nhỏ |
| rail | iPad sáng | khớp phần còn lại | ô "W" đã sửa; chip Relay đã sửa |

## Kiểm tra

- `vue-tsc` và `vitest` chạy tay: không lỗi ở file đã sửa. Lỗi duy nhất: thiếu `src/gen/core_remote_wasm.js` (7 file test import wasm không chạy được, 19 file / 182 test còn lại pass).
- `npm run typecheck|test -w @ccm/pwa` không chạy được: `scripts/ensure-wasm.mjs` hỏng vì `wasm-bindgen` CLI 0.2.129 lệch crate 0.2.130 (`cargo install -f wasm-bindgen-cli --version 0.2.130` sẽ sửa); lần chạy này làm `apps/pwa/src/gen/` (gitignored) rỗng, cần chạy lại `npm run wasm -w @ccm/pwa`.
