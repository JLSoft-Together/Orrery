const en = {
  shellDone: {
    setting: 'Tell me when a long shell command ends',
    settingHint: 'When a command in a shell pane runs at least this long and ends while the pane is out of sight, Orrery shows a notification with the exit code and time, and a dot on the tab. Needs shell command marks.',
    seconds: 'Minimum run time (seconds)',
    okTitle: 'Command finished (exit {code}, {time})',
    failTitle: 'Command failed (exit {code}, {time})',
    s: '{s} s',
    ms: '{m} min {s} s',
    hm: '{h} h {m} min',
    open: 'Open',
    tabFail: 'Command failed with exit {code}',
  },
}

const vi: typeof en = {
  shellDone: {
    setting: 'Báo khi lệnh shell chạy lâu xong',
    settingHint: 'Lệnh trong pane shell chạy từ ngần này trở lên mà xong lúc pane không đang được nhìn thì Orrery hiện thông báo kèm exit code và thời gian, thêm chấm trên tab. Cần bật đánh dấu lệnh shell.',
    seconds: 'Thời gian chạy tối thiểu (giây)',
    okTitle: 'Lệnh xong (exit {code}, {time})',
    failTitle: 'Lệnh lỗi (exit {code}, {time})',
    s: '{s} giây',
    ms: '{m} phút {s} giây',
    hm: '{h} giờ {m} phút',
    open: 'Mở',
    tabFail: 'Lệnh lỗi, exit {code}',
  },
}

export default { en, vi }
