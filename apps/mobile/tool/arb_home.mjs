const S = (en, vi, ph) => ({ en, vi, ph })

export const H = {
  homeTitle: S('My computers', 'Máy của tôi'),
  homeConnected: S('Connected', 'Đang kết nối'),
  homeDuplicates: S('Same name', 'Bản trùng tên'),
  idleCount: S('{count} idle', '{count} rảnh', { count: 'int' }),
  pairedOn: S('Paired {date}', 'Ghép {date}', { date: 'String' }),
  cleanOld: S('{count, plural, =1{Remove 1 old copy} other{Remove {count} old copies}}', 'Dọn {count} bản cũ', { count: 'int' }),
  cleanOldTitle: S('Remove old copies?', 'Dọn bản cũ?'),
  cleanOldBody: S('This phone forgets {count} copies that share a name with a newer one. The newer one stays.', 'Điện thoại sẽ quên {count} bản trùng tên với bản mới hơn. Bản mới giữ nguyên.', { count: 'int' }),
  cleanOldDone: S('Old copies removed.', 'Đã dọn bản cũ.'),
  pairNewMachine: S('Pair a new computer', 'Ghép máy mới'),
  enterCodeManual: S('Enter code manually', 'Nhập mã thủ công'),
  pairTabScan: S('Scan QR', 'Quét QR'),
  pairTabCode: S('Enter code', 'Nhập mã'),
  pairScanTitle: S('Point the camera at the QR code', 'Đưa mã QR vào khung'),
  pairE2eTitle: S('End-to-end encrypted', 'Kết nối mã hoá đầu cuối'),
  pairE2eBody: S('This phone and the computer share a Noise key. The relay only carries encrypted bytes.', 'Điện thoại và máy dùng chung khoá Noise. Relay chỉ chuyển byte đã mã hoá.'),
  pairCodeLabel: S('Pairing code from the computer', 'Mã ghép trên máy tính'),
  pairCodeField: S('Pairing code', 'Mã ghép'),
  pairCodeSubmit: S('Pair', 'Ghép'),
}
