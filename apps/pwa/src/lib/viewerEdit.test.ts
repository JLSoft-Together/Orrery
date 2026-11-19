import { describe, expect, it } from 'vitest'
import { answerConfirm, pendingConfirm } from './confirm'
import { editDirty, leaveOk } from './editGuard'
import { commitMessage, commitStaged, discardPath, isDirty, reloadRun, saveFile, stagePaths, startRun, stopRun, unstagePaths } from './viewerActions'
import { ViewerError } from './viewerApi'
import { firstReady, parseChanges, parseFile, parseRunConfigs, parseRunDevices } from './viewerModel'
import { RemoteCommandError, type JsonMap } from '../remote/types'

function recorder(reply: JsonMap | Error) {
  const calls: { type: string; fields?: JsonMap; timeoutMs?: number }[] = []
  const command = async (type: string, fields?: JsonMap, timeoutMs?: number): Promise<JsonMap> => {
    calls.push({ type, fields, timeoutMs })
    if (reply instanceof Error) throw reply
    return reply
  }
  return { calls, command }
}

describe('file editor', () => {
  it('reads the version stamp of a file', () => {
    expect(parseFile({ kind: 'text', path: '/a', size: 3, mtimeMs: 1700, text: 'abc' })).toMatchObject({ mtimeMs: 1700, text: 'abc' })
    expect(parseFile({}).mtimeMs).toBe(0)
  })

  it('saves with the version it read and returns the new stamp', async () => {
    const r = recorder({ data: { ok: true, mtimeMs: 1800 } })
    expect(await saveFile(r.command, '/a', 'new', 1700)).toEqual({ kind: 'saved', mtimeMs: 1800 })
    expect(r.calls[0]).toMatchObject({ type: 'fs.write', fields: { path: '/a', text: 'new', base: 1700 } })
  })

  it('keeps the old stamp when the desktop does not return one', async () => {
    expect(await saveFile(recorder({ data: { ok: true } }).command, '/a', 'x', 5)).toEqual({ kind: 'saved', mtimeMs: 5 })
  })

  it('reports a conflict, a pending confirmation and a denied write', async () => {
    await expect(saveFile(recorder({ data: { ok: false, error: 'conflict' } }).command, '/a', 'x', 1)).rejects.toMatchObject({ code: 'conflict' })
    expect(await saveFile(recorder({ confirm: 'c1' }).command, '/a', 'x', 1)).toEqual({ kind: 'pending' })
    await expect(saveFile(recorder(new RemoteCommandError('denied')).command, '/a', 'x', 1)).rejects.toBeInstanceOf(ViewerError)
  })

  it('knows when a draft differs from the file', () => {
    expect(isDirty('a', 'a')).toBe(false)
    expect(isDirty('a', 'ab')).toBe(true)
  })

  it('asks before leaving only while the draft is dirty', async () => {
    editDirty.value = false
    expect(await leaveOk((k) => k)).toBe(true)
    editDirty.value = true
    const answer = leaveOk((k) => k)
    expect(pendingConfirm.value?.title).toBe('edit.leaveTitle')
    answerConfirm(false)
    expect(await answer).toBe(false)
    const again = leaveOk((k) => k)
    answerConfirm(true)
    expect(await again).toBe(true)
    editDirty.value = false
  })
})

describe('git stage and commit', () => {
  it('splits entries into staged and unstaged like the Android app', () => {
    const { entries } = parseChanges({
      entries: [
        { path: 'a', kind: 'modified', x: '.', y: 'M' },
        { path: 'b', kind: 'modified', x: 'M', y: '.' },
        { path: 'c', kind: 'modified', x: 'M', y: 'M' },
        { path: 'd', kind: 'untracked', x: '?', y: '?' },
        { path: 'e', kind: 'untracked' },
      ],
    })
    expect(entries.map((e) => [e.staged, e.unstaged])).toEqual([[false, true], [true, false], [true, true], [false, true], [false, true]])
  })

  it('sends stage, unstage and discard with the root and paths', async () => {
    const r = recorder({ data: { ok: true } })
    await stagePaths(r.command, '/r', ['a', 'b'])
    await unstagePaths(r.command, '/r', ['a'])
    await discardPath(r.command, '/r', 'a')
    expect(r.calls.map((c) => [c.type, c.fields])).toEqual([
      ['git.stage', { root: '/r', paths: ['a', 'b'] }],
      ['git.unstage', { root: '/r', paths: ['a'] }],
      ['git.discard', { root: '/r', path: 'a' }],
    ])
  })

  it('discard waits for the confirmation on the phone', async () => {
    expect(await discardPath(recorder({ confirm: 'c9' }).command, '/r', 'a')).toEqual({ kind: 'pending' })
    expect(await discardPath(recorder({ cancelled: true }).command, '/r', 'a')).toEqual({ kind: 'cancelled' })
  })

  it('commits a trimmed message and refuses an empty one', async () => {
    expect(commitMessage('  hi  ')).toBe('hi')
    expect(commitMessage('   ')).toBeNull()
    const r = recorder({ data: { ok: true } })
    await commitStaged(r.command, '/r', '  fix bug \n')
    expect(r.calls[0]).toMatchObject({ type: 'git.commit', fields: { root: '/r', message: 'fix bug' } })
    await expect(commitStaged(r.command, '/r', ' ')).rejects.toThrow()
    expect(r.calls).toHaveLength(1)
  })

  it('surfaces a failed commit from the desktop', async () => {
    await expect(commitStaged(recorder({ data: { ok: false, error: 'failed' } }).command, '/r', 'x')).rejects.toMatchObject({ code: 'failed' })
  })
})

describe('run', () => {
  it('parses configs and devices and picks the first ready device', () => {
    expect(parseRunConfigs([{ id: 'c1', name: 'App', type: 'flutter' }, { name: 'no id' }, { id: 'c2' }])).toEqual([
      { id: 'c1', name: 'App', type: 'flutter' },
      { id: 'c2', name: 'c2', type: 'custom' },
    ])
    const devices = parseRunDevices([{ id: 'd1', name: 'Off', state: 'offline' }, { id: 'd2', name: 'Pixel', state: 'ready' }, { id: 'd3' }])
    expect(devices.map((d) => d.ready)).toEqual([false, true, true])
    expect(firstReady(devices)).toBe('d2')
    expect(firstReady([])).toBeNull()
  })

  it('starts with the slow timeout, stops and reloads by session', async () => {
    const r = recorder({ data: { ok: true, sessionId: 'r2' } })
    const started = await startRun(r.command, '/r', 'c1', 'd2')
    expect(started).toEqual({ kind: 'done', data: { ok: true, sessionId: 'r2' } })
    await stopRun(r.command, 'r2')
    await reloadRun(r.command, 'r2', true)
    await reloadRun(r.command, 'r2', false)
    expect(r.calls[0]).toMatchObject({ type: 'run.start', fields: { root: '/r', configId: 'c1', deviceId: 'd2' }, timeoutMs: 60000 })
    expect(r.calls.slice(1).map((c) => [c.type, c.fields])).toEqual([
      ['run.stop', { sessionId: 'r2' }],
      ['run.reload', { sessionId: 'r2', hot: true }],
      ['run.reload', { sessionId: 'r2', hot: false }],
    ])
  })

  it('shows a trust confirmation as pending and maps a refusal to denied', async () => {
    expect(await startRun(recorder({ confirm: 'c1' }).command, '/r', 'c1', null)).toEqual({ kind: 'pending' })
    await expect(startRun(recorder({}).command, '/r', 'c1', null)).rejects.toMatchObject({ code: 'denied' })
  })
})
