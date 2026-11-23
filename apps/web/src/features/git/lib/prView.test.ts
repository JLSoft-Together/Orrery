import { describe, expect, it } from 'vitest'
import type { GhPr } from '@ccm/shared'
import { CHIP_HOLD_MS, canMerge, chipState, ciLook, elapsedMs, elapsedParts, errorKey, panelState, permissionLabel, readMethod, runLook, toastParts, writeMethod } from './prView'

const pr = (over: Partial<GhPr> = {}): GhPr => ({ number: 7, title: 't', url: 'u', state: 'open', draft: false, head: 'feat', headSha: 'a'.repeat(40), base: 'main', mergeable: true, mergeableState: 'clean', ...over })

describe('ciLook', () => {
  it('maps each state to icon, tone, text and color token', () => {
    expect(ciLook('pending')).toMatchObject({ icon: 'spin', tone: 'wait', text: 'v111pr.ciState.pending', cls: 'text-warning' })
    expect(ciLook('success')).toMatchObject({ icon: 'check', tone: 'ok', cls: 'text-success' })
    expect(ciLook('failure')).toMatchObject({ icon: 'x', tone: 'bad', cls: 'text-st-error' })
    expect(ciLook('neutral').icon).toBe('minus')
    expect(ciLook('none').tone).toBe('idle')
  })
  it('never colours a state without an icon and text', () => {
    for (const s of ['pending', 'success', 'failure', 'neutral', 'none'] as const) {
      const l = ciLook(s)
      expect(l.icon && l.text && l.cls).toBeTruthy()
    }
  })
  it('gives a run its own wording', () => {
    expect(runLook('failure').runText).toBe('v111pr.runState.failure')
  })
})

describe('elapsed', () => {
  it('measures a finished and a running run', () => {
    expect(elapsedMs({ startedAt: '2026-11-23T10:00:00Z', completedAt: '2026-11-23T10:01:20Z' }, 0)).toBe(80_000)
    expect(elapsedMs({ startedAt: '2026-11-23T10:00:00Z', completedAt: null }, Date.parse('2026-11-23T10:00:30Z'))).toBe(30_000)
    expect(elapsedMs({ startedAt: null, completedAt: null }, 5)).toBeNull()
    expect(elapsedMs({ startedAt: 'nope', completedAt: null }, 5)).toBeNull()
  })
  it('splits into seconds, minutes, hours', () => {
    expect(elapsedParts(45_000)).toMatchObject({ key: 's', n: 45 })
    expect(elapsedParts(80_000)).toMatchObject({ key: 'm', m: 1, s: 20 })
    expect(elapsedParts(3_900_000)).toMatchObject({ key: 'h', h: 1, m: 5 })
  })
})

describe('panelState', () => {
  const base = { fail: null, pr: null, branch: 'feat', base: 'main', upstream: 'origin/feat', ahead: 0 }
  it('hides itself when GitHub is not usable', () => {
    for (const error of ['gh.notGithub', 'gh.noAccount', 'gh.noAccess', 'denied'] as const) expect(panelState({ ...base, fail: { ok: false, error } }).kind).toBe('hidden')
  })
  it('shows the missing permission on gh.scope', () => {
    expect(panelState({ ...base, fail: { ok: false, error: 'gh.scope', permission: 'pull_requests' } })).toEqual({ kind: 'scope', permission: 'pull_requests' })
  })
  it('offers Create PR when there is none, and asks for a push first when needed', () => {
    expect(panelState(base)).toEqual({ kind: 'noPr', canCreate: true, needsPush: null })
    expect(panelState({ ...base, upstream: null })).toMatchObject({ needsPush: 'branch' })
    expect(panelState({ ...base, ahead: 2 })).toMatchObject({ needsPush: 'ahead' })
  })
  it('stays hidden on the base branch, on a detached head and while loading', () => {
    expect(panelState({ ...base, branch: 'main' }).kind).toBe('hidden')
    expect(panelState({ ...base, branch: null }).kind).toBe('hidden')
    expect(panelState({ ...base, pr: undefined }).kind).toBe('hidden')
  })
  it('shows the PR when found', () => {
    expect(panelState({ ...base, pr: pr() }).kind).toBe('pr')
  })
})

describe('canMerge', () => {
  it('needs an open, non-draft PR GitHub does not call unmergeable', () => {
    expect(canMerge(pr())).toBe(true)
    expect(canMerge(pr({ state: 'merged' }))).toBe(false)
    expect(canMerge(pr({ draft: true }))).toBe(false)
    expect(canMerge(pr({ mergeable: false }))).toBe(false)
    expect(canMerge(pr({ mergeable: null }))).toBe(true)
  })
})

describe('merge method memory', () => {
  const mem = () => {
    const m = new Map<string, string>()
    return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) }
  }
  it('remembers one method per repo and defaults to merge', () => {
    const s = mem()
    expect(readMethod('/a', s)).toBe('merge')
    writeMethod('/a', 'squash', s)
    writeMethod('/b', 'rebase', s)
    expect(readMethod('/a', s)).toBe('squash')
    expect(readMethod('/b', s)).toBe('rebase')
  })
  it('ignores a broken value', () => {
    const s = mem()
    s.setItem('ccm.gh.mergeMethod', '{bad')
    expect(readMethod('/a', s)).toBe('merge')
    s.setItem('ccm.gh.mergeMethod', JSON.stringify({ '/a': 'fast-forward' }))
    expect(readMethod('/a', s)).toBe('merge')
  })
})

describe('chipState', () => {
  const now = 1_000_000
  it('shows pending and failure until something changes', () => {
    expect(chipState({ state: 'pending', at: 0, branch: 'x' }, now)).toBe('pending')
    expect(chipState({ state: 'failure', at: 0, branch: 'x' }, now)).toBe('failure')
  })
  it('shows success for 10 minutes then hides', () => {
    expect(chipState({ state: 'success', at: now - CHIP_HOLD_MS + 1, branch: 'x' }, now)).toBe('success')
    expect(chipState({ state: 'success', at: now - CHIP_HOLD_MS, branch: 'x' }, now)).toBeNull()
  })
  it('hides neutral, none and unknown', () => {
    expect(chipState({ state: 'neutral', at: now, branch: 'x' }, now)).toBeNull()
    expect(chipState({ state: 'none', at: now, branch: 'x' }, now)).toBeNull()
    expect(chipState(undefined, now)).toBeNull()
  })
})

describe('toastParts and errorKey', () => {
  it('maps CI results to a toast', () => {
    expect(toastParts({ state: 'success', failed: [], branch: 'b' })?.kind).toBe('ok')
    expect(toastParts({ state: 'failure', failed: ['lint'], branch: 'b' })).toMatchObject({ kind: 'bad', body: 'v111pr.toast.bodyFailed' })
    expect(toastParts({ state: 'failure', failed: [], branch: 'b' })?.body).toBe('v111pr.toast.body')
    expect(toastParts({ state: 'pending', failed: [], branch: 'b', timedOut: true })?.title).toBe('v111pr.toast.timeoutTitle')
    expect(toastParts({ state: 'pending', failed: [], branch: 'b' })).toBeNull()
  })
  it('falls back to a generic error', () => {
    expect(errorKey('gh.logGone')).toBe('v111pr.error.gh.logGone')
    expect(errorKey('gh.forkHead')).toBe('v111pr.error.gh.forkHead')
    expect(errorKey('weird')).toBe('v111pr.error.unknown')
    expect(errorKey(undefined)).toBe('v111pr.error.unknown')
  })
})

describe('permissionLabel', () => {
  it('turns the accepted-permissions header into readable text', () => {
    expect(permissionLabel('pull_requests=read')).toBe('pull_requests (read)')
    expect(permissionLabel('pull_requests=write, contents=read')).toBe('pull_requests (write), contents (read)')
    expect(permissionLabel('Pull requests (write)')).toBe('Pull requests (write)')
    expect(permissionLabel(null)).toBe('')
  })
})
