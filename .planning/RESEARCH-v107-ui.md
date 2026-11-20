# RESEARCH v107: UI/UX đợt 3

Ngày 20/11/2026. Đợt 1 (U1 đến U5) và đợt 2 (U6 đến U10) đã làm, `status: done` trong `.planning/site/public/data/ui.json`.
Trang: `.planning/site/public/ui/` (chưa deploy). U16, U17 do chủ nhân thêm cùng ngày.

## Xếp hạng (lợi ích trên công sức)

| # | Id | Nơi | Công sức | Việc |
|---|----|-----|----------|------|
| 1 | U11 | desktop | S | Tab strip: tên tab là tên pane, project chỉ 1 chip |
| 2 | U12 | desktop | S | Màn trống: thẻ agent không lặp tên, "Project gần đây", rail không nói 2 lần |
| 3 | U13 | desktop | S | History Hôm nay: chữ dưới thanh, hoa văn, "làm N%" |
| 4 | U14 | iPad / điện thoại | M | iPad cột phải là chi tiết; trang chi tiết không lặp tên, trả lời trên thẻ |
| 5 | U15 | Android | M | Thẻ Chờ bạn trả lời ngay, chip pane có trạng thái + chấm trang |
| 6 | U16 | điện thoại + Android | S | Nút Shell mới 1 chạm, vào thẳng terminal |
| 7 | U17 | điện thoại + Android | L | Mạng yếu kiểu mosh: gõ đoán, nối lại giữ màn, chip độ trễ |

## Bằng chứng
- U11: `ccm-qa-fast/rail/rail-03-needs.png` (3 tab "alpha"), `apps/web/src/stores/workspace.ts:252-257`, `composables/usePaneInfo.ts:121-127`.
- U12: `docs/qa/v103/WP-70b/01-more-open.png` ("Claude Code / Claude Code", "Open sessions"), `components/workspace/LaunchPicker.vue:49, 94, 185`, `i18n/mist-v102.ts:88`.
- U13: `ccm-qa-fast/v73/v73-01-toolstats-30.png` (thanh xám kín + "0%"), `components/history/TimelineChart.vue:111-142` (chỉ màu, trái rule icon + chữ + màu).
- U14: `apps/pwa/e2e/.out/shots-55a/monitor-b-ipad-light-webkit.png`, `WP-68b3/detail-ipad-light-webkit.png` ("docs" 4 lần, 2 nút cùng mở terminal), `apps/pwa/src/views/SessionView.vue:66-80, 100, 130`, `components/IpadRail.vue:88-96`.
- U15: `apps/mobile/shots/03c-workspace-vi.png`, `apps/mobile/lib/ui/workspace/pane_card.dart:91-95`, `workspace_single.dart:120-130` (chip Codex tô cam accent).
- U16: `apps/pwa/src/views/WorkspaceView.vue:213, 221` (nút chỉ khi tab có cwd), `components/NewPaneSheet.vue:16-21` (mở xong ở lại lưới), `apps/mobile/lib/ui/workspace/open_pane_page.dart:75-93` (pop về Workspace).
- U17: `apps/pwa/src/terminal/ptyStreams.ts:99-108` và `apps/mobile/lib/remote/pty_stream.dart:189-192` (phím lúc chưa ready bị bỏ im lặng), `TerminalView.vue:152` (dải offline web), `terminal_pane.dart:358-376` (Android không có dải nối lại), rtt chỉ ở `LinkChip` (`WorkspaceView.vue:178`, `MachineCard.vue:48`).

## Học từ SSH / mosh
- Termius, Blink: shell mới là 1 chạm từ màn chính, vào thẳng terminal, nhớ host / thư mục lần trước. Orrery đã có `ws.open` preset shell, chỉ thiếu nút và bước chuyển thẳng vào terminal (U16). Remote tắt thì ẩn.
- mosh, predictive local echo: ký tự in được hiện ngay, gạch chân tới khi server xác nhận; tự tắt khi đoán sai, khi màn full-screen, khi echo tắt. Áp cho Orrery chỉ ở lớp vẽ: phím vẫn gửi như cũ, chỉ sau khi chủ nhân gõ; tắt đoán khi alt screen (`ESC[?1049h` có sẵn trong `pty.snapshot`) và khi password.
- mosh, roaming: đổi IP không mất session. Orrery đã có phần lõi: `ptyStreams.onReady` attach lại, `pty.snapshot` dựng lại màn (docs/remote-integration.md §4.4). Thiếu: giữ màn cũ khi đang nối, dải nhỏ "Đang nối lại, lần N", không bỏ phím im lặng (đưa vào ô "Chưa gửi", chủ nhân bấm Gửi, không tự gửi).
- mosh hiện độ trễ khi chậm; Orrery đo rtt rồi nhưng trang terminal chưa hiện: chip "Relay 180 ms, chậm" (chữ + icon + màu).
- Thứ tự cho U17: bước 1 (nối lại giữ màn + ô Chưa gửi + chip độ trễ, cỡ M), bước 2 (gõ đoán, cỡ L, web xterm trước, Android sau).

## Để sau (chưa chọn)
- Ctrl+K: chưa có ảnh E2E mới của palette next; cần chụp trước khi đánh giá.
- Settings: trang Permissions & security dày chữ (`ccm-qa-fast/tabs-cwd/fix-04-settings-forget.png`), phần "Ignored per project" và "File access" có thể gom; chưa ưu tiên.
- History, heatmap "When agents work": ô 14px không có số, chỉ đậm nhạt (`v73-01-toolstats-30.png`).
- App Android "My computers" trống (`apps/mobile/shots/02-machines-empty.png`) nhắc "Settings > Remote" trong khi desktop đã có thẻ "Cài app lên điện thoại"; nên đổi chữ cho khớp tên thẻ mới.
