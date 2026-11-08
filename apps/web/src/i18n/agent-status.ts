// Top-level `agentStatus` of en.ts / vi.ts: workflow status chip on agent panes (docs §11ap).
const en = {
  agentStatus: {
    chip: 'Workflow: {phase}',
    progress: '{pct}% done',
    updated: 'Updated {when}',
    stale: 'No update for 30 minutes',
  },
}

const vi = {
  agentStatus: {
    chip: 'Workflow: {phase}',
    progress: 'Xong {pct}%',
    updated: 'Cập nhật {when}',
    stale: 'Không có cập nhật trong 30 phút',
  },
}

export default { en, vi }
