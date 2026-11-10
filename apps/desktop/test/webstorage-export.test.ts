import { mkdtempSync, readFileSync, rmSync, existsSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { exportText, exportWebStorage, WEBSTORAGE_EXPORT_FILE, DUMP_ALL_LOCAL_JS } from '../src/webstorageExport'

const dirs: string[] = []
const tmp = (): string => {
  const d = mkdtempSync(path.join(os.tmpdir(), 'ccm-wsx-'))
  dirs.push(d)
  return d
}

afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

describe('webstorage export for the Tauri shell', () => {
  it('keeps only string values in the shape the shell reads', () => {
    const text = exportText('http://127.0.0.1:4317', { 'ccm.a': '1', 'ccm.b': 2, other: 'x' }, new Date('2026-11-10T00:00:00Z'))
    expect(JSON.parse(text!)).toEqual({ version: 1, origin: 'http://127.0.0.1:4317', savedAt: '2026-11-10T00:00:00.000Z', local: { 'ccm.a': '1', other: 'x' } })
    expect(exportText('o', null, new Date())).toBeNull()
    expect(exportText('o', ['a'], new Date())).toBeNull()
  })

  it('writes the file into the data dir', async () => {
    const dir = tmp()
    expect(await exportWebStorage(async () => ({ 'ccm.theme': 'dark' }), dir, 'http://127.0.0.1:4317')).toBe(true)
    const saved = JSON.parse(readFileSync(path.join(dir, WEBSTORAGE_EXPORT_FILE), 'utf8'))
    expect(saved.local).toEqual({ 'ccm.theme': 'dark' })
  })

  it('gives up on a page that throws or does not answer', async () => {
    const dir = tmp()
    expect(await exportWebStorage(() => Promise.reject(new Error('gone')), dir, 'o')).toBe(false)
    expect(await exportWebStorage(() => new Promise(() => {}), dir, 'o', 20)).toBe(false)
    expect(existsSync(path.join(dir, WEBSTORAGE_EXPORT_FILE))).toBe(false)
  })

  it('fails quietly when the data dir is missing', async () => {
    expect(await exportWebStorage(async () => ({}), path.join(tmp(), 'missing', 'deeper'), 'o')).toBe(false)
  })

  it('dumps every key, not only ccm.*', () => {
    expect(DUMP_ALL_LOCAL_JS).not.toContain("startsWith('ccm.')")
  })
})
