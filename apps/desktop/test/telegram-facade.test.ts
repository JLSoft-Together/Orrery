import { describe, expect, it } from 'vitest'
import type { DeskRouter } from '../src/desk/forward'
import { TelegramFacade } from '../src/telegram/facade'
import type { TelegramNotifier } from '../src/telegram/notifier'

function fakeNode() {
  const seen: string[] = []
  const node = {
    state: () => (seen.push('state'), { enabled: true }),
    set: () => (seen.push('set'), { ok: true }),
    routeFor: () => (seen.push('routeFor'), null),
    report: async () => (seen.push('report'), { ok: true, topic: 'n' }),
    handle: async () => (seen.push('handle'), true),
    alert: async () => (seen.push('alert'), true),
    runState: async () => void seen.push('runState'),
    runFailedReport: async () => (seen.push('runFailed'), true),
    tick: async () => (seen.push('tick'), true),
    stop: () => void seen.push('stop'),
  }
  return { node: node as unknown as TelegramNotifier, seen }
}

function fakeDesk(mode: 'rust' | 'node', answers: Record<string, unknown> = {}) {
  const calls: Array<[string, unknown[]]> = []
  const desk = {
    mode: () => mode,
    invoke: async (channel: string, args: unknown[]) => {
      calls.push([channel, args])
      return channel in answers ? answers[channel] : null
    },
  }
  return { desk: desk as unknown as DeskRouter, calls }
}

describe('TelegramFacade', () => {
  it('node mode uses the TS notifier only', async () => {
    const { node, seen } = fakeNode()
    const { desk, calls } = fakeDesk('node')
    const tg = new TelegramFacade(node, () => desk)
    expect(tg.rustMode).toBe(false)
    expect(await tg.state()).toEqual({ enabled: true })
    await tg.handle({ type: 'telegram', episode: 'e', reason: 'done', terminalId: 't', name: 'n', cwd: '/p' })
    await tg.tick()
    tg.stop()
    expect(seen).toEqual(['state', 'handle', 'tick', 'stop'])
    expect(calls).toEqual([])
  })

  it('rust mode sends each call to its desk channel and keeps the node notifier idle', async () => {
    const { node, seen } = fakeNode()
    const { desk, calls } = fakeDesk('rust', { 'desk:tg-state': { enabled: false }, 'desk:tg-set': { ok: true, state: {} }, 'desk:tg-report': { ok: true, topic: 'r' }, 'desk:tg-handle': false, 'desk:tg-route-for': { project: '/p' }, 'desk:tg-tick': true })
    const tg = new TelegramFacade(node, () => desk)
    expect(await tg.state()).toEqual({ enabled: false })
    expect(await tg.set({ receiving: true })).toEqual({ ok: true, state: {} })
    expect(await tg.routeFor('/p/a')).toEqual({ project: '/p' })
    expect(await tg.report('/p', 'hi', undefined, 'pane-1', [{ label: 'A', value: 'a' }])).toEqual({ ok: true, topic: 'r' })
    expect(await tg.handle({ type: 'telegram', episode: 'e', reason: 'done', terminalId: 't', name: 'n', cwd: '/p' })).toBe(false)
    expect(await tg.alert('disk', { label: 'C:', free: '1 GB' })).toBe(false)
    expect(await tg.tick()).toBe(true)
    tg.progress(3, 0.5)
    expect(calls.map((c) => c[0])).toEqual(['desk:tg-state', 'desk:tg-set', 'desk:tg-route-for', 'desk:tg-report', 'desk:tg-handle', 'desk:tg-alert', 'desk:tg-tick', 'desk:tg-upload-progress'])
    expect(calls[3]![1]).toEqual([{ cwd: '/p', text: 'hi', from: 'pane-1', buttons: [{ label: 'A', value: 'a' }] }])
    expect(seen).toEqual([])
  })
})
