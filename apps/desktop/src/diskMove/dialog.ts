import type { MessageBoxOptions } from 'electron'
import { formatBytes } from '../diskMap/dialog'
import type { MoveAsk } from './service'

const TEXT = {
  en: {
    move: { title: (size: string) => `Move ${size} to another drive?`, detail: (a: MoveAsk) => `From: ${a.source}\nTo: ${a.target}\n\nOrrery copies, checks the copy, leaves a link at the old place, then sends the old copy to the Recycle Bin. Close the tools that use this folder first.`, ok: 'Move' },
    back: { title: (size: string) => `Move ${size} back?`, detail: (a: MoveAsk) => `From: ${a.target}\nTo: ${a.source}\n\nOrrery copies the folder back, removes the link, then deletes the copy on the other drive.`, ok: 'Move back' },
    permanent: { title: () => 'The old copy cannot go to the Recycle Bin', detail: (a: MoveAsk) => `${a.target}\n\nThe folder already lives on the other drive behind the link. Delete the old copy for good?`, ok: 'Delete for good' },
    restore: { title: () => 'Restore the interrupted move?', detail: (a: MoveAsk) => `${a.source}\n\nOrrery puts the folder back where it was and deletes the unfinished copy at ${a.target}.`, ok: 'Restore' },
    finish: { title: () => 'Finish the move?', detail: (a: MoveAsk) => `${a.source}\n\nThe folder works from ${a.target}. Orrery removes what is left of the old copy.`, ok: 'Finish' },
    cancel: 'Cancel',
  },
  vi: {
    move: { title: (size: string) => `Chuyển ${size} sang ổ khác?`, detail: (a: MoveAsk) => `Từ: ${a.source}\nSang: ${a.target}\n\nOrrery copy, kiểm bản copy, để lại link ở chỗ cũ rồi đưa bản cũ vào Thùng rác. Đóng các tool đang dùng thư mục này trước.`, ok: 'Chuyển' },
    back: { title: (size: string) => `Chuyển ${size} về chỗ cũ?`, detail: (a: MoveAsk) => `Từ: ${a.target}\nVề: ${a.source}\n\nOrrery copy thư mục về, gỡ link rồi xoá bản ở ổ kia.`, ok: 'Chuyển về' },
    permanent: { title: () => 'Bản cũ không vào được Thùng rác', detail: (a: MoveAsk) => `${a.target}\n\nThư mục đã nằm ở ổ kia sau link. Xoá hẳn bản cũ?`, ok: 'Xoá hẳn' },
    restore: { title: () => 'Khôi phục lần chuyển bị dừng?', detail: (a: MoveAsk) => `${a.source}\n\nOrrery đưa thư mục về chỗ cũ và xoá bản copy dở ở ${a.target}.`, ok: 'Khôi phục' },
    finish: { title: () => 'Làm nốt lần chuyển?', detail: (a: MoveAsk) => `${a.source}\n\nThư mục đang chạy từ ${a.target}. Orrery xoá phần còn lại của bản cũ.`, ok: 'Làm nốt' },
    cancel: 'Huỷ',
  },
}

export function moveBoxOptions(ask: MoveAsk, locale: string): MessageBoxOptions {
  const t = locale.toLowerCase().startsWith('vi') ? TEXT.vi : TEXT.en
  const k = t[ask.kind]
  return {
    type: ask.kind === 'permanent' ? 'warning' : 'question',
    message: k.title(formatBytes(ask.bytes)),
    detail: k.detail(ask),
    buttons: [k.ok, t.cancel],
    defaultId: 1,
    cancelId: 1,
    noLink: true,
  }
}
