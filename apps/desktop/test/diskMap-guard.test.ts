import { describe, expect, it } from 'vitest'
import { checkDeletable, isUnder, normalizePath, topLevel, type GuardContext } from '../src/diskMap/guard'
import { cleanChildren, cleanScan, cleanSearch, isAbsolutePath } from '../src/diskMap/map'

const win: GuardContext = {
  platform: 'win32',
  home: 'C:\\Users\\dev',
  keep: ['C:\\Users\\dev\\Documents', 'C:\\Users\\dev\\Desktop', 'C:\\Users\\dev\\AppData\\Local\\ccm'],
  roots: ['C:\\', 'D:\\work'],
}
const mac: GuardContext = { platform: 'darwin', home: '/Users/dev', keep: ['/Users/dev/Documents', '/Users/dev/Desktop'], roots: ['/'] }
const linux: GuardContext = { platform: 'linux', home: '/home/dev', keep: ['/home/dev/Documents'], roots: ['/'] }

describe('diskMap guard (windows)', () => {
  it('rejects bad input, relative paths and drive roots', () => {
    expect(checkDeletable(42, win)).toBe('invalid')
    expect(checkDeletable('', win)).toBe('invalid')
    expect(checkDeletable('C:\\a\0b', win)).toBe('invalid')
    expect(checkDeletable('foo\\bar', win)).toBe('relative')
    expect(checkDeletable('\\foo', win)).toBe('relative')
    expect(checkDeletable('C:foo', win)).toBe('relative')
    expect(checkDeletable('C:\\', win)).toBe('root')
    expect(checkDeletable('c:/', win)).toBe('root')
    expect(checkDeletable('D:\\', win)).toBe('root')
    expect(checkDeletable('\\\\server\\share', win)).toBe('root')
    expect(checkDeletable('\\\\server\\share\\', win)).toBe('root')
  })

  it('rejects system folders and their insides, Program Files itself', () => {
    expect(checkDeletable('C:\\Windows', win)).toBe('system')
    expect(checkDeletable('c:\\windows\\System32\\drivers', win)).toBe('system')
    expect(checkDeletable('C:\\work\\..\\Windows\\Temp', win)).toBe('system')
    expect(checkDeletable('C:\\Program Files', win)).toBe('system')
    expect(checkDeletable('C:\\Program Files (x86)\\', win)).toBe('system')
    expect(checkDeletable('C:\\ProgramData', win)).toBe('system')
    expect(checkDeletable('C:\\$Recycle.Bin\\S-1-5', win)).toBe('system')
    expect(checkDeletable('C:\\Users', win)).toBe('system')
    expect(checkDeletable('C:\\Program Files\\OldApp', win)).toBeNull()
  })

  it('protects home, Documents, Desktop and app data themselves but not their contents', () => {
    expect(checkDeletable('C:\\Users\\dev', win)).toBe('protected')
    expect(checkDeletable('c:\\users\\DEV\\', win)).toBe('protected')
    expect(checkDeletable('C:\\Users\\dev\\Documents', win)).toBe('protected')
    expect(checkDeletable('C:\\Users\\dev\\Desktop', win)).toBe('protected')
    expect(checkDeletable('C:\\Users\\dev\\AppData', win)).toBe('protected')
    expect(checkDeletable('C:\\Users\\dev\\AppData\\Local\\ccm', win)).toBe('protected')
    expect(checkDeletable('C:\\Users\\dev\\Documents\\old.zip', win)).toBeNull()
    expect(checkDeletable('C:\\Users\\dev\\Desktop\\Ảnh cũ 🎉', win)).toBeNull()
    expect(checkDeletable('C:\\Users\\dev\\node_modules', win)).toBeNull()
  })

  it('requires the path inside an indexed root', () => {
    expect(checkDeletable('D:\\work\\big', win)).toBeNull()
    expect(checkDeletable('D:\\workspace\\big', win)).toBe('outside')
    expect(checkDeletable('E:\\x', win)).toBe('outside')
    expect(checkDeletable('C:\\x', { ...win, roots: [] })).toBe('outside')
  })

  it('accepts long and unicode paths', () => {
    const long = `D:\\work\\${'thư mục dài '.repeat(40)}\\file.bin`
    expect(long.length).toBeGreaterThan(400)
    expect(checkDeletable(long, win)).toBeNull()
  })

  it('normalizes and nests case-insensitively', () => {
    expect(normalizePath('c:\\a\\b\\\\', 'win32')).toBe('C:\\a\\b')
    expect(isUnder('C:\\A\\b', 'c:\\a', 'win32')).toBe(true)
    expect(isUnder('C:\\ab', 'C:\\a', 'win32')).toBe(false)
    expect(topLevel(['D:\\w\\a\\x', 'D:\\w\\a', 'd:\\W\\A', 'D:\\w\\b'], 'win32')).toEqual(['D:\\w\\a', 'D:\\w\\b'])
  })
})

describe('diskMap guard (macOS / linux)', () => {
  it('blocks system folders on macOS', () => {
    expect(checkDeletable('/', mac)).toBe('root')
    expect(checkDeletable('/System/Library', mac)).toBe('system')
    expect(checkDeletable('/usr/local/lib', mac)).toBe('system')
    expect(checkDeletable('/Library', mac)).toBe('system')
    expect(checkDeletable('/Library/Caches/foo', mac)).toBeNull()
    expect(checkDeletable('/Applications', mac)).toBe('system')
    expect(checkDeletable('/Users', mac)).toBe('system')
    expect(checkDeletable('/users/DEV', mac)).toBe('protected')
    expect(checkDeletable('/Users/dev/Desktop', mac)).toBe('protected')
    expect(checkDeletable('/Users/dev/Library/Caches/x', mac)).toBeNull()
    expect(checkDeletable('relative/x', mac)).toBe('relative')
  })

  it('is case-sensitive on linux', () => {
    expect(checkDeletable('/etc/passwd', linux)).toBe('system')
    expect(checkDeletable('/home', linux)).toBe('system')
    expect(checkDeletable('/home/dev', linux)).toBe('protected')
    expect(checkDeletable('/home/Dev', linux)).toBeNull()
    expect(checkDeletable('/home/dev/.cache/pip', linux)).toBeNull()
  })
})

describe('diskMap request validation', () => {
  it('checks absolute paths per platform', () => {
    expect(isAbsolutePath('C:\\x', 'win32')).toBe(true)
    expect(isAbsolutePath('\\\\srv\\share\\x', 'win32')).toBe(true)
    expect(isAbsolutePath('/x', 'win32')).toBe(false)
    expect(isAbsolutePath('/x', 'darwin')).toBe(true)
    expect(isAbsolutePath('x', 'linux')).toBe(false)
  })

  it('cleans scan / children / search requests', () => {
    expect(cleanScan({ roots: ['C:\\', 'C:\\'], level: 'max', excludes: ['node_modules'] }, 'win32')).toEqual({ roots: ['C:\\'], level: 'max', excludes: ['node_modules'] })
    expect(cleanScan({ roots: ['C:\\'], level: 'turbo' }, 'win32')).toEqual({ roots: ['C:\\'], level: 'balanced', excludes: [] })
    expect(cleanScan({ roots: ['rel'] }, 'win32')).toBeNull()
    expect(cleanScan({ roots: [] }, 'win32')).toBeNull()
    expect(cleanScan({ roots: ['C:\\'], excludes: [1] }, 'win32')).toBeNull()
    expect(cleanChildren({ path: '', sort: 'name', limit: 20 }, 'win32')).toEqual({ path: '', sort: 'name', limit: 20 })
    expect(cleanChildren({ path: 'C:\\a', limit: 1e9 }, 'win32')).toEqual({ path: 'C:\\a', sort: 'size', limit: 10_000 })
    expect(cleanChildren({ path: 'a' }, 'win32')).toBeNull()
    expect(cleanSearch({ query: 'x', minSize: 10, kind: 'video', only: 'file', under: 'C:\\a', sort: 'mtime' }, 'win32')).toEqual({ query: 'x', minSize: 10, kind: 'video', only: 'file', under: 'C:\\a', sort: 'mtime', limit: 100 })
    expect(cleanSearch({ kind: 'weird' }, 'win32')).toBeNull()
    expect(cleanSearch({ minSize: -1 }, 'win32')).toBeNull()
    expect(cleanSearch({ under: 'rel' }, 'win32')).toBeNull()
    expect(cleanSearch(undefined, 'win32')).toEqual({ limit: 100 })
  })
})
