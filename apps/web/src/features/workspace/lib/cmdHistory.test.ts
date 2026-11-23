import { afterEach, describe, expect, it, vi } from 'vitest'
import { addCmdHistory, clockText, filterHistory, isMultiline, loadCmdHistory, moveSelection, pickMode, removeCmdHistory, type CmdEntry } from './cmdHistory'

const e = (cmd: string, code = 0, at = 1): CmdEntry => ({ cmd, code, ms: 10, at })

afterEach(() => {
  delete (window as unknown as { ccmDesktop?: unknown }).ccmDesktop
})

describe('filterHistory', () => {
  const items = [e('npm run build'), e('git status'), e('npm test -- cmdHistory'), e('cargo test')]

  it('keeps the order and returns everything for an empty query', () => {
    expect(filterHistory(items, '  ')).toEqual(items)
  })

  it('needs every word, case-insensitive, in any order', () => {
    expect(filterHistory(items, 'TEST npm').map((x) => x.cmd)).toEqual(['npm test -- cmdHistory'])
    expect(filterHistory(items, 'test').map((x) => x.cmd)).toEqual(['npm test -- cmdHistory', 'cargo test'])
    expect(filterHistory(items, 'zzz')).toEqual([])
  })
})

describe('selection and run mode', () => {
  it('wraps around both ways and survives an empty list', () => {
    expect(moveSelection(0, -1, 3)).toBe(2)
    expect(moveSelection(2, 1, 3)).toBe(0)
    expect(moveSelection(1, 1, 3)).toBe(2)
    expect(moveSelection(0, 1, 0)).toBe(0)
  })

  it('Enter fills, Shift+Enter runs only a single line at an idle prompt', () => {
    expect(pickMode(false, e('ls'), true)).toBe('fill')
    expect(pickMode(true, e('ls'), true)).toBe('run')
    expect(pickMode(true, e('ls'), false)).toBe('fill')
    expect(pickMode(true, e('a\nb'), true)).toBe('fill')
    expect(isMultiline('a\r\nb')).toBe(true)
  })
})

describe('clockText', () => {
  it('shows the time today and the date before', () => {
    const now = new Date(2026, 10, 23, 15, 0).getTime()
    expect(clockText(new Date(2026, 10, 23, 9, 5).getTime(), now)).toBe('09:05')
    expect(clockText(new Date(2026, 10, 12, 14, 30).getTime(), now)).toBe('12/11 14:30')
  })
})

describe('bridge', () => {
  it('lists, adds and removes through the desktop bridge and ignores junk', async () => {
    const list = vi.fn().mockResolvedValue([e('a'), { cmd: 5 }, null, e('b')])
    const add = vi.fn().mockResolvedValue(true)
    const remove = vi.fn().mockResolvedValue(true)
    ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = { cmdHistList: list, cmdHistAdd: add, cmdHistRemove: remove }
    expect((await loadCmdHistory('/p')).map((x) => x.cmd)).toEqual(['a', 'b'])
    await addCmdHistory('/p', e('c'))
    await removeCmdHistory('/p', { at: 1, cmd: 'c' })
    expect(add).toHaveBeenCalledWith('/p', e('c'))
    expect(remove).toHaveBeenCalledWith('/p', { at: 1, cmd: 'c' })
  })

  it('returns an empty list when the bridge is missing or throws', async () => {
    expect(await loadCmdHistory('/p')).toEqual([])
    ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = { cmdHistList: () => Promise.reject(new Error('x')) }
    expect(await loadCmdHistory('/p')).toEqual([])
  })
})
