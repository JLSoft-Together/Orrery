const en = {
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
