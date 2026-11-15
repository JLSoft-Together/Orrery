import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { e2eAutoConfirm } from '../src/diskMap/delete'
import { canonicalOf, checkDeletable, checkDeletableCanonical, type CanonicalIo, type GuardContext } from '../src/diskMap/guard'

const win: GuardContext = { platform: 'win32', home: 'C:\\Users\\dev', keep: ['C:\\Users\\dev\\Documents'], roots: ['C:\\', 'D:\\work', '\\\\localhost\\C$', '\\\\srv\\c$'] }
const mac: GuardContext = { platform: 'darwin', home: '/Users/dev', keep: ['/Users/dev/Tài liệu'], roots: ['/'], volumes: ['/', '/Volumes/Ext', '/Volumes/Data'] }
const linux: GuardContext = { platform: 'linux', home: '/home/dev', keep: [], roots: ['/'] }

describe('windows path variants that Windows folds onto protected folders', () => {
  it('rejects trailing dots and spaces in any segment', () => {
    expect(checkDeletable('C:\\Users\\dev.', win)).toBe('invalid')
    expect(checkDeletable('C:\\Users\\dev ', win)).toBe('invalid')
    expect(checkDeletable('C:\\Users.', win)).toBe('invalid')
    expect(checkDeletable('C:\\Users. \\dev', win)).toBe('invalid')
    expect(checkDeletable('C:\\Windows.\\Temp', win)).toBe('invalid')
    expect(checkDeletable('C:\\Users\\dev\\Documents...', win)).toBe('invalid')
    expect(checkDeletable('D:\\work\\a.\\b', win)).toBe('invalid')
  })

  it('rejects alternate data streams, wildcards and device prefixes', () => {
    expect(checkDeletable('D:\\work\\a:stream', win)).toBe('invalid')
    expect(checkDeletable('D:\\work\\a::$DATA', win)).toBe('invalid')
    expect(checkDeletable('D:\\work\\*', win)).toBe('invalid')
    expect(checkDeletable('D:\\work\\a?', win)).toBe('invalid')
    expect(checkDeletable('\\\\?\\C:\\Users\\dev', win)).toBe('relative')
    expect(checkDeletable('\\\\.\\C:\\Users\\dev', win)).toBe('relative')
    expect(checkDeletable('//?/C:/Users/dev', win)).toBe('relative')
    expect(checkDeletable('', win)).toBe('invalid')
    expect(checkDeletable('..\\Users', win)).toBe('relative')
  })

  it('maps admin shares onto the local drive', () => {
    expect(checkDeletable('\\\\localhost\\C$\\Users\\dev', win)).toBe('protected')
    expect(checkDeletable('\\\\localhost\\c$\\Users\\other', win)).toBe('protected')
    expect(checkDeletable('\\\\srv\\c$\\Windows\\System32', win)).toBe('system')
    expect(checkDeletable('\\\\localhost\\C$', win)).toBe('root')
    expect(checkDeletable('\\\\localhost\\C$\\Users\\dev\\big.iso', win)).toBeNull()
  })

  it('keeps ordinary names with dots, spaces and tildes', () => {
    expect(checkDeletable('D:\\work\\.gradle', win)).toBeNull()
    expect(checkDeletable('D:\\work\\a b\\c.d', win)).toBeNull()
    expect(checkDeletable('D:\\work\\~$report.docx', win)).toBeNull()
  })
})

describe('protected list', () => {
  it('windows: AppData roots, .ssh tree, OneDrive root, every profile', () => {
    for (const p of ['AppData', 'AppData\\Roaming', 'AppData\\Local', 'AppData\\LocalLow', 'appdata\\LOCAL\\', '.ssh', '.ssh\\id_ed25519', '.gnupg\\x', 'OneDrive', 'OneDrive - Acme']) {
      expect(checkDeletable(`C:\\Users\\dev\\${p}`, win), p).toBe('protected')
    }
    for (const p of ['C:\\Users\\Public', 'C:\\Users\\Default', 'C:\\Users\\other', 'D:\\Users\\x']) expect(checkDeletable(p, { ...win, roots: ['C:\\', 'D:\\'] }), p).toBe('protected')
    for (const p of ['C:\\Users\\dev\\AppData\\Local\\Temp\\x', 'C:\\Users\\dev\\AppData\\Roaming\\npm-cache', 'C:\\Users\\dev\\OneDrive\\big.zip', 'C:\\Users\\Public\\Videos\\x', 'C:\\Users\\other\\node_modules']) {
      expect(checkDeletable(p, win), p).toBeNull()
    }
  })

  it('macOS: Library root, iCloud and CloudStorage trees, keychains, volumes', () => {
    for (const p of ['/Users/dev/Library', '/Users/dev/Library/Mobile Documents', '/Users/dev/Library/Mobile Documents/com~apple~CloudDocs/a.pdf', '/Users/dev/Library/CloudStorage/GoogleDrive-x/y', '/Users/dev/Library/Keychains/login.keychain-db', '/Users/dev/.ssh/config', '/Users/other', '/Volumes/Ext', '/Volumes/Data', '/Volumes/Other']) {
      expect(checkDeletable(p, mac), p).toBe('protected')
    }
    expect(checkDeletable('/Users/dev/Ta\u0300i lie\u0323\u0302u'.normalize('NFD'), mac)).toBe('protected')
    for (const p of ['/Users/dev/Library/Caches/com.app', '/Users/dev/Library/Developer/Xcode/DerivedData', '/Volumes/Ext/big.mov']) expect(checkDeletable(p, mac), p).toBeNull()
  })

  it('linux: profiles and mounts', () => {
    for (const p of ['/home/other', '/mnt/data', '/media/dev', '/media/dev/usb', '/home/dev/.config', '/home/dev/.ssh/id_rsa']) expect(checkDeletable(p, linux), p).toBe('protected')
    expect(checkDeletable('/media/dev/usb/big.iso', linux)).toBeNull()
  })
})

const enoent = (): NodeJS.ErrnoException => Object.assign(new Error('ENOENT'), { code: 'ENOENT' })

function fakeIo(real: Record<string, string>, links: string[] = []): CanonicalIo {
  return {
    lstat: async (p) => {
      if (links.includes(p)) return { isSymbolicLink: () => true }
      if (p in real) return { isSymbolicLink: () => false }
      throw enoent()
    },
    realpath: async (p) => {
      if (p in real) return real[p]!
      throw enoent()
    },
  }
}

describe('canonical guard with a fake filesystem', () => {
  it('resolves 8.3 names and links in the middle', async () => {
    const io = fakeIo({ 'C:\\PROGRA~1': 'C:\\Program Files', 'C:\\Users\\DEV~1': 'C:\\Users\\dev', 'D:\\work\\lnk\\dev\\.ssh': 'C:\\Users\\dev\\.ssh', 'D:\\work\\x': 'D:\\work\\x' })
    expect(await checkDeletableCanonical('C:\\PROGRA~1', win, io)).toBe('system')
    expect(await checkDeletableCanonical('C:\\Users\\DEV~1', win, io)).toBe('protected')
    expect(await checkDeletableCanonical('D:\\work\\lnk\\dev\\.ssh', win, io)).toBe('protected')
    expect(await checkDeletableCanonical('D:\\work\\x', win, io)).toBeNull()
  })

  it('rejects what cannot be resolved and leftover short names', async () => {
    const io: CanonicalIo = { lstat: async () => ({ isSymbolicLink: () => false }), realpath: async () => Promise.reject(Object.assign(new Error('EPERM'), { code: 'EPERM' })) }
    expect(await checkDeletableCanonical('D:\\work\\x', win, io)).toBe('invalid')
    expect(await checkDeletableCanonical('D:\\work\\x', win, fakeIo({ 'D:\\work\\x': 'D:\\work\\ABCDEF~1\\x' }))).toBe('invalid')
    expect(await checkDeletableCanonical('D:\\work\\x', win, fakeIo({ 'D:\\work\\x': '\\\\?\\Volume{1234}\\x' }))).toBe('invalid')
    expect(await checkDeletableCanonical('D:\\work\\x', win, fakeIo({ 'D:\\work\\x': '\\\\?\\C:\\Users\\dev' }))).toBe('protected')
  })

  it('checks a link by its own name, not by its target', async () => {
    const io = fakeIo({ 'D:\\work': 'D:\\work' }, ['D:\\work\\home-link'])
    expect(await canonicalOf('D:\\work\\home-link', 'win32', io)).toEqual({ path: 'D:\\work\\home-link', link: true, missing: false })
    expect(await checkDeletableCanonical('D:\\work\\home-link', win, io)).toBeNull()
  })

  it('resolves the parent of a missing target', async () => {
    const io = fakeIo({ 'D:\\work\\lnk': 'C:\\Users\\dev' })
    expect(await canonicalOf('D:\\work\\lnk\\.ssh', 'win32', io)).toEqual({ path: 'C:\\Users\\dev\\.ssh', link: false, missing: true })
    expect(await checkDeletableCanonical('D:\\work\\lnk\\.ssh', win, io)).toBe('protected')
  })
})

describe('e2e auto confirm', () => {
  it('never applies to a packaged app', () => {
    expect(e2eAutoConfirm({ CCM_E2E_DISKMAP_CONFIRM: '1' }, true)).toBe(false)
    expect(e2eAutoConfirm({ CCM_E2E_DISKMAP_CONFIRM: '1' }, false)).toBe(true)
    expect(e2eAutoConfirm({}, false)).toBe(false)
  })
})

describe.runIf(process.platform === 'win32')('canonical guard on the real filesystem (windows)', () => {
  let base = ''
  const links: string[] = []
  const ctx = (): GuardContext => ({ platform: 'win32', home: path.join(base, 'home'), keep: [path.join(base, 'home', 'Documents')], roots: [base] })

  beforeAll(() => {
    base = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-guard-')))
    fs.mkdirSync(path.join(base, 'home', '.ssh'), { recursive: true })
    fs.mkdirSync(path.join(base, 'home', 'Documents'), { recursive: true })
    fs.mkdirSync(path.join(base, 'home', 'longprotectedfolder'), { recursive: true })
    fs.mkdirSync(path.join(base, 'scan', 'free'), { recursive: true })
    const lnk = path.join(base, 'scan', 'lnk')
    fs.symlinkSync(path.join(base, 'home'), lnk, 'junction')
    links.push(lnk)
    const users = path.join(base, 'scan', 'users')
    fs.symlinkSync('C:\\Users', users, 'junction')
    links.push(users)
  })

  afterAll(() => {
    for (const l of links) {
      try {
        fs.rmdirSync(l)
      } catch {
        fs.unlinkSync(l)
      }
    }
    if (links.every((l) => !fs.existsSync(l))) fs.rmSync(base, { recursive: true, force: true })
  })

  it('a junction in the middle does not escape the protected list', async () => {
    const p = path.join(base, 'scan', 'lnk', '.ssh')
    expect(checkDeletable(p, ctx())).toBeNull()
    expect(await checkDeletableCanonical(p, ctx())).toBe('protected')
    expect(await checkDeletableCanonical(path.join(base, 'scan', 'lnk', 'Documents'), ctx())).toBe('protected')
    expect(await checkDeletableCanonical(path.join(base, 'scan', 'lnk', 'Documents', 'gone.txt'), ctx())).toBeNull()
    expect(await checkDeletableCanonical(path.join(base, 'scan', 'free'), ctx())).toBeNull()
  })

  it('a junction to C:\\Users exposes no profile', async () => {
    const me = path.basename(os.homedir())
    expect(await checkDeletableCanonical(path.join(base, 'scan', 'users', me), { ...ctx(), home: os.homedir() })).toBe('protected')
    expect(await checkDeletableCanonical(path.join(base, 'scan', 'users', 'Public'), ctx())).toBe('protected')
  })

  it('the junction itself is checked by its own name', async () => {
    expect(await checkDeletableCanonical(path.join(base, 'scan', 'lnk'), ctx())).toBeNull()
  })

  it('8.3 names resolve before the check', async () => {
    expect(await checkDeletableCanonical('C:\\PROGRA~1', { ...win, roots: ['C:\\'] })).toBe('system')
    let short = ''
    try {
      short = execFileSync('cmd.exe', ['/d', '/c', `for %I in ("${path.join(base, 'home', 'longprotectedfolder')}") do @echo %~sI`], { encoding: 'utf8' }).trim()
    } catch {
      short = ''
    }
    if (!short || !short.includes('~')) return
    const c = { ...ctx(), roots: ['C:\\'], keep: [path.join(base, 'home', 'longprotectedfolder')] }
    expect(checkDeletable(short, { ...c, keep: [] })).toBeNull()
    expect(await checkDeletableCanonical(short, c)).toBe('protected')
  })
})
