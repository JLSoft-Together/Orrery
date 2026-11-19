export interface DiskAppsText {
  uninstallTitle: (name: string) => string
  uninstallDetail: (cmd: string) => string
  uninstallConfirm: string
  trashTitle: (name: string) => string
  trashDetail: (path: string) => string
  trashConfirm: string
  killTitle: (n: number, name: string) => string
  killDetail: (lines: string[]) => string
  killConfirm: (n: number) => string
  cancel: string
}

const en: DiskAppsText = {
  uninstallTitle: (name) => `Uninstall ${name}?`,
  uninstallDetail: (cmd) => `Orrery will start the app's own uninstaller with this command:\n\n${cmd}\n\nThe uninstaller may ask for administrator rights. Nothing else is run.`,
  uninstallConfirm: 'Start uninstaller',
  trashTitle: (name) => `Move ${name} to the Trash?`,
  trashDetail: (p) => `${p}\n\nApp data in ~/Library stays until you clean leftovers.`,
  trashConfirm: 'Move to Trash',
  killTitle: (n, name) => (n === 1 ? `End ${name}?` : `End ${n} processes?`),
  killDetail: (lines) => `${lines.join('\n')}\n\nUnsaved work in these processes will be lost.`,
  killConfirm: (n) => (n === 1 ? 'End process' : `End ${n} processes`),
  cancel: 'Cancel',
}

const vi: DiskAppsText = {
  uninstallTitle: (name) => `Gỡ ${name}?`,
  uninstallDetail: (cmd) => `Orrery sẽ mở trình gỡ của chính app này bằng lệnh:\n\n${cmd}\n\nTrình gỡ có thể xin quyền quản trị. Không chạy lệnh nào khác.`,
  uninstallConfirm: 'Mở trình gỡ',
  trashTitle: (name) => `Chuyển ${name} vào Thùng rác?`,
  trashDetail: (p) => `${p}\n\nDữ liệu app trong ~/Library vẫn còn cho tới khi bạn dọn sót.`,
  trashConfirm: 'Chuyển vào Thùng rác',
  killTitle: (n, name) => (n === 1 ? `Kết thúc ${name}?` : `Kết thúc ${n} process?`),
  killDetail: (lines) => `${lines.join('\n')}\n\nViệc chưa lưu trong các process này sẽ mất.`,
  killConfirm: (n) => (n === 1 ? 'Kết thúc process' : `Kết thúc ${n} process`),
  cancel: 'Huỷ',
}

export function diskAppsText(locale: string): DiskAppsText {
  return locale === 'en' ? en : vi
}
