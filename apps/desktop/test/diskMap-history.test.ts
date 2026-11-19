import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { cleanHistoryEntry, DiskHistory } from '../src/diskMap/history'

const dirs: string[] = []
const tmp = (): string => {
  const d = mkdtempSync(path.join(os.tmpdir(), 'diskmap-hist-'))
  dirs.push(d)
  return d
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

describe('DiskHistory', () => {
  it('appends JSONL and reads newest first with a limit', async () => {
    const dir = path.join(tmp(), 'disk')
    const h = new DiskHistory(dir)
    expect(await h.read(10)).toEqual([])
    await Promise.all([1, 2, 3].map((i) => h.append({ at: i, action: i === 2 ? 'delete' : 'trash', count: i, freed: i * 100, paths: [`C:\\x${i}`], failed: 0 })))
    const all = await h.read(10)
    expect(all.map((e) => e.at)).toEqual([3, 2, 1])
    expect(all[1]).toEqual({ at: 2, action: 'delete', count: 2, freed: 200, paths: ['C:\\x2'], failed: 0 })
    expect((await h.read(2)).map((e) => e.at)).toEqual([3, 2])
    expect(readFileSync(h.file, 'utf8').trim().split('\n')).toHaveLength(3)
  })

  it('skips broken lines and caps stored paths', async () => {
    const dir = tmp()
    const h = new DiskHistory(dir)
    await h.append({ at: 1, action: 'trash', count: 60, freed: 1, paths: Array.from({ length: 60 }, (_, i) => `/p/${i}`), failed: 2 })
    writeFileSync(h.file, `${readFileSync(h.file, 'utf8')}not json\n{"at":"x"}\n\n`)
    await h.append({ at: 2, action: 'delete', count: 1, freed: 5, paths: ['/q'], failed: 0 })
    const got = await h.read(50)
    expect(got.map((e) => e.at)).toEqual([2, 1])
    expect(got[1]!.paths).toHaveLength(50)
    expect(got[1]!.failed).toBe(2)
  })

  it('trims the file when it grows past 1 MB', async () => {
    const h = new DiskHistory(tmp())
    const big = 'x'.repeat(600)
    for (let i = 0; i < 2000; i++) writeFileSync(h.file, `${JSON.stringify({ at: i, action: 'trash', count: 1, freed: 1, paths: [big], failed: 0 })}\n`, { flag: 'a' })
    await h.append({ at: 9999, action: 'trash', count: 1, freed: 1, paths: ['/last'], failed: 0 })
    expect(statSync(h.file).size).toBeLessThan(1024 * 1024)
    const got = await h.read(5)
    expect(got[0]!.at).toBe(9999)
    expect(readFileSync(h.file, 'utf8').trim().split('\n')).toHaveLength(1000)
  })

  it('validates entries', () => {
    expect(cleanHistoryEntry({ at: 1, action: 'nuke', count: 1, freed: 1, paths: [], failed: 0 })).toBeNull()
    expect(cleanHistoryEntry({ at: -1, action: 'trash', count: 1, freed: 1, paths: [], failed: 0 })).toBeNull()
    expect(cleanHistoryEntry([1])).toBeNull()
    expect(cleanHistoryEntry({ at: 1, action: 'move', count: 1, freed: 9, paths: ['C:\a', 'D:\a'], failed: 0 })?.action).toBe('move')
    expect(cleanHistoryEntry({ at: 1, action: 'trash', count: 1, freed: 1, paths: [1, 'a'], failed: 0 })?.paths).toEqual(['a'])
  })
})
