import type { DiskHistoryEntry } from '@ccm/shared'
import { describe, expect, it, vi } from 'vitest'
import { deleteBoxOptions, formatBytes } from '../src/diskMap/dialog'
import { mapLimit, reasonOf, runDelete, type DeleteDeps } from '../src/diskMap/delete'
import { stringsFor } from '../src/strings'

const errno = (code: string): NodeJS.ErrnoException => Object.assign(new Error(code), { code })

function deps(over: Partial<DeleteDeps> = {}) {
  const records: DiskHistoryEntry[] = []
  const forgotten: string[][] = []
  const trashed: string[] = []
  const removed: string[] = []
  const d: DeleteDeps = {
    guard: async () => ({ platform: 'win32', home: 'C:\\Users\\dev', keep: ['C:\\Users\\dev\\Documents'], roots: ['D:\\work'] }),
    sizeOf: async (p) => (p.endsWith('nosize') ? null : p.length * 10),
    confirm: vi.fn(async () => true),
    confirmPermanent: vi.fn(async () => true),
    trash: async (p) => {
      if (p.endsWith('locked')) throw new Error('Failed to move item to trash')
      if (p.endsWith('gone')) throw errno('ENOENT')
      trashed.push(p)
    },
    remove: async (p) => {
      if (p.endsWith('locked')) throw errno('EBUSY')
      removed.push(p)
    },
    forget: async (paths) => {
      forgotten.push(paths)
    },
    record: async (e) => {
      records.push(e)
    },
    now: () => 1234,
    ...over,
  }
  return { d, records, forgotten, trashed, removed }
}

describe('runDelete', () => {
  it('trashes what it can, lists the rest, forgets and records once', async () => {
    const { d, records, forgotten, trashed } = deps()
    const r = await runDelete(['D:\\work\\a', 'D:\\work\\b\\locked', 'D:\\work\\a\\inner', 'C:\\Windows', 'rel', 'D:\\work\\c\\gone', 'D:\\work\\nosize'], false, d)
    expect(trashed.sort()).toEqual(['D:\\work\\a', 'D:\\work\\nosize'])
    expect(r.done).toEqual([{ path: 'D:\\work\\a', size: 90 }, { path: 'D:\\work\\nosize', size: 0 }])
    expect(r.freed).toBe(90)
    expect(r.permanent).toBe(false)
    expect(r.cancelled).toBeUndefined()
    expect(r.failed).toEqual(
      expect.arrayContaining([
        { path: 'C:\\Windows', reason: 'system' },
        { path: 'rel', reason: 'relative' },
        { path: 'D:\\work\\b\\locked', reason: 'Failed to move item to trash' },
        { path: 'D:\\work\\c\\gone', reason: 'ENOENT' },
      ]),
    )
    expect(r.failed).toHaveLength(4)
    expect(forgotten).toHaveLength(1)
    expect(forgotten[0]!.sort()).toEqual(['D:\\work\\a', 'D:\\work\\c\\gone', 'D:\\work\\nosize'])
    expect(records).toEqual([{ at: 1234, action: 'trash', count: 2, freed: 90, paths: ['D:\\work\\a', 'D:\\work\\nosize'], failed: 4 }])
    const ask = (d.confirm as ReturnType<typeof vi.fn>).mock.calls[0]![0]
    expect(ask).toEqual({ count: 4, bytes: 90 + 'D:\\work\\b\\locked'.length * 10 + 'D:\\work\\c\\gone'.length * 10, first: ['D:\\work\\a', 'D:\\work\\b\\locked', 'D:\\work\\c\\gone'], permanent: false })
    expect(d.confirmPermanent).not.toHaveBeenCalled()
  })

  it('stops on cancel without touching anything', async () => {
    const { d, records, forgotten, trashed } = deps({ confirm: vi.fn(async () => false) })
    const r = await runDelete(['D:\\work\\a'], false, d)
    expect(r).toEqual({ done: [], failed: [], freed: 0, permanent: false, cancelled: true })
    expect(trashed).toEqual([])
    expect(forgotten).toEqual([])
    expect(records).toEqual([])
  })

  it('asks twice for a permanent delete and cancels on the second no', async () => {
    const second = vi.fn(async () => false)
    const { d, removed } = deps({ confirmPermanent: second })
    const r = await runDelete(['D:\\work\\a'], true, d)
    expect(r.cancelled).toBe(true)
    expect(second).toHaveBeenCalledOnce()
    expect(removed).toEqual([])
  })

  it('deletes permanently with error codes for locked items', async () => {
    const { d, removed, records } = deps()
    const r = await runDelete(['D:\\work\\a', 'D:\\work\\x\\locked'], true, d)
    expect(removed).toEqual(['D:\\work\\a'])
    expect(r.failed).toEqual([{ path: 'D:\\work\\x\\locked', reason: 'EBUSY' }])
    expect(records[0]!.action).toBe('delete')
  })

  it('never asks when nothing passes the guard', async () => {
    const { d, records } = deps()
    const r = await runDelete(['C:\\', 'C:\\Users\\dev'], false, d)
    expect(r.failed.map((f) => f.reason)).toEqual(['root', 'protected'])
    expect(d.confirm).not.toHaveBeenCalled()
    expect(records).toEqual([])
  })

  it('keeps going when forget or record fail', async () => {
    const { d } = deps({ forget: async () => Promise.reject(new Error('core gone')), record: async () => Promise.reject(new Error('disk full')) })
    const r = await runDelete(['D:\\work\\a'], false, d)
    expect(r.done).toHaveLength(1)
  })
})

describe('delete helpers', () => {
  it('limits concurrency', async () => {
    let live = 0
    let peak = 0
    const out = await mapLimit([1, 2, 3, 4, 5, 6, 7, 8], 3, async (n) => {
      live++
      peak = Math.max(peak, live)
      await new Promise((r) => setTimeout(r, 5))
      live--
      return n * 2
    })
    expect(out).toEqual([2, 4, 6, 8, 10, 12, 14, 16])
    expect(peak).toBe(3)
    expect(await mapLimit([], 3, async () => 1)).toEqual([])
  })

  it('formats reasons and sizes', () => {
    expect(reasonOf(errno('EPERM'))).toBe('EPERM')
    expect(reasonOf(new Error('  a\n b '))).toBe('a b')
    expect(reasonOf('x'.repeat(300))).toHaveLength(200)
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(5 * 1024 ** 3)).toBe('5.0 GB')
    expect(formatBytes(300 * 1024 ** 2)).toBe('300 MB')
  })

  it('builds native dialogs with Cancel as default', () => {
    const en = stringsFor('en').diskMap
    const vi = stringsFor('vi').diskMap
    const ask = { count: 5, bytes: 2 * 1024 ** 3, first: ['D:\\a', 'D:\\b', 'D:\\c'], permanent: false }
    const box = deleteBoxOptions(ask, false, en, false)
    expect(box.message).toBe('Move 5 items to the Recycle Bin?')
    expect(box.detail).toBe('Total: 2.0 GB\n\nD:\\a\nD:\\b\nD:\\c\n…and 2 more')
    expect(box.buttons).toEqual(['Move to Recycle Bin', 'Cancel'])
    expect([box.defaultId, box.cancelId]).toEqual([1, 1])
    const perm = deleteBoxOptions({ ...ask, permanent: true }, false, vi, true)
    expect(perm.message).toBe('Xoá hẳn 5 mục?')
    expect(perm.buttons).toEqual(['Xoá hẳn', 'Huỷ'])
    const second = deleteBoxOptions({ ...ask, permanent: true }, true, en, true)
    expect(second.message).toBe('This cannot be undone')
    expect(second.defaultId).toBe(1)
    expect(deleteBoxOptions({ ...ask, count: 1, first: ['D:\\a'] }, false, en, true).message).toBe('Move 1 item to the Trash?')
  })
})
