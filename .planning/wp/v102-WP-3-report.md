WP-3 (rail sống): ĐẠT (còn 4 điểm hạn chế bên dưới)

File:
- Mới: `apps/web/src/lib/next/railLive.ts` (+ `railLive.test.ts`, 13 test), `lib/next/railSnooze.ts`, `components/next/rail/{RailRowItem,RailNeeds,RailCollapsed}.vue`.
- Viết lại: `components/next/rail/LiveRail.vue` (props `{collapsed}` giữ nguyên).
- Sửa nhỏ: `i18n/mist-v102.ts` (khối `mist.rail`, en + vi), `.code_index/web.md`.

SPEC ID đã làm: R1 (row 2 dòng min 48px, gauge 28x4 vàng từ 65%, thiếu model thì trống, shell / run không hiện model), R2 (segmented nhớ `railGroup`; nhóm Lỗi, Đang chạy, Xong chưa xem, Rảnh), R3 (khối Cần bạn sticky, ẩn khi rỗng, nút Mở, "Mở kế tiếp" gọi `workspace.nextAttention()` giống Ctrl+Shift+A), R4 (khoá / ? / x, xong chưa xem nằm nhóm riêng), R5 (hoãn 5/15/60, lỗi max 15, toast hoàn tác 5s dùng `toasts.actions`), R6 (`freezeLayout` khi con trỏ trong rail), R7 (rail 56: monogram 28, chấm trạng thái, badge, flyout 280, nhãn hiện khi focus / hover), R8 (`info.modelOf` có sẵn từ WP6, không thêm), M3 (viền 450ms 1 lần, `--ccm-dur-signal`, theo `data-motion`).
Giữ hành vi cũ: bấm mở / focus pane, kéo row giữa tab (`useRailDrag`), kéo project đổi thứ tự (`useClusterDrag`), multi-select Ctrl/Shift, menu chuột phải row / project / chọn nhiều, F2 đổi tên (`TabRenameInput`), đóng (`requestClose`), roving focus phím mũi tên.

Typecheck: pass (`npm run typecheck -w @ccm/web`).
Test: 193 pass / 0 fail (`vitest run --root apps/web src/lib/next src/lib/rail src/components/next src/i18n src/stores/uiGen.test.ts`).

Hạn chế / việc cho gói sau:
- Thứ tự khối Cần bạn theo SPEC R4 (duyệt quyền / câu hỏi trước lỗi); bất biến cũ "lỗi trước asked" vẫn đúng ở `buildAttentionList` + chord Ctrl+Shift+A (không đổi). Cần chủ nhân chốt nếu muốn đồng nhất.
- Menu row bỏ "Chia phải / dưới" và "Hiện trong Explorer"; menu project bỏ Run, Scripts, Layouts, Plan, Mission vì các handler nằm trong `WorkspaceView` (WP4a). Nếu muốn đủ, WP4a / WP2 cần cung cấp `provide` cho các lệnh này.
- Kéo project lên tab strip cần `RAIL_PROJECT_DRAG` do `WorkspaceView` provide, LiveRail nằm ngoài nên chưa nối. Kéo row / project chỉ chạy ở nhóm Project (nhóm Trạng thái không có cluster section).
- Rail không có khái niệm row "ghim"; R6 áp cho cả rail. Hoãn lưu trong RAM (mất khi reload).
- Hộp xác nhận đóng pane do `WorkspaceView` render (`pendingClose`); cần WorkspaceView luôn mounted ở next (hiện đúng).
- Chưa chụp ảnh rail (không chạy app / E2E theo brief).
