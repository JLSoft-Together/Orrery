const en = {
  shortcutsV109: {
    title: 'Keyboard shortcuts',
    hint: 'Click a shortcut, then press the new keys. Esc cancels. Keys that terminals or the system use cannot be taken.',
    press: 'Press the new keys',
    change: 'Change shortcut: {name}',
    reset: 'Reset',
    resetRow: 'Reset {name} to its default',
    resetAll: 'Reset all',
    changed: 'Changed',
    mruTitle: 'Recent tabs',
    mruCurrent: 'Current',
    reason: {
      incomplete: 'Press a letter, digit or function key together with a modifier key.',
      noModifier: 'Add Ctrl, Alt or Cmd: a bare key is typing.',
      terminal: 'Terminals use this key. Pick another.',
      system: 'The system uses this key. Pick another.',
      conflict: 'Already used by "{name}".',
    },
  },
}

const vi = {
  shortcutsV109: {
    title: 'Phím tắt',
    hint: 'Bấm vào phím tắt rồi nhấn tổ hợp mới. Esc để hủy. Phím terminal hoặc hệ thống đang dùng thì không đặt được.',
    press: 'Nhấn tổ hợp phím mới',
    change: 'Đổi phím tắt: {name}',
    reset: 'Về mặc định',
    resetRow: 'Đưa {name} về phím mặc định',
    resetAll: 'Về mặc định tất cả',
    changed: 'Đã đổi',
    mruTitle: 'Tab vừa dùng',
    mruCurrent: 'Đang mở',
    reason: {
      incomplete: 'Nhấn 1 chữ, số hoặc phím F cùng với phím bổ trợ.',
      noModifier: 'Thêm Ctrl, Alt hoặc Cmd: phím trần là đang gõ chữ.',
      terminal: 'Terminal đang dùng phím này. Chọn phím khác.',
      system: 'Hệ thống đang dùng phím này. Chọn phím khác.',
      conflict: 'Đã dùng cho "{name}".',
    },
  },
}

export default { en, vi }
