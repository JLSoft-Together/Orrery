import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'

const ipc = vi.hoisted(() => ({
  relayStatus: vi.fn(),
  relayToken: vi.fn(),
  relayDeploy: vi.fn(),
  relayRemove: vi.fn(),
  relayQuick: vi.fn(),
  relayCancel: vi.fn(),
  relayClipboard: vi.fn(),
  onRelayProgress: vi.fn(() => () => undefined),
}))
vi.mock('./useRemoteSetupIpc', () => ipc)

import { CLIP_ARM_MS, RELAY_QUICK_STEPS, RELAY_TOKEN_URL, useRemoteRelay } from './useRemoteRelay'

const A = '80786e06fd9be997d29bc5bc4de835e6'
const B = '11111111111111111111111111111111'

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

describe('relay setup flows', () => {
  beforeEach(() => {
    Object.values(ipc).forEach((f) => f.mockReset())
    ipc.onRelayProgress.mockReturnValue(() => undefined)
    ipc.relayStatus.mockResolvedValue({ state: 'none', updateAvailable: false, hasToken: false })
    ipc.relayDeploy.mockResolvedValue({ ok: true, url: 'https://orrery-relay.x.workers.dev' })
  })

  it('the token page link pre-fills exactly the two permissions the deploy needs', () => {
    const url = new URL(RELAY_TOKEN_URL)
    expect(url.origin + url.pathname).toBe('https://dash.cloudflare.com/profile/api-tokens')
    expect(JSON.parse(url.searchParams.get('permissionGroupKeys') ?? '')).toEqual([
      { key: 'workers_scripts', type: 'edit' },
      { key: 'account_settings', type: 'read' },
    ])
    expect(url.searchParams.get('name')).toBe('Orrery relay')
    expect(url.searchParams.get('accountId')).toBe('*')
  })

  it('one tap shows the wrangler steps and asks for an account when there are several', async () => {
    ipc.relayQuick.mockResolvedValueOnce({ ok: false, error: 'pick_account', accounts: [{ id: A, name: 'Jin' }, { id: B, name: 'Work' }] })
    const { api } = boot()
    expect(api.steps.value).toEqual(RELAY_QUICK_STEPS)
    await api.quick()
    expect(api.accounts.value).toHaveLength(2)
    expect(api.pickFor.value).toBe('quick')
    expect(api.error.value).toBeNull()
    ipc.relayQuick.mockResolvedValueOnce({ ok: true, url: 'https://orrery-relay.x.workers.dev' })
    api.picked.value = B
    await api.build()
    expect(ipc.relayQuick).toHaveBeenLastCalledWith(B)
    expect(ipc.relayDeploy).not.toHaveBeenCalled()
    expect(api.accounts.value).toEqual([])
  })

  it('no Node switches to the token path and keeps the detail of a failed deploy', async () => {
    ipc.relayQuick.mockResolvedValueOnce({ ok: false, error: 'no_node' })
    const { api } = boot()
    await api.quick()
    expect(api.error.value).toBe('no_node')
    expect(api.tokenMode.value).toBe(true)
    ipc.relayQuick.mockResolvedValueOnce({ ok: false, error: 'deploy_failed', detail: 'Authentication error [code: 10000]' })
    await api.quick()
    expect(api.detail.value).toContain('10000')
  })

  it('a cancelled run shows no error', async () => {
    ipc.relayQuick.mockResolvedValueOnce({ ok: false, error: 'cancelled' })
    const { api } = boot()
    await api.quick()
    expect(api.error.value).toBeNull()
  })

  it('a token with one account deploys by itself', async () => {
    ipc.relayToken.mockResolvedValue({ ok: true, accounts: [{ id: A, name: 'Jin' }] })
    const { api } = boot()
    api.token.value = 'cfut_token'
    await api.check()
    expect(ipc.relayDeploy).toHaveBeenCalledWith(A)
    expect(api.token.value).toBe('')
  })

  it('the clipboard is read on focus only after the token page was opened, and only once', async () => {
    ipc.relayClipboard.mockResolvedValue({ ok: true, accounts: [{ id: A, name: 'Jin' }, { id: B, name: 'Work' }] })
    const { api } = boot()
    await api.readClipboard()
    expect(ipc.relayClipboard).not.toHaveBeenCalled()
    api.armClipboard()
    window.dispatchEvent(new Event('focus'))
    await vi.waitFor(() => expect(api.accounts.value).toHaveLength(2))
    expect(api.pickFor.value).toBe('token')
    expect(ipc.relayDeploy).not.toHaveBeenCalled()
    await api.readClipboard()
    expect(ipc.relayClipboard).toHaveBeenCalledTimes(1)
  })

  it('a clipboard without a token keeps waiting until the arm runs out', async () => {
    ipc.relayClipboard.mockResolvedValue(null)
    const { api } = boot()
    const now = Date.now()
    api.armClipboard(now)
    await api.readClipboard(now + 1000)
    await api.readClipboard(now + CLIP_ARM_MS + 1)
    expect(ipc.relayClipboard).toHaveBeenCalledTimes(1)
  })

  it('an update of a wrangler relay goes through one tap again', async () => {
    ipc.relayStatus.mockResolvedValue({ state: 'ready', url: 'https://orrery-relay.x.workers.dev', updateAvailable: true, hasToken: false, via: 'wrangler' })
    ipc.relayQuick.mockResolvedValue({ ok: true, url: 'https://orrery-relay.x.workers.dev' })
    const { api } = boot()
    await nextTick()
    await vi.waitFor(() => expect(api.status.value?.via).toBe('wrangler'))
    await api.update()
    expect(ipc.relayQuick).toHaveBeenCalledWith('')
    expect(ipc.relayDeploy).not.toHaveBeenCalled()
  })

  it('cancel only asks the shell while deploying', async () => {
    const { api } = boot()
    await api.cancel()
    expect(ipc.relayCancel).not.toHaveBeenCalled()
    let finish!: (v: unknown) => void
    ipc.relayQuick.mockReturnValue(new Promise((r) => (finish = r)))
    const run = api.quick()
    await api.cancel()
    expect(ipc.relayCancel).toHaveBeenCalledTimes(1)
    finish({ ok: false, error: 'cancelled' })
    await run
  })
})

describe('quick result parsing', () => {
  it('keeps only safe extras', async () => {
    const real = await vi.importActual<typeof import('./useRemoteSetupIpc')>('./useRemoteSetupIpc')
    expect(real.parseQuick({ ok: false, error: 'subdomain_missing', link: `https://dash.cloudflare.com/${A}/workers/onboarding` })).toEqual({
      ok: false,
      error: 'subdomain_missing',
      link: `https://dash.cloudflare.com/${A}/workers/onboarding`,
    })
    expect(real.parseQuick({ ok: false, error: 'subdomain_missing', link: 'https://evil.example/' })).toEqual({ ok: false, error: 'subdomain_missing' })
    expect(real.parseQuick({ ok: false, error: 'pick_account', accounts: [{ id: A, name: 'Jin' }, { id: 1 }] })).toEqual({ ok: false, error: 'pick_account', accounts: [{ id: A, name: 'Jin' }] })
    expect(real.parseQuick(null)).toEqual({ ok: false, error: 'rejected' })
  })
})
