import type { MessageBoxOptions } from 'electron'
import { formatBytes } from '../diskMap/dialog'
import type { OneTapAsk } from './service'

const TEXT = {
  en: {
    oneTapTitle: (size: string) => `Clean up ${size}?`,
    oneTapDetail: (perm: string, rest: string, bin: string, hasPerm: boolean, hasRest: boolean) =>
      [
        hasPerm ? `Temp files and browser caches (${perm}) are deleted permanently: apps make them again, so the ${bin} would only fill up.` : '',
        hasRest ? `The rest (${rest}) goes to the ${bin}.` : '',
      ]
        .filter(Boolean)
        .join('\n'),
    clean: (size: string) => `Clean ${size}`,
    allBin: (bin: string) => `Move everything to the ${bin}`,
    cancel: 'Cancel',
    binTitle: (bin: string) => `Empty the ${bin}?`,
    binDetail: (n: number, size: string) => `${n} items, ${size}. They cannot be restored.`,
    binOk: 'Empty',
    bin: (mac: boolean) => (mac ? 'Trash' : 'Recycle Bin'),
  },
  vi: {
    oneTapTitle: (size: string) => `Dọn ${size}?`,
    oneTapDetail: (perm: string, rest: string, _bin: string, hasPerm: boolean, hasRest: boolean) =>
      [
        hasPerm ? `File Temp và cache trình duyệt (${perm}) sẽ xoá hẳn, không vào Thùng rác: app tự tạo lại nên để trong Thùng rác chỉ tốn chỗ.` : '',
        hasRest ? `Phần còn lại (${rest}) vào Thùng rác.` : '',
      ]
        .filter(Boolean)
        .join('\n'),
    clean: (size: string) => `Dọn ${size}`,
    allBin: () => 'Tất cả vào Thùng rác',
    cancel: 'Huỷ',
    binTitle: () => 'Dọn sạch Thùng rác?',
    binDetail: (n: number, size: string) => `${n} mục, ${size}. Không khôi phục được.`,
    binOk: 'Dọn sạch',
    bin: () => 'Thùng rác',
  },
}

const pick = (locale: string) => (locale.toLowerCase().startsWith('vi') ? TEXT.vi : TEXT.en)

export function oneTapBoxOptions(ask: OneTapAsk, locale: string, mac: boolean): MessageBoxOptions {
  const t = pick(locale)
  const bin = t.bin(mac)
  const size = formatBytes(ask.bytes)
  const buttons = ask.permCount ? [t.clean(size), t.allBin(bin), t.cancel] : [t.clean(size), t.cancel]
  return {
    type: ask.permCount ? 'warning' : 'question',
    message: t.oneTapTitle(size),
    detail: t.oneTapDetail(formatBytes(ask.permBytes), formatBytes(ask.trashBytes), bin, ask.permCount > 0, ask.trashCount > 0),
    buttons,
    defaultId: buttons.length - 1,
    cancelId: buttons.length - 1,
    noLink: true,
  }
}

export function oneTapChoice(response: number, ask: OneTapAsk): 'clean' | 'trash' | 'cancel' {
  if (response === 0) return 'clean'
  if (ask.permCount && response === 1) return 'trash'
  return 'cancel'
}

export function binBoxOptions(ask: { bytes: number; count: number }, locale: string, mac: boolean): MessageBoxOptions {
  const t = pick(locale)
  return {
    type: 'warning',
    message: t.binTitle(t.bin(mac)),
    detail: t.binDetail(ask.count, formatBytes(ask.bytes)),
    buttons: [t.binOk, t.cancel],
    defaultId: 1,
    cancelId: 1,
    noLink: true,
  }
}
