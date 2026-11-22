const en = {
  paneV93: {
    orbit: {
      label: '{n} subagent | {n} subagents',
      title: 'Running subagents',
      aria: 'Subagents running: {n}',
      unnamed: 'Subagent',
      idleTool: 'Thinking',
      more: '+{n} more',
    },
    meter: {
      aria: 'Context used, warning threshold',
      title: 'Context {now}, warns at {max}',
      over: 'Context {now} is over the {max} limit',
      compact: 'Compact…',
      compactTitle: 'Write a handoff, then /compact',
    },
    turn: {
      aria: 'Last turn',
      title: 'Last turn',
      files: '{n} file | {n} files',
      more: '+{n} more',
      viewDiff: 'View diff',
      undo: 'Undo turn',
      undoTitle: 'Restore these files to how they were before the turn (asks first)',
      dismiss: 'Dismiss',
      undone: 'Turn undone',
    },
    boot: {
      starting: 'Starting {agent}…',
      slow: 'Starting {agent}… {s}s',
      hint: 'The first start can take a few seconds',
    },
  },
}

const vi: typeof en = {
  paneV93: {
    orbit: {
      label: '{n} subagent | {n} subagent',
      title: 'Subagent đang chạy',
      aria: 'Subagent đang chạy: {n}',
      unnamed: 'Subagent',
      idleTool: 'Đang nghĩ',
      more: '+{n} nữa',
    },
    meter: {
      aria: 'Context đã dùng, ngưỡng cảnh báo',
      title: 'Context {now}, cảnh báo ở {max}',
      over: 'Context {now} vượt limit {max}',
      compact: 'Compact…',
      compactTitle: 'Ghi handoff rồi /compact',
    },
    turn: {
      aria: 'Lượt vừa rồi',
      title: 'Lượt vừa rồi',
      files: '{n} file | {n} file',
      more: '+{n} nữa',
      viewDiff: 'Xem diff',
      undo: 'Hoàn tác lượt',
      undoTitle: 'Trả các file về như trước lượt (có hỏi xác nhận)',
      dismiss: 'Ẩn',
      undone: 'Đã hoàn tác lượt',
    },
    boot: {
      starting: 'Đang khởi động {agent}…',
      slow: 'Đang khởi động {agent}… {s}s',
      hint: 'Lần đầu mở có thể mất vài giây',
    },
  },
}

export default { en, vi }
