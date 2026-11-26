window.NF.gfeat = [
  { id: 'G1', ic: 'check', t: 'Hàng đợi "Chờ bạn" trên điện thoại', d: 'Mỗi lần một thẻ: trả lời, Hoãn 15 phút hoặc Bỏ qua, xong tự sang thẻ kế. Hết việc thì báo rõ, không phải mở từng session.', v: 'cao', e: 'M', to: 'demo-g1', wave: 'Đợt 2' },
  { id: 'G2', ic: 'send', t: 'Prompt đã lưu, chèn không tự gửi', d: 'Lưu câu hay dùng, chèn vào ô nhập của pane (desktop qua Ctrl+K, phone qua chip). Chèn xong vẫn phải bấm Enter.', v: 'cao', e: 'M', to: 'demo-g2', wave: 'Đợt 3' },
  { id: 'G3', ic: 'gear', t: 'Mật độ giao diện: Gọn, Vừa, Thoáng', d: 'Một công tắc đổi khoảng cách hàng và lề cho rail, danh sách, thẻ. Mặc định Vừa như hiện tại.', v: 'vừa', e: 'S-M', to: 'demo-g3', wave: 'Đợt 1' },
  { id: 'G4', ic: 'monitor', t: 'Xem nhanh session trong rail', d: 'Rê chuột (phone: chạm giữ) vào dòng session để thấy 3 dòng cuối và tool đang chạy, không đổi tab.', v: 'cao', e: 'S-M', to: 'demo-g4', wave: 'Đợt 1' },
  { id: 'G5', ic: 'clock', t: '"Từ lúc bạn đi vắng" trên điện thoại', d: 'Mở app sau 30 phút trở lên: một thẻ đếm xong, chờ bạn, lỗi; chạm số để lọc danh sách bên dưới.', v: 'vừa', e: 'M', to: 'demo-g5', wave: 'Đợt 2' },
]
window.NF.gReview = [
  ['G1', 'Dọn hàng đợi chờ trong vài chạm, không mở từng session', 'M: màn triage + gửi snooze qua kênh Noise; trả lời dùng đường nút menu có sẵn', 'Bấm nhầm: menu nguy hiểm vẫn qua vân tay (E5); Hoãn có hoàn tác 5 giây', '<code>workspace.ts::nextAttention</code>, <code>ui/workspace/inbox_page.dart</code>, <code>remote/monitor_reducer.dart</code> chỉ đọc snoozes'],
  ['G2', 'Khỏi gõ lại prompt lặp; phone gõ ít', 'M: store prompt local, palette tiền tố mới, chip trên phone', 'Nhầm với gửi tự động: chỉ chèn, ghi "chưa gửi"; đồng bộ phone qua <code>mask.rs</code>', '<code>chat_composer.dart::chips</code> chỉ dùng cho menu, <code>cmdhist.rs</code> chỉ lệnh shell'],
  ['G3', 'Rail 30 session vẫn quét được; màn lớn bớt chật', 'S-M: 3 mức token <code>--ccm-row-h</code>, <code>--ccm-gap</code>', 'E2E dựa kích thước: Vừa là mặc định, không đổi', '<code>core/stores/settings.ts</code> chỉ có font'],
  ['G4', 'Biết agent đang làm gì mà không rời việc đang làm', 'S-M: popover đọc state có sẵn', 'Workspace giật: chỉ vẽ khi rê trên 400 ms, không poll', '<code>LiveRail.vue</code> chỉ có tooltip tên'],
  ['G5', 'Quay lại điện thoại biết ngay cái gì mới', 'M: dùng lại logic <code>AwaySummary</code> cho phone', 'Nhiễu: chỉ hiện khi vắng 30 phút trở lên, tự ẩn sau khi xem', '<code>history/stores/away.ts</code> (desktop có, phone chưa)'],
]
