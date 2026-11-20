import { describe, expect, it } from 'vitest'
import type { MonitorSession } from './monitorModel'
import { cardState, groupByProject, letterOf, machineSubtitle, osLabel, projectName, projectRows, quotaTone, railCounts, toneOf } from './projectModel'

function session(id: string, cwd: string | null, rail: MonitorSession['rail'] = 'idle', branch: string | null = null): MonitorSession {
  return { id, title: id, rail, provider: 'claude', branch, cwd, tool: null, toolSummary: null, waitingFor: null, since: null, subagents: [] }
}

describe('projectName', () => {
  it('takes the last folder of a posix or windows path', () => {
    expect(projectName('/home/me/api')).toBe('api')
    expect(projectName('D:\\work\\web\\orrery\\')).toBe('orrery')
  })

  it('is null without a path', () => {
    expect(projectName(null)).toBeNull()
    expect(projectName('')).toBeNull()
    expect(projectName('/')).toBeNull()
  })
})

describe('groupByProject', () => {
  it('keeps the order of first appearance and the session order inside a project', () => {
    const groups = groupByProject([session('a', '/x/api'), session('b', '/x/web'), session('c', '/y/api')])
    expect(groups.map((g) => g.name)).toEqual(['api', 'web'])
    expect(groups[0]!.sessions.map((s) => s.id)).toEqual(['a', 'c'])
  })

  it('takes the first branch it finds and a stable tone', () => {
    const [g] = groupByProject([session('a', '/x/api'), session('b', '/x/api', 'idle', 'feat/api')])
    expect(g!.branch).toBe('feat/api')
    expect(g!.letter).toBe('A')
    expect(g!.tone).toBe(toneOf('api'))
    expect(g!.tone).toBeGreaterThanOrEqual(0)
    expect(g!.tone).toBeLessThan(8)
  })

  it('puts sessions without a path in one group with no name', () => {
    const groups = groupByProject([session('a', null), session('b', null)])
    expect(groups).toHaveLength(1)
    expect(groups[0]!.name).toBeNull()
    expect(groups[0]!.letter).toBe('?')
  })
})

describe('railCounts', () => {
  it('counts every state, including failed and done', () => {
    const counts = railCounts([session('a', null, 'working'), session('b', null, 'working'), session('c', null, 'asked'), session('d', null, 'failed'), session('e', null, 'done'), session('f', null, 'idle')])
    expect(counts).toEqual({ working: 2, asked: 1, failed: 1, done: 1, idle: 1 })
  })
})

describe('cardState', () => {
  it('is live when ready, lost when offline or revoked, connecting only while there is no data yet', () => {
    expect(cardState('ready', true)).toBe('live')
    expect(cardState('offline', true)).toBe('lost')
    expect(cardState('revoked', false)).toBe('lost')
    expect(cardState('handshaking', false)).toBe('connecting')
    expect(cardState('reconnecting', true)).toBe('lost')
  })
})

describe('machineSubtitle', () => {
  it('shows version and OS only when the machine sent them', () => {
    expect(machineSubtitle({ version: '4.0.7', os: 'win32' })).toBe('Orrery 4.0.7, Windows')
    expect(machineSubtitle({ version: '4.0.7', platform: 'darwin' })).toBe('Orrery 4.0.7, macOS')
    expect(machineSubtitle({ version: '4.0.7' })).toBe('Orrery 4.0.7')
    expect(machineSubtitle({ version: '4.0.7', os: 'plan9' })).toBe('Orrery 4.0.7')
  })

  it('is null without a version', () => {
    expect(machineSubtitle(null)).toBeNull()
    expect(machineSubtitle({ os: 'win32' })).toBeNull()
    expect(machineSubtitle({ version: '' })).toBeNull()
  })

  it('maps OS names', () => {
    expect(osLabel({ os: 'Linux' })).toBe('Linux')
    expect(osLabel({ os: 5 })).toBeNull()
  })
})

describe('quotaTone', () => {
  it('warns from 65 and goes red from 85', () => {
    expect(quotaTone(42)).toBe('ok')
    expect(quotaTone(65)).toBe('warn')
    expect(quotaTone(84)).toBe('warn')
    expect(quotaTone(91)).toBe('bad')
  })
})

describe('letterOf', () => {
  it('uppercases the first letter', () => {
    expect(letterOf('web')).toBe('W')
    expect(letterOf('  đồ án')).toBe('Đ')
  })
})

describe('projectRows', () => {
  it('collapses a project with one session and keeps groups for more', () => {
    const rows = projectRows([session('a', '/x/api'), session('b', '/x/web'), session('c', '/y/web')])
    expect(rows.map((r) => [r.project.name, r.kind])).toEqual([
      ['api', 'single'],
      ['web', 'group'],
    ])
    const single = rows[0]!
    expect(single.kind === 'single' && single.session.id).toBe('a')
  })

  it('floats projects with waiting or failed sessions above running and idle ones', () => {
    const rows = projectRows([
      session('i', '/x/idle', 'idle'),
      session('w', '/x/work', 'working'),
      session('q', '/x/ask', 'asked'),
      session('f', '/x/fail', 'failed'),
      session('d', '/x/done', 'done'),
    ])
    expect(rows.map((r) => r.project.name)).toEqual(['ask', 'fail', 'work', 'idle', 'done'])
  })

  it('ranks a group by its most urgent session', () => {
    const rows = projectRows([session('a', '/x/one', 'working'), session('b', '/x/two', 'idle'), session('c', '/x/two', 'asked')])
    expect(rows.map((r) => r.project.name)).toEqual(['two', 'one'])
  })
})
