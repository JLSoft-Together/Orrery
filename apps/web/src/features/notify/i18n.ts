const en = {
  projectNotify: {
    menu: 'Notifications for this project',
    all: 'All',
    waiting: 'Only when waiting for me',
    off: 'Off',
    waitingHint: 'Notifications only when a session waits for you',
    offHint: 'Notifications off (the taskbar badge still counts)',
  },
  notifyGroup: {
    done: '{n} session finished | {n} sessions finished',
    waiting: '{n} needs you | {n} need you',
    more: ' (+{n} more)',
  },
  taskbarBadge: {
    setting: 'Waiting count on the taskbar',
    settingHint: 'Shows how many sessions wait for your answer on the taskbar icon (Dock on macOS) and flashes it once when a new one arrives while Orrery is not in front. Quiet hours turn the flash off.',
  },
}

const vi = {
  projectNotify: {
    menu: 'Thông báo của project này',
    all: 'Tất cả',
    waiting: 'Chỉ khi chờ chủ nhân',
    off: 'Tắt',
    waitingHint: 'Chỉ thông báo khi có session chờ chủ nhân',
    offHint: 'Đã tắt thông báo (số trên taskbar vẫn đếm)',
  },
  notifyGroup: {
    done: '{n} session xong',
    waiting: '{n} đang chờ chủ nhân',
    more: ' (+{n} session nữa)',
  },
  taskbarBadge: {
    setting: 'Số session đang chờ trên taskbar',
    settingHint: 'Hiện số session đang chờ chủ nhân trả lời trên icon taskbar (Dock trên macOS), và nháy 1 lần khi có session mới chờ lúc Orrery không ở phía trước. Giờ yên lặng tắt nháy.',
  },
}

export default { en, vi }
