# RESEARCH v107: gọn UI next desktop

Chủ nhân 20/11: "quá nhiều thông tin gây ngộp mà có khi lại không dùng hết". Kiểm kê phần tử hiện thường xuyên
trên desktop UI next, đề xuất Giữ / Ẩn (hover, menu ...) / Gộp / Bỏ. Trang chọn: `.planning/site/public/gon/`
(https://orrery.jinvu.vn/gon/, chưa deploy). Ảnh: `ccm-qa-fast` ngày 20/11. Không bỏ lối duy nhất vào tính năng nào.

Viết tắt (dưới `apps/web/src/components/`): NS `next/NextShell.vue`, LR `next/rail/LiveRail.vue`, RR `next/rail/RailRowItem.vue`,
WV `workspace/WorkspaceView.vue`, TP `workspace/TermPane.vue`, QB `next/NextQuotaBar.vue`, MB `monitor/ModeBar.vue`,
SB `monitor/SessionBoard.vue`, BC `monitor/BoardCard.vue`, CP `monitor/CommandPanel.vue`, IP `inbox/InboxPanel.vue`,
IC `inbox/InboxChip.vue`, PAL `CommandPalette.vue`, SM `SettingsMenu.vue`, SI `settings/index.ts`.

## Tổng: 128 phần tử, Giữ 56, Ẩn 28, Gộp 32, Bỏ 12

| Bề mặt | Số | Giữ | Ẩn | Gộp | Bỏ |
|---|---|---|---|---|---|
| Rail | 32 | 15 | 9 | 5 | 3 |
| Dải tab + toolbar | 19 | 9 | 2 | 4 | 4 |
| Đầu pane | 28 | 12 | 6 | 9 | 1 |
| Footer quota | 4 | 2 | 1 | 0 | 1 |
| Giám sát (Bảng) | 19 | 6 | 7 | 6 | 0 |
| Inbox popover | 5 | 3 | 1 | 1 | 0 |
| Ctrl+K | 10 | 5 | 1 | 1 | 3 |
| Cài đặt | 9 | 3 | 1 | 5 | 0 |
| Workspace trống | 2 | 1 | 0 | 1 | 0 |

## Kiểm kê (số = số trên trang; G Giữ, A Ẩn, M Gộp, B Bỏ)

| Bề mặt | Phần tử, đề xuất, file:line |
|---|---|
| Rail đầu | 1 Logo B NS:278; 2 Workspace/Giám sát G NS:279; 3 thu rail A NS:280; 4 ô tìm M NS:307; 5 phím Ctrl+K B NS:310 |
| Rail nhóm | 6 Project/Trạng thái A LR:648; 7 + mở project G LR:657; 8 gập G LR:683; 9 chữ cái project M LR:688; 10 tên G LR:691; 11 branch A LR:692; 12 số session A LR:693; 13 menu ... G LR:697 |
| Rail hàng | 14 icon G RR:91; 15 tên G RR:97; 16 chip trạng thái G RR:111; 17 chi tiết M RR:112; 18 thời gian M RR:105; 19 Đụng file M RR:99; 20 context mini A RR:118; 21 nút ngắm B RR:138; 22 đóng G RR:148; 23 Công cụ G LR:725; 24 chấm chưa lưu G LR:733 |
| Rail chân | 25 cập nhật G NS:331; 26 Telegram A NS:337; 27 điện thoại A NS:338; 28 trạng thái máy A NS:355; 29 Thống kê A NS:357; 30 Lịch sử G NS:363; 31 Cài đặt G NS:369; 32 ... G NS:381 |
| Dải tab | 1 chip project B WV:1774; 2 chuông B WV:1776; 3 thư project M WV:1782; 4 chữ cái + chấm G WV:1868; 5 tên G WV:1880; 6 thư tab M WV:1888; 7 Telegram A WV:1900; 8 "2 pane" B WV:1910; 9 zoom G WV:1918; 10 chưa lưu G WV:1925; 11 chip Lỗi M WV:1928; 12 chip chờ M WV:1930; 13 × G WV:1941; 14 +N G WV:1958; 15 + G WV:1974 |
| Toolbar | 16 Bố cục A WV:2003; 17 Inbox G WV:2005; 18 phím Ctrl+Alt+Q B IC:39; 19 Explorer G WV:2015 |
| Đầu pane | 1 icon G TP:1282; 2 tên G TP:1289; 3 project riêng G TP:1297; 4 diff A TP:1307; 5 điện thoại A TP:1320; 6 Đụng file G TP:1321; 7 overlap M TP:1472; 8 subagent M TP:1322; 9 thanh context M TP:1323; 10 token B TP:1324; 11 ContextMeter M TP:1542; 12 gợi ý compact M TP:1379 |
| Đầu pane (tiếp) | 13 branch/worktree G TP:1333; 14 ghim A TP:1348; 15 công cụ A TP:1366; 16 trạng thái G TP:1412; 17 hoạt động G TP:1425; 18 tiến trình M TP:1433; 19 tin session khác M TP:1451; 20 điều phối M TP:1466; 21 workflow M TP:1473; 22 limit G TP:1474; 23 bước compact G TP:1395; 24 ... G TP:1476; 25 pane ẩn G TP:1484; 26 zoom A TP:1500; 27 chia A TP:1517; 28 đóng G TP:1534 |
| Footer | 1 thanh quota A QB:75; 2 "Sắp hết" G QB:80; 3 "Chưa có hạn mức gói" B QB:84; 4 popover G QB:92 |
| Giám sát | 1 tóm tắt G MB:38; 2 Bảng/Quỹ đạo G MB:42; 3 nhóm theo M MB:62; 4 lọc project M MB:63; 5-6 cột dọc 0 A SB:94; 7 chữ cái G BC:128; 8 tên G BC:130; 9 dòng phụ M BC:131; 10 subagent G BC:136; 11 trạng thái G BC:140; 12 thời gian M BC:159; 13 branch A BC:160; 14 context A BC:165; 15 lượt cuối A BC:179; 16 Mở Workspace M BC:185; 17 Xem diff A BC:186; 18 Lệnh đang chạy A CP:44; 19 rail khi Giám sát M NS:279 |
| Inbox | 1 tiêu đề G IP:145; 2 dải phím A IP:146; 3 nhóm G IP:149; 4 dòng 2 M IP:160; 5 project G IP:160 |
| Ctrl+K | 1 ô tìm G PAL:888; 2 Esc B PAL:905; 3 tab phạm vi G PAL:910; 4 số đếm nhóm B PAL:943; 5 icon G PAL:978; 6 dòng phụ G PAL:988; 7 "x phút trước" M PAL:1004; 8 Shift+Enter G PAL:1008; 9 dải phím A PAL:1021; 10 dải prefix B PAL:1035 |
| Cài đặt | 1 tiêu đề G SM:334; 2 tìm G SM:346; 3 14 trang M SM:355 + SI:3; 4-7 gộp trang M SI:3; 8 Office A SI:106; 9 đóng G SM:375 |
| Trống | 1 tiêu đề + mô tả M WV:2027; 2 khôi phục G WV:2030 |

## 5 chỗ lợi nhất

1. Đầu pane: 28 phần tử có thể hiện, gom 7 chip (subagent, tiến trình, điều phối, workflow, overlap, peer) vào chip trạng thái / Đụng file / Inbox, 3 chỉ số context thành 1 hiện khi trên 70%, ghim + công cụ + diff + điện thoại vào menu ..., zoom / chia hiện khi hover. Lúc thường còn khoảng 6.
2. Hàng rail: 5 mẩu chữ (chip, chi tiết, thời gian, Đụng file, vòng context) còn 1 chip "Cần duyệt, 2 phút"; bỏ nút ngắm.
3. Dải tab: bỏ chip project đầu dải + chuông (rail và Inbox đã có), chip Lỗi / chờ về chấm màu, bỏ chữ "2 pane", bỏ phím trong Inbox.
4. Chân rail: Telegram, điện thoại, Thống kê, trạng thái máy vào menu ..., còn Lịch sử, Cài đặt, ...
5. Giám sát: card còn tên, trạng thái, subagent; bấm card mở Workspace; nhóm + lọc thành 1 menu; cột 0 ẩn; rail thu về icon.
