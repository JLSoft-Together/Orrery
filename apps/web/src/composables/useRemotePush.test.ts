import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'

const ipc = vi.hoisted(() => ({ pushStatus: vi.fn(), pushSetup: vi.fn(), pushTest: vi.fn(), pushRemove: vi.fn() }))
vi.mock('./useRemoteSetupIpc', () => ipc)

import { useRemotePush } from './useRemotePush'

const gs = (p = 'orrery-1') => new File([JSON.stringify({ project_info: { project_id: p } })], 'google-services.json')
const sa = (p = 'orrery-1') => new File([JSON.stringify({ type: 'service_account', project_id: p })], 'key.json')

function boot() {
  let api!: ReturnType<typeof useRemotePush>
  const wrapper = mount(
    defineComponent({
      setup() {
        api = useRemotePush()
        return () => h('div')
      },
    }),
  )
  return { api, wrapper }
}

describe('push setup state', () => {
  beforeEach(() => {
    Object.values(ipc).forEach((f) => f.mockReset())
    ipc.pushStatus.mockResolvedValue({ configured: false, tokens: 0 })
  })

  it('sends both files once and forgets their content', async () => {
    ipc.pushSetup.mockResolvedValue({ ok: true, projectId: 'orrery-1' })
    const { api } = boot()
    await api.add([gs(), sa()])
    expect(api.canSend.value).toBe(true)
    ipc.pushStatus.mockResolvedValue({ configured: true, projectId: 'orrery-1', tokens: 2 })
    await api.send()
    expect(ipc.pushSetup).toHaveBeenCalledTimes(1)
    expect(ipc.pushSetup.mock.calls[0]?.[0]).toContain('project_info')
    expect(api.files.take()).toBeNull()
    expect(api.configured.value).toBe(true)
  })

  it('does not send two files of different projects', async () => {
    const { api } = boot()
    await api.add([gs('a-1'), sa('b-1')])
    expect(api.problem.value).toBe('mismatch_project')
    await api.send()
    expect(ipc.pushSetup).not.toHaveBeenCalled()
  })

  it('shows the code the shell returns and keeps the files for a retry', async () => {
    ipc.pushSetup.mockResolvedValue({ ok: false, error: 'no_android_client' })
    const { api } = boot()
    await api.add([gs(), sa()])
    await api.send()
    expect(api.problem.value).toBe('no_android_client')
    expect(api.files.complete.value).toBe(true)
  })

  it('reports a test result or its error code', async () => {
    ipc.pushStatus.mockResolvedValue({ configured: true, projectId: 'p-1', tokens: 1 })
    ipc.pushTest.mockResolvedValueOnce({ ok: true, sent: 1 }).mockResolvedValueOnce({ ok: false, error: 'unregistered' })
    const { api } = boot()
    await api.test('dev-1')
    expect(ipc.pushTest).toHaveBeenCalledWith('dev-1')
    expect(api.sent.value).toBe(1)
    await api.test()
    expect(api.sent.value).toBeNull()
    expect(api.error.value).toBe('unregistered')
  })

  it('clears held files when the page is left', async () => {
    const { api, wrapper } = boot()
    await api.add([gs()])
    wrapper.unmount()
    expect(api.files.googleServices.value).toBeNull()
  })
})
