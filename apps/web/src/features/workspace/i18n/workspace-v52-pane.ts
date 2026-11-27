const en = {
  paneHead: {
    more: 'More actions',
    zoom: 'Zoom pane ({key})',
    unzoom: 'Show all panes ({key})',
    hidden: '{n} hidden pane - show all | {n} hidden panes - show all',
    branch: 'Worktrees, {branch}',
    attachCount: '{label} ({n})',
    compact: {
      label: 'Context {pct}%, Compact…',
      title: 'Context is {pct}% full. Compact… asks the agent to write a handoff file, then stops so you can run /compact',
    },
    phone: {
      label: 'Open on phone',
      sent: 'Sent to {n} phone | Sent to {n} phones',
      none: 'No phone has push turned on',
      limit: 'Sent too many times, try again later',
      off: 'Orrery Remote is off',
      notFound: 'This pane is not open anymore',
      unavailable: 'Could not send to the phone',
    },
  },
}

const vi: typeof en = {
  paneHead: {
    more: 'Thao tác khác',
    zoom: 'Phóng to pane ({key})',
    unzoom: 'Hiện lại mọi pane ({key})',
    hidden: '{n} pane đang ẩn - hiện tất cả | {n} pane đang ẩn - hiện tất cả',
    branch: 'Worktree, {branch}',
    attachCount: '{label} ({n})',
    compact: {
      label: 'Ngữ cảnh {pct}%, Compact…',
      title: 'Ngữ cảnh đã dùng {pct}%. Compact… nhờ agent viết file handoff rồi dừng lại để bạn chạy /compact',
    },
    phone: {
      label: 'Mở trên điện thoại',
      sent: 'Đã gửi tới {n} điện thoại | Đã gửi tới {n} điện thoại',
      none: 'Chưa có điện thoại nào bật push',
      limit: 'Đã gửi quá nhiều, thử lại sau',
      off: 'Orrery Remote đang tắt',
      notFound: 'Pane này không còn mở',
      unavailable: 'Không gửi được tới điện thoại',
    },
  },
}

export default { en, vi }
