const en = {
  cmdHist: {
    title: 'Command history',
    placeholder: 'Filter commands',
    empty: 'No command yet in this project',
    noMatch: 'No command matches',
    remove: 'Remove from history',
    close: 'Close',
    keys: 'Enter fills the line, Shift+Enter runs it, Shift+Delete removes, Esc closes',
    exit: 'exit {code}',
    row: '{cmd}, exit {code}, {time}',
  },
  termMarks: {
    manual: 'Bookmark',
    error: 'Error, exit {code}',
    server: 'Server ready, {host}',
    added: 'Bookmark added',
    removed: 'Bookmark removed',
    noPrev: 'No earlier bookmark',
    noNext: 'No later bookmark',
    toggle: 'Bookmark this line',
    toggleHint: '{key}',
    line: 'line {n}',
    clear: 'Clear bookmarks',
    history: 'Command history',
    historyHint: '{key}',
  },
}

const vi = {
  cmdHist: {
    title: 'Lịch sử lệnh',
    placeholder: 'Lọc lệnh',
    empty: 'Project này chưa có lệnh nào',
    noMatch: 'Không có lệnh khớp',
    remove: 'Xoá khỏi lịch sử',
    close: 'Đóng',
    keys: 'Enter điền vào dòng, Shift+Enter chạy luôn, Shift+Delete xoá, Esc đóng',
    exit: 'exit {code}',
    row: '{cmd}, exit {code}, {time}',
  },
  termMarks: {
    manual: 'Mốc',
    error: 'Lỗi, exit {code}',
    server: 'Server ready, {host}',
    added: 'Đã đặt mốc',
    removed: 'Đã bỏ mốc',
    noPrev: 'Không còn mốc phía trên',
    noNext: 'Không còn mốc phía dưới',
    toggle: 'Đặt mốc dòng này',
    toggleHint: '{key}',
    line: 'dòng {n}',
    clear: 'Xoá hết mốc',
    history: 'Lịch sử lệnh',
    historyHint: '{key}',
  },
}

export default { en, vi }
