import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'
import { mount } from '@vue/test-utils'
import type { PtyPreset } from '@ccm/shared'
import messages from '../i18n/saved-prompts'

describe('prompt insert and palette provider', () => {
  let ws: Awaited<ReturnType<typeof boot>>['ws']
  let toasts: Awaited<ReturnType<typeof boot>>['toasts']
  let ui: Awaited<ReturnType<typeof boot>>['ui']
  let mod: typeof import('./useInsertPrompt')
  let paletteMod: typeof import('./usePalettePrompts')
  let store: Awaited<ReturnType<typeof boot>>['store']

  async function boot() {
    vi.resetModules()
    setActivePinia(createPinia())
    ;(window as unknown as { ccmDesktop: unknown }).ccmDesktop = { platform: 'win32', ptyEndpoint: async () => null }
    const wsMod = await import('../../workspace/stores/workspace')
    const toastMod = await import('../../../core/stores/toasts')
    const uiMod = await import('../../../core/stores/ui')
    const storeMod = await import('../stores/savedPrompts')
    mod = await import('./useInsertPrompt')
    paletteMod = await import('./usePalettePrompts')
    return { ws: wsMod.useWorkspaceStore(), toasts: toastMod.useToastsStore(), ui: uiMod.useUiStore(), store: storeMod.useSavedPromptsStore() }
  }

  const i18n = () => createI18n({ legacy: false, locale: 'en', messages: { en: messages.en, vi: messages.vi } })

  function insertWith<T>(fn: (api: ReturnType<typeof mod.usePromptInsert>) => T): T {
    let out!: T
    mount(
      defineComponent({
        setup() {
          out = fn(mod.usePromptInsert())
          return () => h('div')
        },
      }),
      { global: { plugins: [i18n()] } },
    )
    return out
  }

  function pane(preset: PtyPreset, cwd: string, patch: Record<string, unknown> = {}) {
    const p = ws.open(preset, cwd)
    ws.update(p.key, { state: 'running', ...patch })
    ws.activate(p.key)
    return () => ws.panes.find((x) => x.key === p.key)!
  }

  beforeEach(async () => {
    localStorage.clear()
    sessionStorage.clear()
    ;({ ws, toasts, ui, store } = await boot())
  })

  it('types the prompt into the focused agent pane and never submits', () => {
    const get = pane('claude', 'D:\\work\\app', { branch: 'feat/x' })
    expect(insertWith((a) => a.insert({ body: 'Review {branch} in {project}\nthen stop' }))).toBe('inserted')
    expect(get().prefill).toBe('Review feat/x in app\nthen stop')
    expect(get().prefillNow).toBe(true)
    expect(get().prefillSubmit).toBeFalsy()
  })

  it('inserts a prompt that starts with a slash as text only', () => {
    const get = pane('codex', 'D:\\work\\app')
    insertWith((a) => a.insert({ body: '/review' }))
    expect(get().prefill).toBe('/review')
    expect(get().prefillSubmit).toBeFalsy()
  })

  it('keeps an unknown variable as written and says so', () => {
    const get = pane('claude', 'D:\\work\\app')
    insertWith((a) => a.insert({ body: 'On {branch}' }))
    expect(get().prefill).toBe('On {branch}')
    expect(toasts.list.some((t) => t.text.includes('{branch}'))).toBe(true)
  })

  it('refuses a busy input line and leaves the pane untouched', () => {
    const get = pane('claude', 'D:\\work\\app', { prefill: 'half typed by Orrery' })
    expect(insertWith((a) => a.insert({ body: 'x' }))).toBe('busy')
    expect(get().prefill).toBe('half typed by Orrery')
    const compact = pane('claude', 'D:\\work\\other', { compact: { step: 'write' } })
    expect(insertWith((a) => a.insert({ body: 'x' }))).toBe('busy')
    expect(compact().prefill).toBeUndefined()
  })

  it('does not type into a shell pane or when nothing is focused', () => {
    const sh = pane('shell', 'D:\\work\\app')
    expect(insertWith((a) => a.insert({ body: 'x' }))).toBe('notAgent')
    expect(sh().prefill).toBeUndefined()
    ws.activeKey = null as never
    expect(insertWith((a) => a.insert({ body: 'x' }))).toBe('noPane')
    expect(toasts.list.length).toBeGreaterThan(0)
  })

  it('opens the palette in ; mode only after the palette reset, once', async () => {
    const seen: string[] = []
    const { paletteSeed } = await import('../lib/paletteSeed')
    mount(
      defineComponent({
        setup() {
          paletteMod.usePalettePrompts(() => null, (q) => seen.push(q))
          return () => h('div')
        },
      }),
      { global: { plugins: [i18n()] } },
    )
    paletteSeed.value = true
    ui.paletteOpen = true
    await nextTick()
    expect(seen).toEqual([])
    await nextTick()
    await nextTick()
    await nextTick()
    expect(seen).toEqual([';'])
    ui.paletteOpen = false
    await nextTick()
    ui.paletteOpen = true
    await nextTick()
    await nextTick()
    await nextTick()
    expect(seen).toEqual([';'])
    expect(store.supported).toBe(false)
  })

  it('lists saved prompts for ; mode and runs one into the pane', async () => {
    const get = pane('claude', 'D:\\work\\app')
    store.items = [
      { id: 'a', title: 'Fix build', body: 'Fix the build on {branch}', scope: 'all', updatedAt: 1 },
      { id: 'b', title: 'Other', body: 'x', scope: 'D:\\work\\elsewhere', updatedAt: 1 },
    ]
    let items: ReturnType<typeof paletteMod.usePalettePrompts>['value'] = []
    const mode = ref<string | null>(null)
    mount(
      defineComponent({
        setup() {
          const list = paletteMod.usePalettePrompts(() => mode.value)
          return () => {
            items = list.value
            return h('div')
          }
        },
      }),
      { global: { plugins: [i18n()] } },
    )
    expect(items).toEqual([])
    mode.value = 'fix'
    await nextTick()
    expect(items.map((i) => i.id)).toEqual(['pr:a'])
    items[0]!.run()
    expect(get().prefill).toBe('Fix the build on {branch}')
  })
})
