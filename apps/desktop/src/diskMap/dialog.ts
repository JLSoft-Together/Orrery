import type { MessageBoxOptions } from 'electron'
import type { Strings } from '../strings'
import type { DeleteAsk } from './delete'

export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = Math.max(0, bytes)
  let u = 0
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024
    u++
  }
  return u === 0 ? `${v} B` : `${v >= 100 ? Math.round(v) : v.toFixed(1)} ${units[u]}`
}

export function deleteBoxOptions(ask: DeleteAsk, second: boolean, text: Strings['diskMap'], mac: boolean): MessageBoxOptions {
  const size = formatBytes(ask.bytes)
  if (second) {
    return { type: 'warning', message: text.permanentTitle, detail: text.permanentDetail(ask.count, size, mac), buttons: [text.deleteConfirm, text.cancel], defaultId: 1, cancelId: 1, noLink: true }
  }
  return {
    type: ask.permanent ? 'warning' : 'question',
    message: ask.permanent ? text.deleteTitle(ask.count) : text.trashTitle(ask.count, mac),
    detail: text.detail(size, ask.first, Math.max(0, ask.count - ask.first.length)),
    buttons: [ask.permanent ? text.deleteConfirm : text.trashConfirm(mac), text.cancel],
    defaultId: 1,
    cancelId: 1,
    noLink: true,
  }
}
