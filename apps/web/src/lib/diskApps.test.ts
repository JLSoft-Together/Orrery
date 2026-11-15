import { describe, expect, it } from 'vitest'
import { createI18n } from 'vue-i18n'
import type { InstalledApp, RamProc } from '@ccm/shared'
import messages from '../i18n/disk-apps-v101'
import { filterApps, formatPct, groupRam, ramShare } from './diskApps'

const app = (name: string, size: number | null, installedAt: number | null = null, publisher: string | null = null): InstalledApp => ({
  id: name,
  name,
  publisher,
  version: null,
  installedAt,
  size,
  sizeFrom: null,
  location: null,
  source: 'registry',
  scope: 'machine',
  arch: null,
  bundleId: null,
  lastUsed: null,
  uninstall: null,
  blocked: null,
})

const proc = (pid: number, name: string, ram: number, extra: Partial<RamProc> = {}): RamProc => ({ pid, ppid: 1, start: `s${pid}`, name, ramBytes: ram, cpu: 1, tag: null, lock: null, ...extra })

describe('diskApps lib', () => {
  it('filters by name or publisher and sorts', () => {
    const list = [app('Zed', 10, 3), app('alpha', 30, 1, 'Acme'), app('Beta', null, 2)]
    expect(filterApps(list, '', 'size').map((a) => a.name)).toEqual(['alpha', 'Zed', 'Beta'])
    expect(filterApps(list, '', 'name').map((a) => a.name)).toEqual(['alpha', 'Beta', 'Zed'])
    expect(filterApps(list, '', 'date').map((a) => a.name)).toEqual(['Zed', 'Beta', 'alpha'])
    expect(filterApps(list, 'acme', 'size').map((a) => a.name)).toEqual(['alpha'])
  })

  it('groups processes by app, keeps locked ones out of kill targets', () => {
    const groups = groupRam([
      proc(1, 'chrome.exe', 100),
      proc(2, 'chrome.exe', 300),
      proc(3, 'node.exe', 50, { tag: 'dev' }),
      proc(4, 'node.exe', 70, { tag: 'self', lock: 'self' }),
      proc(5, 'electron.exe', 500, { tag: 'self', lock: 'self' }),
    ])
    expect(groups.map((g) => g.name)).toEqual(['electron', 'chrome', 'node'])
    const chrome = groups.find((g) => g.name === 'chrome')!
    expect(chrome.ramBytes).toBe(400)
    expect(chrome.count).toBe(2)
    expect(chrome.targets.map((t) => t.pid)).toEqual([2, 1])
    const node = groups.find((g) => g.name === 'node')!
    expect(node.tag).toBe('self')
    expect(node.targets).toEqual([])
    expect(node.lock).toBe('self')
    expect(groups.find((g) => g.name === 'electron')!.targets).toEqual([])
  })

  it('formats shares', () => {
    expect(ramShare(50, 200)).toBe(25)
    expect(ramShare(null, 200)).toBeNull()
    expect(formatPct(3.14)).toBe('3.1%')
    expect(formatPct(42.6)).toBe('43%')
  })
})

type Tree = { [key: string]: Tree | string }
const leaves = (tree: Tree, prefix = ''): [string, string][] => Object.entries(tree).flatMap(([k, v]) => (typeof v === 'string' ? [[`${prefix}${k}`, v] as [string, string]] : leaves(v, `${prefix}${k}.`)))

describe('disk-apps-v101 messages', () => {
  for (const locale of ['en', 'vi'] as const) {
    it(`${locale} compiles and matches the other locale`, () => {
      const tree = messages[locale] as unknown as Tree
      const i18n = createI18n({ legacy: false, locale, messages: { [locale]: tree }, missingWarn: false, fallbackWarn: false })
      const bad: string[] = []
      for (const [key] of leaves(tree)) {
        try {
          i18n.global.t(key, {}, { plural: 1 })
        } catch (e) {
          bad.push(`${key}: ${String(e)}`)
        }
      }
      expect(bad).toEqual([])
      expect(leaves(tree).map(([k]) => k)).toEqual(leaves(messages.en as unknown as Tree).map(([k]) => k))
    })
  }

  it('vi text has no banned words or separators', () => {
    const bad = leaves(messages.vi as unknown as Tree).filter(([, v]) => /trần|—|·/i.test(v))
    expect(bad).toEqual([])
  })
})
