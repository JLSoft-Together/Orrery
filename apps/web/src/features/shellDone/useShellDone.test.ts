import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../i18n/en'
import vi_ from '../../i18n/vi'
import { useConnectionStore } from '../../stores/connection'
import { useToastsStore } from '../../stores/toasts'
import { useWorkspaceStore } from '../../stores/workspace'
import { useShellDone, type CmdMessage } from './useShellDone'

const i18n = createI18n({ legacy: false, locale: 'vi', messages: { en, vi: vi_ } })

function rig(visible = false) {
  let api: ReturnType<typeof useShellDone> | null = null
  const Probe = defineComponent({
    setup() {
      api = useShellDone({ paneKey: () => 'k1', paneTitle: () => 'shell api', visible: () => visible })
      return () => h('div')
    },
  })
  mount(Probe, { global: { plugins: [i18n] } })
  return api!
}

const end = (code: number, ms: number, command = 'npm run build'): CmdMessage => ({ t: 'cmd', phase: 'end', command, code, ms })

describe('useShellDone', () => {
  let focused = true
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    focused = true
    vi.spyOn(document, 'hasFocus').mockImplementation(() => focused)
    const ws = useWorkspaceStore()
    ws.panes.push({ key: 'k1', preset: 'shell', state: 'running' } as never)
  })

  it('records every finished command and ignores start marks', () => {
    const s = rig(true)
    s.onCmd({ t: 'cmd', phase: 'start', command: 'x' })
    expect(useWorkspaceStore().panes[0]!.shellLast).toBeUndefined()
    s.onCmd(end(0, 500))
    expect(useWorkspaceStore().panes[0]!.shellLast).toEqual({ code: 0, ms: 500, command: 'npm run build' })
    expect(useToastsStore().list).toHaveLength(0)
  })

  it('shows an in-app toast and the dot when another tab is in front', () => {
    const s = rig(false)
    s.onCmd(end(2, 133_000))
    const toast = useToastsStore().list[0]!
    expect(toast.text).toBe('Lệnh lỗi (exit 2, 2 phút 13 giây): npm run build')
    expect(toast.kind).toBe('warning')
    expect(toast.attrs?.['data-shell-done-exit']).toBe('2')
    expect(useWorkspaceStore().panes[0]!.attention).toBe('done')
  })

  it('asks the core toaster when the window is not focused', () => {
    focused = false
    const conn = useConnectionStore()
    const sent = vi.spyOn(conn, 'shellDone').mockReturnValue(true)
    rig(true).onCmd(end(0, 12_000, 'make'))
    expect(sent).toHaveBeenCalledWith({ paneKey: 'k1', name: 'shell api', command: 'make', code: 0, ms: 12_000 })
    expect(useToastsStore().list).toHaveLength(0)
  })

  it('stays quiet under the threshold or with the switch off', () => {
    const s = rig(false)
    s.onCmd(end(0, 9_000))
    localStorage.setItem('ccm.shellDone', '0')
    setActivePinia(createPinia())
    useWorkspaceStore().panes.push({ key: 'k1', preset: 'shell', state: 'running' } as never)
    rig(false).onCmd(end(0, 60_000))
    expect(useToastsStore().list).toHaveLength(0)
  })
})
