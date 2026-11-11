import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { REMOTE_SUMMARY_DEBOUNCE_MS, type RemoteDevice, type RemoteStatus } from '@ccm/shared'

const device = (state: RemoteDevice['state']): RemoteDevice => ({ deviceId: 'd1', name: 'Pixel', platform: 'android', pairedAt: 1, lastSeen: 1, state })
const status = (devices: RemoteDevice[]): RemoteStatus => ({ enabled: true, listening: true, addresses: [], port: 4320, relay: 'off', keepAwake: true, devices })

describe('workspace summary for phones', () => {
  const publish = vi.fn(async (_s: unknown) => true)
  let ws: Awaited<ReturnType<typeof boot>>['ws']
  let remote: Awaited<ReturnType<typeof boot>>['remote']
  let ask: () => unknown

  async function boot() {
    vi.resetModules()
    setActivePinia(createPinia())
    let handler: ((args: unknown, caller: unknown) => unknown) | undefined
    ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = { platform: 'win32', onOrrery: () => undefined }
    vi.doMock('../lib/remote', async (orig) => ({ ...(await orig<typeof import('../lib/remote')>()), remotePublish: publish }))
    vi.doMock('./usePaneInfo', () => ({
      usePaneInfo: () => ({ sessionOf: () => undefined, railStateOf: () => 'idle', titleOf: () => 'Agent', isBusy: () => false, subagentsOf: () => 0 }),
    }))
    vi.doMock('./useOrreryApi', () => ({ hasOrrery: () => true, handleOrrery: (_a: string, h: typeof handler) => (handler = h) }))
    const mod = await import('./useRemoteSummary')
    const store = await import('../stores/workspace')
    const remoteStore = await import('../stores/remote')
    mod.installRemoteSummary()
    return { ws: store.useWorkspaceStore(), remote: remoteStore.useRemoteStore(), ask: () => handler!({}, { source: 'phone' }) }
  }

  beforeEach(async () => {
    vi.useFakeTimers()
    publish.mockClear()
    localStorage.clear()
    sessionStorage.clear()
    ;({ ws, remote, ask } = await boot())
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('sends nothing while no phone is connected', async () => {
    ws.open('claude', 'C:\\work\\A')
    await nextTick()
    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS * 3)
    expect(publish).not.toHaveBeenCalled()
    remote.status = status([device('idle')])
    await nextTick()
    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS * 3)
    expect(publish).not.toHaveBeenCalled()
  })

  it('publishes once per burst after a phone connects, with a higher revision per change, and skips an unchanged summary', async () => {
    remote.status = status([device('connected')])
    await nextTick()
    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS + 10)
    expect(publish).toHaveBeenCalledTimes(1)
    const first = publish.mock.calls[0]![0] as { rev: number; tabs: unknown[] }
    expect(first.tabs).toHaveLength(0)

    ws.open('claude', 'C:\\work\\A')
    ws.open('shell', 'C:\\work\\B')
    await nextTick()
    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS - 20)
    expect(publish).toHaveBeenCalledTimes(1)
    await vi.advanceTimersByTimeAsync(40)
    expect(publish).toHaveBeenCalledTimes(2)
    const second = publish.mock.calls[1]![0] as { rev: number; tabs: unknown[] }
    expect(second.tabs).toHaveLength(2)
    expect(second.rev).toBeGreaterThan(first.rev)

    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS * 5)
    expect(publish).toHaveBeenCalledTimes(2)
  })

  it('stops publishing once the last phone disconnects', async () => {
    remote.status = status([device('connected')])
    await nextTick()
    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS + 10)
    publish.mockClear()
    ws.open('claude', 'C:\\work\\A')
    remote.status = status([device('idle')])
    await nextTick()
    await vi.advanceTimersByTimeAsync(REMOTE_SUMMARY_DEBOUNCE_MS * 3)
    expect(publish).not.toHaveBeenCalled()
  })

  it('answers the gateway with the current summary even with no phone connected, bumping the revision only on a change', async () => {
    const a = ask() as { rev: number; tabs: unknown[] }
    const same = ask() as { rev: number }
    expect(same.rev).toBe(a.rev)
    ws.open('claude', 'C:\\work\\A')
    const b = ask() as { rev: number; tabs: unknown[] }
    expect(b.tabs).toHaveLength(1)
    expect(b.rev).toBe(a.rev + 1)
  })
})
