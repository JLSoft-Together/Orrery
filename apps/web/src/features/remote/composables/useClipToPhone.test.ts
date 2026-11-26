import { beforeEach, describe, expect, it, vi } from 'vitest'

const { push, sendClipboardToPhone } = vi.hoisted(() => ({ push: vi.fn(), sendClipboardToPhone: vi.fn() }))

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))
vi.mock('../../../core/stores/toasts', () => ({ useToastsStore: () => ({ push }) }))
vi.mock('../lib/remoteClip', async (orig) => ({ ...(await orig<typeof import('../lib/remoteClip')>()), sendClipboardToPhone }))

import { useClipToPhone } from './useClipToPhone'

beforeEach(() => {
  push.mockClear()
  sendClipboardToPhone.mockReset()
})

describe('useClipToPhone', () => {
  it('toasts info when a phone received the clipboard', async () => {
    sendClipboardToPhone.mockResolvedValue({ ok: true, sent: 1 })
    await useClipToPhone().send()
    expect(push).toHaveBeenCalledWith(expect.objectContaining({ kind: 'info', text: 'remoteClip.sent' }))
  })

  it('tells the owner to open the phone app when no session is open', async () => {
    sendClipboardToPhone.mockResolvedValue({ ok: false, error: 'noSession' })
    await useClipToPhone().send()
    expect(push).toHaveBeenCalledWith(expect.objectContaining({ kind: 'warning', text: 'remoteClip.noSession' }))
  })

  it('blocks text that looks like a secret', async () => {
    sendClipboardToPhone.mockResolvedValue({ ok: false, error: 'secret' })
    await useClipToPhone().send()
    expect(push).toHaveBeenCalledWith(expect.objectContaining({ kind: 'warning', text: 'remoteClip.secret' }))
  })
})
