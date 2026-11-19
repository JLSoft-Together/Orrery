# Review v102, lượt r3

| mức | file:dòng | vấn đề | đề xuất |
|---|---|---|---|
| cao | `apps/web/src/App.vue:200` | Đổi generation thay loại component cha từ `div` sang `NextShell`. Vue sẽ unmount subtree cũ rồi mount lại, bao gồm `WorkspaceView` và xterm, dù ngay dưới đó code ghi rõ cần giữ xterm state khi đổi view. Việc bật/tắt next có thể làm mất trạng thái pane/terminal, trái bất biến T3 và yêu cầu chuyển generation không restart. | Giữ nguyên một component cha và chuyển shell bằng nhánh bên trong hoặc slot để `WorkspaceView` không bị remount khi `isNext` đổi. |
| thấp | `apps/web/src/next/tokens.css:703` | Palette card animate `border-color`, ngoài quy tắc animation chỉ dùng `transform` và `opacity` ở rules; thay đổi border có thể kích hoạt paint. | Bỏ transition `border-color`, giữ transition `transform`/`opacity` hoặc đổi trạng thái viền tức thời. |
| thấp | `apps/web/src/next/tokens.css:528` | `:has()` kiểm tra trạng thái rỗng trên từng phần tab extras. Không có bằng chứng hiện tại cho thấy thành nút thắt, nhưng selector này áp lên mỗi tab và được tính lại khi nội dung tab thay đổi. | Ưu tiên class/attribute trạng thái rỗng do component render; chỉ giữ `:has()` nếu đo trên workspace nhiều tab cho thấy chi phí không đáng kể. |

## Kết luận

**REJECT** do nguy cơ remount `WorkspaceView`/xterm khi chuyển generation. Các phần đã đối chiếu: CSS next đều scope `[data-ui='next']`; các sửa đổi viewer/Mission có nhánh `isNext`; selector dữ liệu rail và pane vẫn được giữ; không thấy comment mới, hex hardcode trong component, hoặc chuỗi UI viết trực tiếp trong các file component được yêu cầu. Logic rail sort/freeze/snooze, ngưỡng Explorer pin và thời lượng dust đọc được phù hợp checklist tương ứng. Chưa đánh giá trực quan/E2E; reports cũng ghi nhận các bước này chưa chạy.
