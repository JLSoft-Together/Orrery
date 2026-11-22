const en = {
  termSearchV109: {
    title: 'Find in every terminal',
    palette: 'Find in every open terminal',
    placeholder: 'Find in the scrollback of every open terminal',
    caseSensitive: 'Match case',
    regex: 'Regular expression',
    close: 'Close',
    intro: 'Type to search every open terminal. Only text already in the terminals is read.',
    searching: 'Searching…',
    none: 'No matches',
    invalid: 'Invalid regular expression',
    summary: '{n} matching line in {panes} pane | {n} matching lines in {panes} panes',
    truncated: 'Stopped at {n} results',
    unloaded: 'Not loaded',
    unloadedHint: 'This pane has no output in memory yet. Open it or start it, then search again.',
    count: '{n} match | {n} matches',
    results: 'Results',
    hints: { move: 'Move', open: 'Go to line', close: 'Close' },
  },
}

const vi = {
  termSearchV109: {
    title: 'Tìm trong mọi terminal',
    palette: 'Tìm trong mọi terminal',
    placeholder: 'Tìm trong scrollback của mọi terminal đang mở',
    caseSensitive: 'Phân biệt hoa thường',
    regex: 'Biểu thức chính quy (regex)',
    close: 'Đóng',
    intro: 'Gõ để tìm trong mọi terminal đang mở. Chỉ đọc chữ đang có trong terminal.',
    searching: 'Đang tìm…',
    none: 'Không có kết quả',
    invalid: 'Regex không hợp lệ',
    summary: '{n} dòng khớp trong {panes} pane',
    truncated: 'Dừng ở {n} kết quả',
    unloaded: 'Chưa tải',
    unloadedHint: 'Pane này chưa có output trong bộ nhớ. Mở hoặc chạy pane rồi tìm lại.',
    count: '{n} khớp',
    results: 'Kết quả',
    hints: { move: 'Di chuyển', open: 'Tới dòng', close: 'Đóng' },
  },
}

export default { en, vi }
