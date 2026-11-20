const en = {
  deepseekSetup: {
    intro: 'Claude Code and Codex can hand side tasks to a DeepSeek pane with the Orrery tool open_session; DeepSeek hands the result back with hand_back.',
    status: 'DeepSeek CLI (Codewhale)',
    apiKey: 'DeepSeek API key',
    apiKeyHint: 'Saved by Codewhale in its own credential store. Orrery never keeps or shows it.',
    logout: 'Log out',
    logoutHint: 'Deletes the saved DeepSeek key from Codewhale.',
    telemetry: 'Send usage counts to Codewhale',
    telemetryHint: 'Aggregate counts only (PostHog), on by default in Codewhale. Turn off to stop them.',
    sync: 'Copy rules, skills and MCP from Claude',
    syncHint: 'Writes the Orrery hand-off workflow and your ~/.claude/CLAUDE.md into ~/.codewhale/AGENTS.md, lets Codewhale read project CLAUDE.md files, adds your Claude MCP servers and the orrery-handoff / orrery-delegate skills. Existing files are backed up first.',
    agentNotice: 'An agent asked for a DeepSeek pane, but DeepSeek is not set up on this computer yet.',
    openSetup: 'Set up DeepSeek',
    launchNote: {
      install: 'The DeepSeek CLI is not installed yet.',
      download: 'Codewhale still has to download its program.',
      login: 'No DeepSeek API key yet.',
      stale: 'Rules changed in Claude since the last copy.',
    },
    update: {
      check: 'Check for update',
    },
  },
}

const vi: typeof en = {
  deepseekSetup: {
    intro: 'Claude Code và Codex có thể giao việc phụ cho một pane DeepSeek bằng công cụ open_session của Orrery; DeepSeek bàn giao kết quả lại bằng hand_back.',
    status: 'DeepSeek CLI (Codewhale)',
    apiKey: 'API key DeepSeek',
    apiKeyHint: 'Codewhale lưu trong kho thông tin đăng nhập riêng. Orrery không giữ và không hiện key.',
    logout: 'Đăng xuất',
    logoutHint: 'Xoá key DeepSeek đã lưu trong Codewhale.',
    telemetry: 'Gửi số liệu sử dụng cho Codewhale',
    telemetryHint: 'Chỉ số đếm tổng hợp (PostHog), Codewhale bật sẵn. Tắt để ngừng gửi.',
    sync: 'Chép rule, skill và MCP từ Claude',
    syncHint: 'Ghi quy trình bàn giao của Orrery và ~/.claude/CLAUDE.md của bạn vào ~/.codewhale/AGENTS.md, cho Codewhale đọc CLAUDE.md của dự án, thêm MCP server của Claude và skill orrery-handoff / orrery-delegate. File đã có được sao lưu trước.',
    agentNotice: 'Một agent muốn mở pane DeepSeek, nhưng DeepSeek chưa được cài đặt trên máy này.',
    openSetup: 'Cài đặt DeepSeek',
    launchNote: {
      install: 'Chưa cài DeepSeek CLI.',
      download: 'Codewhale còn phải tải chương trình.',
      login: 'Chưa có API key DeepSeek.',
      stale: 'Rule trong Claude đã đổi từ lần chép trước.',
    },
    update: {
      check: 'Kiểm tra bản mới',
    },
  },
}

export default { en, vi }
