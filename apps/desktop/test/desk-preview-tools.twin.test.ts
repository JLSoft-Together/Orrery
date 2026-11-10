import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { registerPreviewIpc } from '../src/preview/ipc'
import type { PreviewServer } from '../src/preview/server'
import type { RootWatcher } from '../src/preview/watch'
import { fakeIpc, HAVE_DESK, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = realpathSync.native(mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-ptools-')))

const FILES: Record<string, string> = {
  '.env': '﻿API_KEY=abc # note\nEMPTY=\nexport MULTI="a\\nb\nc"\nEXTRA=1\nAPI_KEY=override\n',
  '.env.local': "ONLY='x'\n",
  '.env.example': 'API_KEY=\nEMPTY=\nMISSING=\nMULTI=\n',
  '.env.b.sample': 'X=1\n',
  'package.json': JSON.stringify({ scripts: { dev: 'vite', '10': 'n', 'bad name': 'x', build: 'vite build && tsc', num: 3 } }),
  'pnpm-lock.yaml': '',
  makefile: 'all: build\nbuild:\n\tcc\nX := 1\n.PHONY: all\n',
  justfile: 'set shell := ["bash"]\nfmt:\n  x\nrun arg:\n  y\n',
  'Cargo.toml': '[package]\nname = "x"\n',
  'pyproject.toml': '[project.scripts]\ncli = "pkg:main"\n"quoted" = "a:b"\n',
  'poetry.lock': '',
}

function makeSide(name: string) {
  const root = path.join(tmp, name)
  const proj = path.join(root, 'proj')
  const bare = path.join(root, 'bare')
  mkdirSync(proj, { recursive: true })
  mkdirSync(bare, { recursive: true })
  for (const [f, text] of Object.entries(FILES)) writeFileSync(path.join(proj, f), text)
  writeFileSync(path.join(bare, '.env'), 'A=1\nB=\n')
  return { root, proj, bare }
}

function side(roots: string[], twin?: Twin) {
  const ipc = fakeIpc()
  registerPreviewIpc({
    ipcMain: ipc.ipcMain,
    isAppFrame: () => true,
    server: { tokensUnder: () => [] } as unknown as PreviewServer,
    watcher: { listen: () => undefined } as unknown as RootWatcher,
    shell: {} as never,
    dialog: {} as never,
    getWindow: () => null,
    getRoots: () => roots,
    confirmAccess: async () => false,
    desk: twin?.router,
  })
  return { ...ipc, ev: { sender: { id: 5 } } }
}

describe.skipIf(!HAVE_DESK)('desk previewTools twin (TS vs orrery-core desk)', () => {
  let twin: Twin
  let n: ReturnType<typeof makeSide>
  let r: ReturnType<typeof makeSide>
  let node: ReturnType<typeof side>
  let rust: ReturnType<typeof side>

  beforeAll(async () => {
    n = makeSide('n')
    r = makeSide('r')
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'data') })
    node = side([n.proj, n.bare, path.join(n.root, 'i18n')])
    rust = side([r.proj, r.bare, path.join(r.root, 'i18n')], twin)
  }, 60_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const same = async (channel: string, mk: (s: ReturnType<typeof makeSide>) => unknown[]): Promise<unknown> => {
    const a = await node.call(channel, node.ev, ...mk(n))
    const b = await rust.call(channel, rust.ev, ...mk(r))
    expect(b, channel).toEqual(swapRoot(a, n.root, r.root))
    return b
  }

  it('checks and reveals .env files the same', async () => {
    expect(twin.host.channels).toContain('ccm:env-check')
    const res = (await same('ccm:env-check', (s) => [s.proj])) as { files: unknown[] }
    expect(res.files.length).toBe(2)
    await same('ccm:env-check', (s) => [s.bare])
    await same('ccm:env-check', (s) => [path.join(s.root, 'nope')])
    for (const key of ['API_KEY', 'MULTI', 'EMPTY', 'NOPE', '']) await same('ccm:env-reveal', (s) => [{ root: s.proj, file: '.env', key }])
    await same('ccm:env-reveal', (s) => [{ root: s.proj, file: '../.env', key: 'A' }])
    await same('ccm:env-reveal', (s) => [{ root: s.proj, file: 'package.json', key: 'A' }])
  })

  it('scans translations the same', async () => {
    const I18N: Record<string, string> = {
      'app/src/main/res/values/strings.xml': '<resources>\n<!-- <string name="gone">x</string> -->\n<string name="hello">Hello world</string>\n<string name="id" translatable="false">ID</string>\n<string name="only_base">Base</string>\n<plurals name="files"><item quantity="one">%d file</item></plurals>\n<string-array name="arr"><item>a</item></string-array>\n</resources>\n',
      'app/src/main/res/values-vi/strings.xml': '<resources>\n<string name="hello">Hello world</string>\n<string name="extra_vi">Thêm</string>\n</resources>\n',
      'app/src/main/res/values-pt-rBR/strings.xml': '<resources><string name="hello">Olá</string></resources>',
      'app/src/main/res/values-car/strings.xml': '<resources><string name="hello">car</string></resources>',
      'app/src/main/res/values-night/strings.xml': '<resources/>',
      'mobile/pubspec.yaml': 'name: m\n',
      'mobile/l10n.yaml': "arb-dir: lib/l10n # c\ntemplate-arb-file: 'app_en.arb'\n",
      'mobile/lib/l10n/app_en.arb': '{\n  "@@locale": "en",\n  "title": "My app {name}",\n  "@title": {"placeholders": {}},\n  "count": "{n} items"\n}\n',
      'mobile/lib/l10n/app_vi.arb': '{\n  "title": "My app {name}",\n  "other": "x"\n}\n',
      'mobile/lib/l10n/app_fr.arb': '{ broken',
      'web/src/locales/en.json': '{\n  "nav": {\n    "home": "Home page",\n    "about": "About"\n  },\n  "n": 3,\n  "list": [1, 2]\n}\n',
      'web/src/locales/de.json': '{\n  "nav": { "home": "Home page" },\n  "extra": "x"\n}\n',
      'web/src/locales/es.json': '[1]',
      'node_modules/x/locales/en.json': '{}',
    }
    for (const s of [n, r]) {
      for (const [f, text] of Object.entries(I18N)) {
        mkdirSync(path.dirname(path.join(s.root, 'i18n', f)), { recursive: true })
        writeFileSync(path.join(s.root, 'i18n', f), text)
      }
    }
    const scan = (await same('ccm:i18n-scan', (s) => [path.join(s.root, 'i18n')])) as { sets: unknown[] }
    expect(scan.sets.length).toBe(3)
    await same('ccm:i18n-scan', (s) => [path.join(s.root, 'nope')])
  })

  it('detects scripts the same', async () => {
    const res = (await same('ccm:scripts', (s) => [s.proj])) as { groups: unknown[] }
    expect(res.groups.length).toBe(5)
    await same('ccm:scripts', (s) => [s.bare])
    await same('ccm:scripts', (s) => [s.root])
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})
