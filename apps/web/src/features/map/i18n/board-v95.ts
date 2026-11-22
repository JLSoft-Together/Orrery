const en = {
  boardV95: {
    summary: {
      running: '{n} agent running | {n} agents running',
      waiting: '{n} waiting for you',
      noneWaiting: 'none waiting for you',
    },
    groupBy: { label: 'Group by', status: 'Status', project: 'Project', agent: 'Agent' },
    strip: { expand: 'Show empty column {label}', collapse: 'Collapse empty column {label}' },
    subline: '{project}, {agent}',
    openWorkspace: 'Open in Workspace',
    viewDiff: 'View diff',
    viewDiffTitle: 'Open the uncommitted changes of this project in Workspace',
    editing: 'Editing {file}',
    reading: 'Reading {file}',
    context: 'Context {now}, warns at {max}',
    contextAria: 'Context used, warning threshold',
    lastTurn: 'Last turn changed {n} file | Last turn changed {n} files',
    branch: 'Branch {name}',
    elapsed: 'Running for {time}',
    done: {
      region: 'Finished today',
      files: '{n} file | {n} files',
      more: 'Show {n} more',
      less: 'Show fewer',
    },
  },
}

const vi = {
  boardV95: {
    summary: {
      running: '{n} agent đang chạy | {n} agent đang chạy',
      waiting: '{n} chờ chủ nhân',
      noneWaiting: 'không ai chờ chủ nhân',
    },
    groupBy: { label: 'Nhóm theo', status: 'Trạng thái', project: 'Project', agent: 'Agent' },
    strip: { expand: 'Mở cột rỗng {label}', collapse: 'Thu cột rỗng {label}' },
    subline: '{project}, {agent}',
    openWorkspace: 'Mở trong Workspace',
    viewDiff: 'Xem diff',
    viewDiffTitle: 'Mở thay đổi chưa commit của project này trong Workspace',
    editing: 'Đang sửa {file}',
    reading: 'Đang đọc {file}',
    context: 'Context {now}, cảnh báo ở {max}',
    contextAria: 'Context đã dùng, ngưỡng cảnh báo',
    lastTurn: 'Lượt cuối sửa {n} file | Lượt cuối sửa {n} file',
    branch: 'Nhánh {name}',
    elapsed: 'Đã chạy {time}',
    done: {
      region: 'Xong hôm nay',
      files: '{n} file | {n} file',
      more: 'Xem thêm {n}',
      less: 'Thu gọn',
    },
  },
}

export default { en, vi }
