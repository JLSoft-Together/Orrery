const en = {
  explorerV93: {
    edit: 'edit',
    read: 'read',
    conflict: '{n} agents editing',
    tipEdit: '{agent} ({name}) edited at {time}',
    tipRead: '{agent} ({name}) read at {time}',
    folderEdit: 'An agent is editing a file inside',
    folderRead: 'An agent is reading a file inside',
    folderConflict: 'Several agents are editing the same file inside',
    agentFallback: 'agent',
  },
}

const vi: typeof en = {
  explorerV93: {
    edit: 'sửa',
    read: 'đọc',
    conflict: '{n} agent cùng sửa',
    tipEdit: '{agent} ({name}) sửa lúc {time}',
    tipRead: '{agent} ({name}) đọc lúc {time}',
    folderEdit: 'Có agent đang sửa file bên trong',
    folderRead: 'Có agent đang đọc file bên trong',
    folderConflict: 'Nhiều agent cùng sửa một file bên trong',
    agentFallback: 'agent',
  },
}

export default { en, vi }
