# WP-31 báo cáo: cụm 3 Monitor, Workspace, terminal

## Đợt — Monitor
- A: thẻ (viền màu agent, chip trạng thái, branch mono, dải tool / waitingFor, thời gian, chip lọc Tất cả / Đang chạy / Chờ bạn / Rảnh). B: nhóm theo trạng thái, "Chờ bạn" lên đầu, dòng gọn. Nút đổi chế độ ở AppBar, đổi có chuyển cảnh fade + scale 220 ms. Nhớ lựa chọn trong `view_prefs.json` (app support, tự ghi / đọc).
- Chi tiết session: thẻ agent, branch, giờ bắt đầu, máy, tool / ngữ cảnh / token (chỉ khi có số thật), sub-agent, nút Mở terminal, đổi tên ở menu.
- Ảnh: docs/qa/v97/WP-31/mon-a, mon-b, detail (-light, -dark).

## Đợt — Workspace
- A: tiêu đề, dải tab, chip pane theo agent, 1 thẻ pane (giếng tối + nút Mở toàn màn hình), vuốt ngang đổi pane, gợi ý "Vuốt ngang… N pane trong tab". B: lưới 2 cột của tab đang chọn, chip lọc, FAB. Thanh dưới vạch cam.
- Ảnh: ws-a, ws-b (-light, -dark).

## Đợt — Terminal
- AppBar: nút chọn pane (agent · tên tab) mở sheet `term-pick`; dải mảnh chip trạng thái + tool + "Bạn đang gõ / Chỉ xem"; phím phụ viền; ô gõ pill + nút gửi tròn cam; permission bar dùng nút kit. Terminal luôn nền tối.
- Không đụng `_pendingOut`, `LineProbe`, `pty_stream.dart` (chỉ thêm cờ `compactHeader` cho khung). Ảnh: term, term-pick (-light, -dark).

## Đã đổi
- Mới: lib/ui/monitor/{view_prefs,monitor_filter,monitor_cards,session_detail_page,since_text}.dart; lib/ui/workspace/{shell_nav,workspace_body,workspace_tabs,workspace_single,pane_card,pane_preview,pane_grid,pane_picker,offline_notice}.dart; lib/ui/kit/{rail_chip,view_mode_button,filter_chip,quiet_size}.dart; tool/arb_cluster3.mjs; test monitor_wp31_test, wp31_shots_test, support/wp31_scenario.
- Sửa: monitor_page, mission_page (thành `MissionBody` + `MissionPage` mỏng), workspace_page, tab_page (bỏ chấm / chip tab anh em), pane_header, input_bar, permission_bar, terminal_pane (chỉ cờ `compactHeader`), open_pane_page (QuietSize), kit AnimatedItems (`gap`), tool/gen_arb.mjs, ARB + l10n sinh lại, .code_index/mobile.md, test cũ theo luồng mới (ui_screens, rename, viewers, terminal_pane, deep_link, ui_integration, ui_fakes).

## Tiêu chí n/m
- Monitor A/B, Workspace A/B, terminal khớp mockup, ảnh sáng + tối, nhớ lựa chọn: đạt, có chỗ khác (bên dưới).
- Đường ghi terminal không đổi, test terminal cũ pass: đạt.
- Motion đúng spec, tắt khi giảm chuyển động: đạt (có test; tìm thấy và sửa lỗi thật: `AnimatedSize` Duration.zero assert và `animateToPage` 0 ms khi bật giảm chuyển động).
- analyze sạch, `flutter test --concurrency=4` 263 pass: đạt. vi + en, không comment, file ≤ 400 dòng, index cập nhật: đạt.

## Test thật
flutter analyze: No issues. flutter test --concurrency=4: All tests passed (263, 53 skip do tag / shots). Test mới: lọc / đổi view / nhớ view / motion xoá dòng (monitor_wp31), chip pane, lưới B, picker (ui_screens).

## Nhìn
Đã mở ảnh mon-a/mon-b/detail/ws-a/ws-b/term/term-pick. Bố cục, chip, thẻ, nút khớp mockup. Lỗi tự bắt và sửa trong lúc nhìn: chữ "Workspace" bị xuống dòng (bỏ LinkPill khỏi header), chip lọc / tab / nhãn nav mất font (style không kế thừa theme), giếng tối canh giữa (đổi sang canh trên trái), 2 thanh kéo ở sheet.

## Khác mockup (cần chủ nhân biết)
- Thanh dưới giữ Workspace / Quỹ đạo / Inbox (không có Cài đặt: cụm 4; Inbox là tính năng đang có). Thứ tự giữ để mở máy vào Workspace.
- Giếng tối ở thẻ pane KHÔNG phải output terminal thật (không có nguồn không-attach, giới hạn 4 attach): hiện waitText / tool đang chạy / subagent từ snapshot, rảnh thì hiện "Rảnh". Mockup vẽ 6 dòng output.
- Phím phụ chưa có Ctrl / Alt dính (cần chặn đường nhập, tránh đụng terminal); giữ Esc / Tab / Shift+Tab / ^C / 4 mũi tên / Enter.
- Chi tiết session chưa có "Hoạt động gần đây", nút gửi, nút dừng (không có nguồn dữ liệu / lệnh đã duyệt).
- Thời gian dạng "<1m / 14m / 2h" (không có giây, không "p / g"). Chip LAN không có "ms" (chưa có số đo thật).
- Trong terminal, tên pane chỉ thấy trong sheet chọn pane (nút ở AppBar hiện "Agent · tên tab").
- Lưới B: thẻ vào có hiệu ứng, thẻ xoá không có hiệu ứng trượt ra (chỉ đổi bộ lọc có fade).
- Harness test mặc định bật `disableAnimations` (PulseDot lặp vô hạn làm `pumpAndSettle` treo).

## Commit message đề xuất
feat(mobile): new Monitor (cards, grouped list, session detail), Workspace (pane pager, mission grid, tab strip) and terminal chrome (pane picker sheet, key row, pill input) with remembered view modes and reduce-motion-safe animations
