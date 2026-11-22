const en = {
  footerSys: {
    label: 'This computer',
    cpu: 'CPU',
    ram: 'RAM',
    cpuTip: 'CPU {pct}%, {threads} threads',
    ramTip: 'RAM {used} / {total} GB',
    high: 'High',
    aria: 'CPU and RAM of this computer, open Resources',
    setting: 'Show CPU and RAM in the footer',
    settingHint: 'Shows how busy this computer is (CPU and RAM) at the right of the footer, sampled every 3 seconds while Orrery is visible. From 85% the number turns to a warning color.',
  },
}

const vi: typeof en = {
  footerSys: {
    label: 'Máy này',
    cpu: 'CPU',
    ram: 'RAM',
    cpuTip: 'CPU {pct}%, {threads} luồng',
    ramTip: 'RAM {used} / {total} GB',
    high: 'Cao',
    aria: 'CPU và RAM của máy này, mở Tài nguyên',
    setting: 'Hiện CPU / RAM ở footer',
    settingHint: 'Hiện mức tải của máy (CPU và RAM) ở bên phải footer, lấy mẫu mỗi 3 giây khi Orrery đang hiện. Từ 85% số đổi sang màu cảnh báo.',
  },
}

export default { en, vi }
