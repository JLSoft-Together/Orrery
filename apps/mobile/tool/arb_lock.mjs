const S = (en, vi, ph) => ({ en, vi, ph })

export const L = {
  lockReason: S('Unlock Orrery Remote', 'Mở khoá Orrery Remote'),
  lockTitle: S('Orrery Remote is locked', 'Orrery Remote đang khoá'),
  lockBody: S("Verify with fingerprint, face or your phone's screen lock to continue.", 'Xác thực bằng vân tay, khuôn mặt hoặc khoá màn hình của máy để tiếp tục.'),
  lockUnlock: S('Unlock', 'Mở khoá'),
  lockCancelled: S('Verification was cancelled.', 'Đã huỷ xác thực.'),
  lockFailed: S('Verification failed. Try again.', 'Xác thực lỗi, thử lại.'),
  lockNoCredentials: S('This phone has no screen lock. Set a PIN, pattern, password or fingerprint in the system settings.', 'Máy chưa đặt khoá màn hình. Đặt PIN, mẫu hình, mật khẩu hoặc vân tay trong cài đặt hệ thống.'),
  lockOpenSecurity: S('Open security settings', 'Mở cài đặt bảo mật'),
  lockTurnOff: S('Turn off app lock', 'Tắt khoá app'),
  lockTurnOffTitle: S('Turn off app lock?', 'Tắt khoá app?'),
  lockTurnOffWarn: S('Anyone who holds this phone can then open your sessions and terminals.', 'Bất kỳ ai cầm điện thoại này sẽ mở được session và terminal của bạn.'),
  lockSettingsTitle: S('App lock', 'Khoá app'),
  lockSwitch: S('Require verification', 'Yêu cầu xác thực'),
  lockSwitchHint: S('Fingerprint, face or the phone screen lock.', 'Vân tay, khuôn mặt hoặc khoá màn hình của máy.'),
  lockAfter: S('Ask again after', 'Hỏi lại sau'),
  lockEveryOpen: S('Every time the app opens', 'Mỗi lần mở app'),
  lockMinutes15: S('15 minutes', '15 phút'),
  lockHour1: S('1 hour (default)', '1 giờ (mặc định)'),
  lockHours4: S('4 hours', '4 giờ'),
  lockSaveFailed: S('Could not save the setting.', 'Không lưu được cài đặt.'),
}
