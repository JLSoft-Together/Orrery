# Review WP2 và WP6, v102

| mức | file:dòng | vấn đề | đề xuất |
|---|---|---|---|
| CẦN SỬA | `apps/web/src/components/next/NextShell.vue:278-303` | `TelegramToggle` chỉ render khi `!iconMode`; ở rail icon 1024-1199px, lối vào Telegram có trong `AppHeader` biến mất. | Giữ toggle dạng icon với tooltip/accessible label ở icon mode. |
| CẦN SỬA | `apps/web/src/lib/next/railMode.ts:23-27` | Hysteresis 24px chỉ áp dụng khi tăng mode; chiều thu nhỏ chuyển tại đúng 1200/1024, nên resize dao động hai phía breakpoint vẫn làm rail đổi mode liên tục. | Dùng ngưỡng giảm mode thấp hơn breakpoint 24px, đối xứng với ngưỡng tăng. |
| CẦN SỬA | `apps/web/src/components/next/NextShell.vue:238-251` | Radiogroup khi rail icon không có roving tabindex hay xử lý ArrowLeft/ArrowRight; cả hai radio đi vào thứ tự Tab bình thường, không theo keyboard pattern của radio group. | Dùng roving tabindex và phím mũi tên để đổi lựa chọn, hoặc render điều khiển tương thích keyboard từ `SegmentedControl`. |
| CẦN SỬA | `apps/web/src/components/next/NextShell.vue:372-382` | Nút mở rail overlay không có `aria-expanded`/`aria-controls`; trạng thái mở chỉ thể hiện bằng rail bị inert và scrim. | Liên kết nút với rail bằng `aria-controls`, cập nhật `aria-expanded`. |
| CẦN SỬA | `apps/web/src/components/CommandPalette.vue:843-844, 863-879` | Khi Next đang bật, placeholder và nhãn scope vẫn đọc `paletteV80.*`; chuỗi tìm kiếm/nhãn nhóm next không nhất quán namespace i18n. | Chọn key `mist.cmdk` cho giao diện next, giữ key cũ ở classic. |
| CẦN SỬA | `apps/web/src/lib/next/palette.ts:40-44` | Trong cùng bucket, `compareSessions` xếp `statusSince` mới nhất trước; nhóm `needs` vì thế không ưu tiên item chờ lâu nhất như tiêu chí rail/acceptance. | Xếp `needs` theo thời điểm cũ nhất trước; xác định riêng chiều sort cho các bucket còn lại. |
| CẦN SỬA | `apps/web/src/components/CommandPalette.vue:759-762` | Tab bị chặn để đổi scope; không thể Tab tới các nút scope, nút xóa truy vấn hoặc các điều khiển khác trong dialog. | Cho phép focus tuần tự tới nút, dùng phím mũi tên trong tablist/radiogroup, và giữ trap focus của dialog. |
| CẦN SỬA | `apps/web/src/next/tokens.css:328` | Thêm comment `WP6 overlay` trong code, trái quy tắc cấm comment trong code của rules. | Xóa comment, giữ nguyên CSS bên dưới. |
| ĐẠT | `apps/web/src/App.vue:197-216` | AppHeader và footer cũ chỉ mount khi cần; nhánh classic giữ AppHeader, footer và `div.contents`. Không thấy thay đổi chức năng classic từ phần diff được kiểm tra. | Không cần sửa. |
| ĐẠT | `apps/web/src/components/next/NextShell.vue:394-449` | CSS shell được scope dưới `:root[data-ui='next']`; không ảnh hưởng classic. | Không cần sửa. |
| ĐẠT | `apps/web/src/components/next/NextShell.vue:234-275, 306-364` | So với `AppHeader`, các view Workspace/Monitor, Usage, History, Inbox, Settings, tìm kiếm Ctrl+K, update, Telegram ở rail rộng, trạng thái/header status và menu thêm đều có lối vào. View Extensions, Devices, Disk nằm trong menu thêm. | Khôi phục Telegram ở rail icon như dòng trên. |
| ĐẠT | `apps/web/src/components/next/NextShell.vue:236-267, 306-364`; `apps/web/src/lib/next/*.ts` | Không thấy literal hex trong component hoặc các lib next. Màu ở `tokens.css` dùng token; riêng khối overlay đã scope `data-ui=next`. | Không cần sửa. |
| ĐẠT | `apps/web/src/components/CommandPalette.vue:201-212, 224-231, 278-287` | Nhánh classic giữ prefix, scope, thứ tự trạng thái và nhóm qua `classicInScope`/`CLASSIC_*`; các khác biệt giao diện row được điều kiện hóa `isNext`. | Giữ các nhánh classic tách biệt khi sửa. |

## Kết luận

**CẦN SỬA.** WP2 còn thiếu Telegram trong rail icon và hysteresis không đối xứng; cần khép các vấn đề keyboard/a11y. WP6 cần sửa thứ tự session, dùng chuỗi i18n đúng namespace cho Next và xóa comment mới trong CSS.
