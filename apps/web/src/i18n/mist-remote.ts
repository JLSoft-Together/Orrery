const en = {
  mistRemote: {
    chip: {
      idle: 'Phone',
      phones: '{n} phone | {n} phones',
      alert: '{n} IP blocked | {n} IPs blocked',
      pushOff: 'Push off',
      tipConnected: '{n} of {total} paired phones connected',
      tipNone: 'No phone paired yet',
      tipPush: 'Push on for {on} of {n}',
      tipPushNone: 'No phone has turned push on',
      tipBlocked: '{n} IP blocked in the last 24 hours | {n} IPs blocked in the last 24 hours',
      tipOpen: 'Click to open Settings, Remote control',
    },
    palette: {
      pair: 'Pair a phone',
      pairSub: 'Show the QR code in Settings, Remote control',
      pairOff: 'Turn on remote control first',
      devices: 'Paired phones',
      devicesSub: '{n} of {total} connected',
    },
  },
}

const vi: typeof en = {
  mistRemote: {
    chip: {
      idle: 'Điện thoại',
      phones: '{n} điện thoại | {n} điện thoại',
      alert: 'Chặn {n} IP | Chặn {n} IP',
      pushOff: 'Push tắt',
      tipConnected: '{n}/{total} điện thoại đã ghép đang nối',
      tipNone: 'Chưa ghép điện thoại nào',
      tipPush: 'Push bật cho {on}/{n} máy',
      tipPushNone: 'Chưa máy nào bật push',
      tipBlocked: 'Đã chặn {n} IP trong 24 giờ qua | Đã chặn {n} IP trong 24 giờ qua',
      tipOpen: 'Bấm để mở Cài đặt, Điều khiển từ xa',
    },
    palette: {
      pair: 'Ghép điện thoại',
      pairSub: 'Hiện mã QR trong Cài đặt, Điều khiển từ xa',
      pairOff: 'Bật điều khiển từ xa trước',
      devices: 'Thiết bị đã ghép',
      devicesSub: '{n}/{total} đang nối',
    },
  },
}

export default { en, vi }
