import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { clampZoom, nextZoom, parseLook, readAppearance, writeAppearance, zoomPercent } from '../src/appearance'
import { cleanDirty } from '../src/editors'
import { LayoutStore, LAYOUTS_FILE, registerLayoutsIpc } from '../src/layouts'
import { errorPage, loadingPage } from '../src/pages'
import { registerRestoreIpc } from '../src/restore/ipc'
import { stringsFor } from '../src/strings'
import { fakeIpc, HAVE_DESK, startTwin, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-layouts-'))
const project = path.join(tmp, 'Proj')
const other = path.join(tmp, 'other')
const ext = path.join(tmp, 'prompts.json')
let twin: Twin
const node = fakeIpc()
const rust = fakeIpc()

const ID = /[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/g
const norm = (v: unknown): unknown => {
  const ids = new Map<string, string>()
  const text = JSON.stringify(v ?? null)
    .replace(/"updatedAt":"[^"]+"/g, '"updatedAt":"T"')
    .replace(ID, (m) => {
      if (!ids.has(m)) ids.set(m, `id${ids.size}`)
      return ids.get(m)!
    })
  return JSON.parse(text.split(JSON.stringify(path.join(tmp, 'node')).slice(1, -1)).join('D').split(JSON.stringify(path.join(tmp, 'rust')).slice(1, -1)).join('D'))
}

const term = (cwd: string) => ({ kind: 'term', preset: 'claude', cwd, resume: '0123abcd-0000-4000-8000-abcdef012345' })
const LAYOUTS = [
  { project, name: '  Main  ', panes: { a: term(project), b: { kind: 'browser', url: 'HTTPS://x.dev' }, c: { kind: 'file', path: path.join(project, 'f.ts'), line: 3.7 }, 9: { kind: 'log', cwd: project } }, tabs: [{ root: { dir: 'row', ratio: 2, a: { pane: 'a' }, b: { dir: 'col', a: { pane: 'b' }, b: { pane: 'zz' } } }, focus: 'b' }, { root: { pane: '9' } }] },
  { project: project.toUpperCase(), name: 'Second', openOnStart: true, panes: { x: { kind: 'term', preset: 'shell', resume: '0123abcd-0000-4000-8000-abcdef012345' }, y: { kind: 'browser', url: null } }, tabs: [{ root: { pane: 'x' } }, { root: { pane: 'x' } }] },
  { project: 'rel', name: 'x', panes: {}, tabs: [] },
  { project, name: '   ', panes: { a: term(project) }, tabs: [{ root: { pane: 'a' } }] },
  { project, name: 'Empty', panes: { a: { kind: 'changes' } }, tabs: [{ root: { pane: 'a' } }] },
  null,
]

describe.skipIf(!HAVE_DESK)('layouts + restore: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    for (const d of [project, other, path.join(tmp, 'node'), path.join(tmp, 'rust')]) mkdirSync(d, { recursive: true })
    writeFileSync(path.join(tmp, 'node', 'run-state.json'), JSON.stringify({ running: true }))
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'rust'), CCM_USER_DATA: path.join(tmp, 'rust') })
    const gate = (e: unknown): boolean => e === 'app'
    const dialogs = { saveDialog: async () => ext, openDialog: async () => ext }
    registerLayoutsIpc({ ipcMain: node.ipcMain, isAppFrame: gate, store: new LayoutStore(path.join(tmp, 'node', LAYOUTS_FILE)) })
    registerLayoutsIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, store: () => new LayoutStore(), desk: twin.router })
    registerRestoreIpc({ ipcMain: node.ipcMain, isAppFrame: gate, previous: { unclean: true }, ...dialogs })
    registerRestoreIpc({ ipcMain: rust.ipcMain, isAppFrame: gate, previous: { unclean: true }, ...dialogs, desk: twin.router })
  }, 30_000)

  afterAll(async () => {
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  const both = async (channel: string, ...args: unknown[]) => {
    const a = norm(await node.call(channel, 'app', ...args))
    const b = norm(await rust.call(channel, 'app', ...args))
    expect(b).toEqual(a)
    return a
  }

  it('saves, lists, flags and deletes layouts the same way, same file', async () => {
    const saved: string[] = []
    for (const l of LAYOUTS) {
      const r = (await node.call('ccm:layouts-save', 'app', l)) as { ok: boolean; layout?: { id: string } }
      const s = (await rust.call('ccm:layouts-save', 'app', l)) as { ok: boolean; layout?: { id: string } }
      expect(norm(s)).toEqual(norm(r))
      if (r.layout && s.layout) saved.push(r.layout.id, s.layout.id)
    }
    await both('ccm:layouts-list', project)
    await both('ccm:layouts-list', 'rel')
    for (const [i, flag] of [[0, true], [2, null]] as const) {
      expect(await rust.call('ccm:layouts-start', 'app', project, flag === null ? null : saved[i + 1])).toEqual(await node.call('ccm:layouts-start', 'app', project, flag === null ? null : saved[i]))
    }
    expect(await rust.call('ccm:layouts-start', 'app', project, 'missing')).toEqual(await node.call('ccm:layouts-start', 'app', project, 'missing'))
    const renamed = { ...(LAYOUTS[0] as object), name: 'Renamed' }
    expect(norm(await rust.call('ccm:layouts-save', 'app', { ...renamed, id: saved[1] }))).toEqual(norm(await node.call('ccm:layouts-save', 'app', { ...renamed, id: saved[0] })))
    expect(await rust.call('ccm:layouts-delete', 'app', project, saved[3])).toEqual(await node.call('ccm:layouts-delete', 'app', project, saved[2]))
    expect(await rust.call('ccm:layouts-delete', 'app', project, saved[3])).toEqual(await node.call('ccm:layouts-delete', 'app', project, saved[2]))
    await both('ccm:layouts-list', project)
    const file = (side: string) => norm(JSON.parse(readFileSync(path.join(tmp, side, LAYOUTS_FILE), 'utf8')))
    expect(file('rust')).toEqual(file('node'))
    for (let i = 0; i < 21; i++) {
      const l = { project: other, name: `n${i}`, panes: { a: term(other) }, tabs: [{ root: { pane: 'a' } }] }
      expect(norm(await rust.call('ccm:layouts-save', 'app', l))).toEqual(norm(await node.call('ccm:layouts-save', 'app', l)))
    }
  })

  it('run state, folders and prompt files match', async () => {
    await both('ccm:run-state')
    await both('ccm:run-state')
    await both('ccm:dirs-exist', [project, path.join(project, 'nope'), 'rel', '\\\\srv\\x', 5, ext])
    await both('ccm:dirs-exist', 'x')
    for (const raw of [JSON.stringify({ prompts: [{ text: 'hi', token: 'x' }], env: { A: 1 } }), '{"x":1}', 'bad', 5]) {
      rmSync(ext, { force: true })
      const a = norm(await node.call('ccm:prompts-export', 'app', raw))
      const fa = existsSync(ext) ? readFileSync(ext, 'utf8') : null
      rmSync(ext, { force: true })
      expect(norm(await rust.call('ccm:prompts-export', 'app', raw))).toEqual(a)
      expect(existsSync(ext) ? readFileSync(ext, 'utf8') : null).toEqual(fa)
    }
    for (const text of ['﻿{"prompts":[]}', 'nope', null]) {
      rmSync(ext, { force: true })
      if (text !== null) writeFileSync(ext, text)
      await both('ccm:prompts-import')
    }
    expect(twin.logs).toEqual([])
    expect(twin.fellBack).toEqual([])
  })
})

describe.skipIf(!HAVE_DESK)('appearance, pages and editors helpers in orrery-core desk match TS', () => {
  beforeAll(async () => {
    mkdirSync(path.join(tmp, 'app'), { recursive: true })
    twin = await startTwin({ CCM_DATA_DIR: path.join(tmp, 'app'), CCM_USER_DATA: path.join(tmp, 'app') })
  }, 30_000)
  afterAll(async () => {
    await twin?.host.stop()
  })
  const call = (channel: string, ...args: unknown[]) => twin.host.call(channel, args)
  const look = { canvas: '#AABBCC', surface: '#ffffff', ink: '#000000', muted: '#111111', faint: '#222222', accent: '#ffcc00', bar: '#ABCDEF', dark: true }

  it('looks, zoom and appearance.json', async () => {
    for (const raw of [look, { ...look, bar: 5 }, { ...look, dark: 'x' }, { ...look, canvas: 'red' }, null, []]) expect(await call('desk:appearance-look', raw)).toEqual(parseLook(raw))
    for (const level of [0, 1.24, 1.25, -9, 'x', 2.75]) for (const action of ['in', 'out', 'reset', 'x']) {
      const l = clampZoom(level)
      const next = action === 'in' || action === 'out' || action === 'reset' ? nextZoom(l, action) : l
      expect(await call('desk:appearance-zoom', level, action)).toEqual({ level: next, percent: zoomPercent(next) })
    }
    const file = path.join(tmp, 'app', 'appearance.json')
    writeFileSync(file, JSON.stringify({ zoom: 1.3, look, locale: 'fr', background: '#123456', extra: 1 }))
    expect(await call('desk:appearance-read')).toEqual(readAppearance(file))
    await call('desk:appearance-save', { locale: 'vi' })
    const rust = readFileSync(file, 'utf8')
    writeAppearance(file, { ...readAppearance(file), locale: 'vi' })
    expect(rust).toBe(readFileSync(file, 'utf8'))
  })

  it('loading and error pages are byte-identical', async () => {
    for (const lang of ['en', 'vi'] as const) {
      const t = stringsFor(lang)
      const texts = { appName: t.appName, loading: t.loading, loadingHint: t.loadingHint, errorTitle: t.errorTitle, retry: t.retry, logHint: t.logHint }
      for (const l of [null, parseLook(look)]) {
        expect(await call('desk:pages-loading', texts, l, '2.3.0')).toBe(loadingPage(t, l, '2.3.0'))
        expect(await call('desk:pages-loading', texts, l, null)).toBe(loadingPage(t, l))
        expect(await call('desk:pages-error', texts, 'Port <4317> & "busy"', ['a', 'b\'c'], l)).toBe(errorPage(t, 'Port <4317> & "busy"', ['a', "b'c"], l))
        expect(await call('desk:pages-error', texts, 'x', [], l)).toBe(errorPage(t, 'x', [], l))
      }
    }
  })

  it('dirty editor lists', async () => {
    const raw = [{ name: 'a'.repeat(300), path: '/p/a' }, { name: 1, path: '/x' }, null, { name: 'b', path: 'p'.repeat(2000) }]
    await call('desk:editor-dirty', 2, raw, true)
    await call('desk:editor-dirty', 1, [{ name: 'c', path: '/c' }], false)
    await call('desk:editor-dirty', 2, raw, true)
    const state = (await call('desk:editor-state')) as { files: unknown; autoSave: boolean }
    expect(state.files).toEqual([...cleanDirty(raw), ...cleanDirty([{ name: 'c', path: '/c' }])])
    expect(state.autoSave).toBe(false)
    await call('desk:editor-dirty', 1, [], false)
    expect(((await call('desk:editor-state')) as { autoSave: boolean }).autoSave).toBe(true)
  })
})
