const en = {
  launcherV93: {
    agents: 'Agent',
    hint: {
      claude: 'Claude Code',
      codex: 'Codex CLI',
      deepseek: 'Codewhale',
      shell: 'Plain terminal',
    },
    recent: 'Recent projects',
    resume: 'Reopen unfinished sessions',
    resumeFor: 'in {project}',
    resumeAria: 'Resume {name}, ended {when}',
    resumeNone: 'No ended session to resume here.',
    keys: '← → agent · ↑ ↓ list · Enter open · Esc close',
    more: 'More',
  },
}

const vi: typeof en = {
  launcherV93: {
    agents: 'Agent',
    hint: {
      claude: 'Claude Code',
      codex: 'Codex CLI',
      deepseek: 'Codewhale',
      shell: 'Terminal thường',
    },
    recent: 'Project gần đây',
    resume: 'Mở lại session dở',
    resumeFor: 'trong {project}',
    resumeAria: 'Resume {name}, kết thúc {when}',
    resumeNone: 'Không có session đã kết thúc để resume ở đây.',
    keys: '← → agent · ↑ ↓ danh sách · Enter mở · Esc đóng',
    more: 'Khác',
  },
}

export default { en, vi }
