import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import type { RelaySetupStatus } from '@ccm/shared'

const ipc = vi.hoisted(() => ({
  relayStatus: vi.fn(),
  relayToken: vi.fn(),
  relayDeploy: vi.fn(),
  relayRemove: vi.fn(),
  progress: null as null | ((p: { step: string }) => void),
}))

vi.mock('./useRemoteSetupIpc', () => ({
  relayStatus: ipc.relayStatus,
  relayToken: ipc.relayToken,
  relayDeploy: ipc.relayDeploy,
  relayRemove: ipc.relayRemove,
  onRelayProgress: (fn: (p: { step: string }) => void) => {
    ipc.progress = fn
    return () => undefined
  },
}))

import { useRemoteRelay } from './useRemoteRelay'

const status = (over: Partial<RelaySetupStatus> = {}): RelaySetupStatus => ({ state: 'none', updateAvailable: false, hasToken: false, ...over })
const flush = async (): Promise<void> => {
  for (let i = 0; i < 6; i++) await Promise.resolve()
}

function boot() {
  let api!: ReturnType<typeof useRemoteRelay>
  const wrapper = mount(
    defineComponent({
      setup() {
        api = useRemoteRelay()
        return () => h('div')
      },
    }),
  )
  return { api, wrapper }
}

const READY = status({ state: 'ready', url: 'https://r.workers.dev', account: 'Me' })

describe('relay setup state', () => {
  beforeEach(() => {
    ipc.relayStatus.mockReset().mockResolvedValue(status())
    ipc.relayToken.mockReset()
    ipc.relayDeploy.mockReset()
    ipc.relayRemove.mockReset()
    ipc.progress = null
  })

  it('checks the token, clears it from the page and preselects a single account', async () => {
    ipc.relayToken.mockResolvedValue({ ok: true, accounts: [{ id: 'a1', name: 'Me' }] })
    const { api } = boot()
    await flush()
    api.token.value = '  secret-token  '
    await api.check()
    expect(ipc.relayToken).toHaveBeenCalledWith('secret-token')
    expect(api.token.value).toBe('')
    expect(api.picked.value).toBe('a1')
    expect(api.error.value).toBeNull()
  })

  it('leaves the account unpicked when there are several', async () => {
    ipc.relayToken.mockResolvedValue({
      ok: true,
      accounts: [
        { id: 'a1', name: 'A' },
        { id: 'a2', name: 'B' },
      ],
    })
    const { api } = boot()
    api.token.value = 't'
    await api.check()
    expect(api.accounts.value).toHaveLength(2)
    expect(api.picked.value).toBe('')
  })

  it('keeps the error code of a refused token and never keeps the token', async () => {
    ipc.relayToken.mockResolvedValue({ ok: false, error: 'token_scope' })
    const { api } = boot()
    api.token.value = 'bad'
    await api.check()
    expect(api.error.value).toBe('token_scope')
    expect(api.token.value).toBe('')
    expect(api.accounts.value).toEqual([])
  })

  it('ignores an empty token', async () => {
    const { api } = boot()
    api.token.value = '   '
    await api.check()
    expect(ipc.relayToken).not.toHaveBeenCalled()
  })

  it('follows deploy progress events and ends on the new status', async () => {
    let finish!: (v: unknown) => void
    ipc.relayDeploy.mockReturnValue(new Promise((r) => (finish = r)))
    const { api } = boot()
    const run = api.deploy('a1')
    expect(api.state.value).toBe('deploying')
    ipc.progress?.({ step: 'upload' })
    expect(api.step.value).toBe('upload')
    ipc.relayStatus.mockResolvedValue(READY)
    finish({ ok: true, url: 'https://r.workers.dev' })
    await run
    expect(api.state.value).toBe('ready')
    expect(api.step.value).toBeNull()
  })

  it('keeps the error of a failed deploy and does not start a second one meanwhile', async () => {
    let finish!: (v: unknown) => void
    ipc.relayDeploy.mockReturnValue(new Promise((r) => (finish = r)))
    const { api } = boot()
    const run = api.deploy('a1')
    await api.deploy('a1')
    expect(ipc.relayDeploy).toHaveBeenCalledTimes(1)
    finish({ ok: false, error: 'offline' })
    await run
    expect(api.error.value).toBe('offline')
    expect(api.state.value).toBe('none')
  })

  it('shows a deploy that is still running after the page was left and came back', async () => {
    vi.useFakeTimers()
    ipc.relayStatus.mockResolvedValue(status({ state: 'deploying' }))
    const { api, wrapper } = boot()
    await vi.advanceTimersByTimeAsync(0)
    expect(api.state.value).toBe('deploying')
    expect(api.busy.value).toBe(true)
    ipc.relayStatus.mockResolvedValue(READY)
    await vi.advanceTimersByTimeAsync(2100)
    expect(api.state.value).toBe('ready')
    wrapper.unmount()
    vi.useRealTimers()
  })

  it('reports that the Worker stays when the shell could not delete it', async () => {
    ipc.relayStatus.mockResolvedValue(READY)
    ipc.relayRemove.mockResolvedValue({ ok: true, workerDeleted: false })
    const { api } = boot()
    await flush()
    await api.remove(true)
    expect(ipc.relayRemove).toHaveBeenCalledWith(true)
    expect(api.notice.value).toBe('workerKept')
  })

  it('treats a cancelled native confirm as no change and no error', async () => {
    ipc.relayStatus.mockResolvedValue(READY)
    ipc.relayRemove.mockResolvedValue({ ok: false, error: 'cancelled' })
    const { api } = boot()
    await flush()
    await api.remove(true)
    expect(api.notice.value).toBe('cancelled')
    expect(api.error.value).toBeNull()
    expect(api.state.value).toBe('ready')
  })

  it('asks for the token again when the saved token or account is gone', async () => {
    ipc.relayStatus.mockResolvedValue({ ...READY, updateAvailable: true })
    ipc.relayDeploy.mockResolvedValue({ ok: false, error: 'no_token' })
    const { api } = boot()
    await flush()
    await api.update()
    expect(ipc.relayDeploy).toHaveBeenCalledWith('')
    expect(api.error.value).toBeNull()
    expect(api.updating.value).toBe(true)
    expect(api.showSteps.value).toBe(true)
  })

  it('updates with the saved account in one tap', async () => {
    ipc.relayStatus.mockResolvedValue({ ...READY, updateAvailable: true })
    ipc.relayDeploy.mockResolvedValue({ ok: true, url: 'https://r.workers.dev' })
    const { api } = boot()
    await flush()
    await api.update()
    expect(ipc.relayDeploy).toHaveBeenCalledWith('')
    expect(api.updating.value).toBe(false)
  })

  it('drops the token when the page is left', () => {
    const { api, wrapper } = boot()
    api.token.value = 'typed'
    wrapper.unmount()
    expect(api.token.value).toBe('')
  })
})
