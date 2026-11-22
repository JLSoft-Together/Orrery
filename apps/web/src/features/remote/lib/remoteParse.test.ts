import { describe, expect, it } from 'vitest'
import { ORRERY_ACTIONS, OPEN_POLICY, REMOTE_TEXT_MAX, isOrrerySource, parseRemoteCommand, parseRemoteKeys, parseRemotePhoneCall } from '@ccm/shared'

const call = (cmd: Record<string, unknown>, extra: Record<string, unknown> = {}) => ({ device: 'Pixel', cmd, ...extra })

describe('phone source in the Orrery table', () => {
  it('is a source with its own open policy: submit only on request, limited, no docking', () => {
    expect(isOrrerySource('phone')).toBe(true)
    expect(OPEN_POLICY.phone).toEqual({ submit: false, limit: true, dock: false, askSubmit: true })
  })

  it('only phones may call the ws.* actions and the summary', () => {
    for (const a of ['remote.summary.get', 'ws.open', 'ws.close', 'ws.rename', 'ws.move', 'ws.reopen', 'ws.compact', 'ws.send', 'ws.keys'] as const) {
      expect(ORRERY_ACTIONS[a].sources).toEqual(['phone'])
      expect(ORRERY_ACTIONS[a].runs).toBe('page')
    }
  })

  it('refuses text over the limit and a bad shape', () => {
    const parse = ORRERY_ACTIONS['ws.send'].parse
    expect(parse(call({ t: 'ws.send', paneKey: 'p1', text: 'x'.repeat(REMOTE_TEXT_MAX) }))).not.toBeNull()
    expect(parse(call({ t: 'ws.send', paneKey: 'p1', text: 'x'.repeat(REMOTE_TEXT_MAX + 1) }))).toBeNull()
    expect(parse(call({ t: 'ws.send', paneKey: 'p1', text: '' }))).toBeNull()
    expect(parse(call({ t: 'ws.send', paneKey: 'p1', text: 'a\0b' }))).toBeNull()
    expect(parse(call({ t: 'ws.close', paneKey: 'p1' }))).toBeNull()
    expect(parse({ cmd: { t: 'ws.send', paneKey: 'p1', text: 'x' } })).toBeNull()
    const open = ORRERY_ACTIONS['ws.open'].parse
    expect(open(call({ t: 'ws.open', cwd: 'C:\\a', preset: 'deepseek' }))).toBeNull()
    expect(open(call({ t: 'ws.open', cwd: 'C:\\a\nb', preset: 'claude' }))).toBeNull()
    expect(open(call({ t: 'ws.open', cwd: 'C:\\a', preset: 'claude', prompt: 'p'.repeat(REMOTE_TEXT_MAX + 1) }))).toBeNull()
    expect(open(call({ t: 'ws.open', cwd: 'C:\\a', preset: 'codex', prompt: 'hi', submit: true, junk: 1 }))).toEqual({
      device: 'Pixel',
      cmd: { t: 'ws.open', cwd: 'C:\\a', preset: 'codex', prompt: 'hi', submit: true },
    })
  })

  it('takes the confirmation flag only at call level, never from inside the phone command, and only as true', () => {
    const parse = ORRERY_ACTIONS['ws.close'].parse
    expect(parse(call({ t: 'ws.close', paneKey: 'p1' }))).toEqual({ device: 'Pixel', cmd: { t: 'ws.close', paneKey: 'p1' } })
    expect(parse(call({ t: 'ws.close', paneKey: 'p1', confirmed: true }))).not.toHaveProperty('confirmed')
    expect(parse(call({ t: 'ws.close', paneKey: 'p1' }, { confirmed: true }))).toMatchObject({ confirmed: true })
    expect(parse(call({ t: 'ws.close', paneKey: 'p1', confirmed: 'yes' }))).not.toHaveProperty('confirmed')
  })

  it('summary request takes no arguments', () => {
    const parse = ORRERY_ACTIONS['remote.summary.get'].parse
    expect(parse(undefined)).toEqual({})
    expect(parse({ device: 'x' })).toEqual({})
    expect(parse('x')).toBeNull()
  })
})

describe('remote commands', () => {
  it('checks keys and rename shapes', () => {
    expect(parseRemoteKeys('approve')).toEqual({ key: 'approve' })
    expect(parseRemoteKeys('3')).toEqual({ key: 'option', option: 3 })
    expect(parseRemoteKeys('10')).toBeNull()
    expect(parseRemoteKeys('rm')).toBeNull()
    expect(parseRemoteCommand({ t: 'ws.keys', paneKey: 'p', keys: 'esc', since: 5 }, 'ws.keys')).toEqual({ t: 'ws.keys', paneKey: 'p', keys: 'esc', since: 5 })
    expect(parseRemoteCommand({ t: 'ws.keys', paneKey: 'p', keys: 'esc' }, 'ws.keys')).toBeNull()
    expect(parseRemoteCommand({ t: 'ws.rename', tabKey: 't', name: null }, 'ws.rename')).toEqual({ t: 'ws.rename', tabKey: 't', name: null })
    expect(parseRemoteCommand({ t: 'ws.rename', tabKey: 't', name: 'n'.repeat(101) }, 'ws.rename')).toBeNull()
    expect(parseRemotePhoneCall({ device: '', cmd: { t: 'ws.reopen' } }, 'ws.reopen')).toBeNull()
  })
})
