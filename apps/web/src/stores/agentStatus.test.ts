import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { i18n } from '../i18n'
import AgentStatusChip from '../components/workspace/AgentStatusChip.vue'
import { parseStatusEvent } from '../lib/agentStatus'

describe('parseStatusEvent', () => {
  it('passes a clean status and a clear', () => {
    expect(parseStatusEvent({ key: 'a', status: { phase: 'x', progress: 0.5, updatedAt: 3 } })).toEqual({ key: 'a', status: { phase: 'x', progress: 0.5, updatedAt: 3 } })
    expect(parseStatusEvent({ key: 'a', status: null })).toEqual({ key: 'a', status: null })
  })
  it('drops malformed pushes', () => {
    expect(parseStatusEvent(null)).toBeNull()
    expect(parseStatusEvent({ status: null })).toBeNull()
    expect(parseStatusEvent({ key: 'a', status: { progress: 1, updatedAt: 1 } })).toBeNull()
  })
})

describe('AgentStatusChip', () => {
  const chip = (status: { phase: string; progress?: number; note?: string; updatedAt: number }) => mount(AgentStatusChip, { props: { status }, global: { plugins: [i18n] } })

  it('shows phase and percent, note goes to the tooltip', () => {
    const w = chip({ phase: 'review', progress: 0.42, note: 'file 3 of 7', updatedAt: Date.now() })
    expect(w.text()).toContain('review')
    expect(w.text()).toContain('42%')
    expect(w.find('[data-agent-status]').attributes('data-agent-status')).toBe('live')
    expect(w.find('[data-agent-status]').attributes('title')).toContain('file 3 of 7')
  })
  it('without progress there is no bar', () => {
    const w = chip({ phase: 'plan', updatedAt: Date.now() })
    expect(w.text()).not.toContain('%')
  })
  it('dims after 30 minutes without a write', () => {
    const w = chip({ phase: 'plan', updatedAt: Date.now() - 31 * 60_000 })
    expect(w.find('[data-agent-status]').attributes('data-agent-status')).toBe('stale')
    expect(w.find('[data-agent-status]').attributes('title')).toContain('30')
  })
})

describe('agent status store', () => {
  beforeEach(() => {
    vi.resetModules()
    setActivePinia(createPinia())
  })

  async function load() {
    let push: (e: unknown) => void = () => undefined
    const watch = vi.fn(async () => true)
    vi.doMock('../lib/desktop', async (orig) => ({
      ...(await orig<Record<string, unknown>>()),
      desktop: {
        agentStatusWatch: watch,
        onAgentStatus: (h: (e: unknown) => void) => {
          push = h
          return () => undefined
        },
      },
    }))
    const { useAgentStatusStore } = await import('./agentStatus')
    const { useWorkspaceStore } = await import('./workspace')
    return { store: useAgentStatusStore(), ws: useWorkspaceStore(), watch, push: (e: unknown) => push(e) }
  }

  it('watches running agent panes by working folder, not shell panes, and stops when they close', async () => {
    const { store, ws, watch, push } = await load()
    const agent = ws.open('claude', 'C:\\repo\\a')
    ws.open('shell', 'C:\\repo\\b')
    ws.update(agent.key, { state: 'running' })
    store.start()
    await nextTick()
    expect(watch).toHaveBeenCalledTimes(1)
    expect(watch).toHaveBeenCalledWith(agent.key, 'C:\\repo\\a')

    push({ key: agent.key, status: { phase: 'go', updatedAt: 1 } })
    expect(store.statusOf(agent.key)?.phase).toBe('go')

    ws.update(agent.key, { liveCwd: 'C:\\repo\\a\\sub' })
    await nextTick()
    expect(watch).toHaveBeenLastCalledWith(agent.key, 'C:\\repo\\a\\sub')

    ws.remove(agent.key)
    await nextTick()
    expect(watch).toHaveBeenLastCalledWith(agent.key, null)
    expect(store.statusOf(agent.key)).toBeUndefined()
    push({ key: agent.key, status: { phase: 'late', updatedAt: 2 } })
    expect(store.statusOf(agent.key)).toBeUndefined()
  })
})
