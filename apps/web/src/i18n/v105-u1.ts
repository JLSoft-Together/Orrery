const en = {
  v105u1: {
    status: { working: 'Working', waiting: 'Waiting', idle: 'Idle', error: 'Error' },
    since: {
      working: 'running {t}',
      waiting: 'waiting {t}',
      idle: 'idle {t}',
      error: 'failed {t}',
    },
    unit: { s: '{n}s', m: '{n} min', h: '{n}h', d: '{n}d' },
    focus: 'Focus pane',
    close: 'Close pane',
  },
}

const vi = {
  v105u1: {
    status: { working: 'Đang chạy', waiting: 'Chờ bạn', idle: 'Rảnh', error: 'Lỗi' },
    since: {
      working: 'chạy {t}',
      waiting: 'chờ {t}',
      idle: 'rảnh {t}',
      error: 'lỗi {t}',
    },
    unit: { s: '{n}s', m: '{n} phút', h: '{n} giờ', d: '{n} ngày' },
    focus: 'Mở pane',
    close: 'Đóng pane',
  },
}

export default { en, vi }
