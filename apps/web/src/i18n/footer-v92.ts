const en = {
  wsFooter: {
    label: 'Workspace status',
    running: '{n} running',
    runningAria: '{n} agents running, focus the first',
    waiting: '{n} waiting',
    waitingAria: '{n} agents waiting for you, focus the first',
    ports: 'Port {list}',
    portsAria: 'Dev server ports: {list}',
    branchChanges: '{branch} · {n} changes',
    branchOnly: '{branch}',
    changesOnly: '{n} changes',
    branchAria: 'Branch {branch}, {n} changed files',
    branchAriaNoCount: 'Branch {branch}',
    changesAria: '{n} changed files',
    context: 'Context {used}, warns at {limit}',
    contextOnly: 'Context {used}',
    contextAria: 'Context {used} tokens, warns at {limit}',
    contextAriaOnly: 'Context {used} tokens',
  },
}

const vi: typeof en = {
  wsFooter: {
    label: 'Trạng thái workspace',
    running: '{n} chạy',
    runningAria: '{n} agent đang chạy, focus agent đầu tiên',
    waiting: '{n} chờ',
    waitingAria: '{n} agent đang chờ chủ nhân, focus agent đầu tiên',
    ports: 'Port {list}',
    portsAria: 'Port dev server: {list}',
    branchChanges: '{branch} · {n} thay đổi',
    branchOnly: '{branch}',
    changesOnly: '{n} thay đổi',
    branchAria: 'Branch {branch}, {n} file thay đổi',
    branchAriaNoCount: 'Branch {branch}',
    changesAria: '{n} file thay đổi',
    context: 'Context {used}, cảnh báo ở {limit}',
    contextOnly: 'Context {used}',
    contextAria: 'Context {used} token, cảnh báo ở {limit}',
    contextAriaOnly: 'Context {used} token',
  },
}

export default { en, vi }
