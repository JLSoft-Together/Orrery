import { describe, expect, it } from 'vitest'
import { INBOX_GROUPS as SHARED_GROUPS, type TerminalSession } from '@ccm/shared'
import { INBOX_GROUPS, inboxGroups, inboxItems, inboxTone, nextInboxItem, pruneDismissed, type InboxInput } from './inbox'

const term = (id: string, over: Partial<TerminalSession> = {}): TerminalSession => ({ id, title: id, status: 'idle', ...over }) as TerminalSession

const base = (over: Partial<InboxInput> = {}): InboxInput => ({
  terminals: [],
  loops: new Map(),
  stuck: new Set(),
  failedRoots: [],
  doneUnseen: new Set(),
  dismissed: {},
  ...over,
})

describe('inbox', () => {
  it('shares one group order with the phones', () => {
    expect(INBOX_GROUPS).toBe(SHARED_GROUPS)
    expect([...INBOX_GROUPS]).toEqual(['menu', 'permission', 'question', 'clash', 'error', 'note', 'done'])
  })

  it('keeps the most urgent reason per agent and sorts by group, then age', () => {
    const items = inboxItems(
      base({
        terminals: [
          term('done', { statusSince: '2026-10-29T01:00:00Z' }),
          term('wait2', { status: 'waiting', statusSince: '2026-10-29T01:05:00Z' }),
          term('wait1', { status: 'waiting', statusSince: '2026-10-29T01:00:00Z' }),
          term('perm', { status: 'waiting', waitingFor: 'permission prompt', statusSince: '2026-10-29T01:09:00Z' }),
          term('check', { cwd: 'F:\\r\\app', status: 'working' }),
          term('loop', { status: 'working' }),
          term('gone', { status: 'stale' }),
        ],
        loops: new Map([['loop', { since: '2026-10-29T01:00:00Z' }], ['wait2', { since: '2026-10-29T00:00:00Z' }]]),
        failedRoots: ['F:/r'],
        doneUnseen: new Set(['done', 'gone']),
      }),
    )
    expect(items.map((i) => `${i.id}:${i.reason}`)).toEqual(['perm:permission', 'wait1:question', 'wait2:question', 'loop:loop', 'check:checks', 'done:done'])
  })

  it('puts a menu above the permission of the same agent and lists a pane menu without a session', () => {
    const items = inboxItems(
      base({
        terminals: [term('a', { status: 'waiting', waitingFor: 'permission prompt', statusSince: '2026-10-29T01:00:00Z' })],
        menus: [
          { paneKey: 'pa', terminalId: 'a', question: 'Allow?', since: Date.parse('2026-10-29T01:01:00Z') },
          { paneKey: 'pb', question: 'Pick one', since: Date.parse('2026-10-29T01:00:00Z') },
        ],
        paneOf: (id) => (id === 'a' ? 'pa' : undefined),
      }),
    )
    expect(items.map((i) => [i.id, i.reason, i.paneKey, i.question])).toEqual([
      ['pane:pb', 'menu', 'pb', 'Pick one'],
      ['a', 'menu', 'pa', 'Allow?'],
    ])
  })

  it('lists a clash once per file with the latest editor first and the other as peer', () => {
    const items = inboxItems(
      base({
        terminals: [term('a'), term('b')],
        clashes: [{ fileKey: 'k', file: 'src/a.ts', peers: [{ terminalId: 'a', name: 'A', at: 1000 }, { terminalId: 'b', name: 'B', at: 2000 }] }],
      }),
    )
    expect(items).toHaveLength(1)
    expect(items[0]).toMatchObject({ id: 'clash:k', reason: 'clash', terminalId: 'b', file: 'src/a.ts', peer: 'A' })
    expect(inboxTone(items)).toBe('warning')
  })

  it('counts failed runs and asked panes from the workspace', () => {
    const items = inboxItems(base({ panes: [{ key: 'run', failed: true, asked: false }, { key: 'sh', failed: false, asked: true }] }))
    expect(items.map((i) => `${i.id}:${i.reason}`)).toEqual(['pane:sh:question', 'pane:run:failed'])
    expect(inboxGroups(items).map((g) => g.group)).toEqual(['question', 'error'])
    expect(inboxTone(items)).toBe('waiting')
  })

  it('leaves out snoozed panes and steps to the next item after the current pane', () => {
    const input = base({ panes: [{ key: 'p1', failed: false, asked: true }, { key: 'p2', failed: true, asked: false }] })
    expect(inboxItems({ ...input, snoozed: (k) => k === 'p1' }).map((i) => i.paneKey)).toEqual(['p2'])
    const items = inboxItems(input)
    expect(nextInboxItem(items, null)?.paneKey).toBe('p1')
    expect(nextInboxItem(items, 'p1')?.paneKey).toBe('p2')
    expect(nextInboxItem(items, 'p2')?.paneKey).toBe('p1')
    expect(nextInboxItem([], 'p1')).toBeNull()
  })

  it('hides a dismissed row until its state changes', () => {
    const waiting = term('a', { status: 'waiting', statusSince: '2026-10-29T01:00:00Z' })
    const [item] = inboxItems(base({ terminals: [waiting] }))
    expect(inboxItems(base({ terminals: [waiting], dismissed: { a: item!.stamp } }))).toEqual([])
    const again = { ...waiting, statusSince: '2026-10-29T01:09:00Z' }
    expect(inboxItems(base({ terminals: [again], dismissed: { a: item!.stamp } }))).toHaveLength(1)
    expect(pruneDismissed({ a: 'x', b: 'y' }, new Set(['a']))).toEqual({ a: 'x' })
  })
})
