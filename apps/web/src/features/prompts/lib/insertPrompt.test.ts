import { describe, expect, it } from 'vitest'
import { planInsert, type InsertTarget } from './insertPrompt'

const agent: InsertTarget = { agent: true, live: true }
const ctx = { branch: 'main', project: 'app', cwd: 'D:\\work\\app' }

describe('planInsert', () => {
  it('types into the input line without submitting and without waiting for idle', () => {
    const plan = planInsert(agent, 'Review {branch}', ctx)
    expect(plan.ok).toBe(true)
    if (!plan.ok) return
    expect(plan.patch.prefill).toBe('Review main')
    expect(plan.patch.prefillSubmit).toBeUndefined()
    expect(plan.patch.prefillNow).toBe(true)
    expect(plan.patch.prefillSubmit).toBeFalsy()
  })
  it('keeps every line of a multi line prompt', () => {
    const plan = planInsert(agent, 'line one\n\n  indented\nlast', ctx)
    expect(plan.ok && plan.patch.prefill).toBe('line one\n\n  indented\nlast')
  })
  it('only types a prompt that starts with a slash', () => {
    const plan = planInsert(agent, '/review {project}', ctx)
    expect(plan.ok && plan.patch.prefill).toBe('/review app')
    expect(plan.ok && plan.patch.prefillSubmit).toBeUndefined()
  })
  it('refuses a pane that already holds a prefill or runs a compact', () => {
    expect(planInsert({ ...agent, prefill: 'x' }, 'p', ctx)).toEqual({ ok: false, reason: 'busy' })
    expect(planInsert({ ...agent, compact: { step: 'write' } }, 'p', ctx)).toEqual({ ok: false, reason: 'busy' })
  })
  it('refuses a missing, ended or shell pane', () => {
    expect(planInsert(null, 'p', ctx)).toEqual({ ok: false, reason: 'noPane' })
    expect(planInsert({ ...agent, live: false }, 'p', ctx)).toEqual({ ok: false, reason: 'noPane' })
    expect(planInsert({ agent: false, live: true }, 'p', ctx)).toEqual({ ok: false, reason: 'notAgent' })
  })
  it('reports missing variables without blocking', () => {
    const plan = planInsert(agent, 'On {branch} {project}', { project: 'app' })
    expect(plan).toMatchObject({ ok: true, missing: ['branch'] })
    expect(plan.ok && plan.patch.prefill).toBe('On {branch} app')
  })
  it('refuses an agent that waits for an answer, so the text is never typed into the question later', () => {
    expect(planInsert({ ...agent, status: 'waiting' }, 'p', ctx)).toEqual({ ok: false, reason: 'waiting' })
  })
  it('refuses an agent in a state that never types the prefill', () => {
    expect(planInsert({ ...agent, status: 'error' }, 'p', ctx)).toEqual({ ok: false, reason: 'busy' })
    expect(planInsert({ ...agent, status: 'unknown' }, 'p', ctx)).toEqual({ ok: false, reason: 'busy' })
  })
  it('allows an idle agent', () => {
    expect(planInsert({ ...agent, status: 'idle' }, 'p', ctx).ok).toBe(true)
  })
  it('allows an agent that is working', () => {
    expect(planInsert(agent, 'p', {}).ok).toBe(true)
  })
})
