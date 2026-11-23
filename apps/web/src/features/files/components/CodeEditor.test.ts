import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { nextTick } from 'vue'
import { EditorView } from '@codemirror/view'
import { i18n } from '../../../i18n'
import CodeEditor from './CodeEditor.vue'

interface Api {
  text: () => string
  markSaved: (saved?: string) => void
  setText: (text: string) => void
}

const editorOf = (props: { doc: string; base?: string }) =>
  mount(CodeEditor, { props: { grammar: null, ...props }, global: { plugins: [i18n, createPinia()] }, attachTo: document.body })

describe('CodeEditor', () => {
  it('reports dirty against the disk text, clean again after markSaved and setText', async () => {
    const w = editorOf({ doc: 'hello' })
    const api = w.vm as unknown as Api
    expect(api.text()).toBe('hello')
    const view = EditorView.findFromDOM(w.element.querySelector('.cm-editor') as HTMLElement)!
    view.dispatch({ changes: { from: 5, insert: '!' } })
    await nextTick()
    expect(w.emitted('dirty')?.at(-1)).toEqual([true])
    expect(api.text()).toBe('hello!')
    api.markSaved('hello!')
    expect(w.emitted('dirty')?.at(-1)).toEqual([false])
    view.dispatch({ changes: { from: 0, insert: '>' } })
    expect(w.emitted('dirty')?.at(-1)).toEqual([true])
    api.setText('from disk')
    expect(api.text()).toBe('from disk')
    expect(w.emitted('dirty')?.at(-1)).toEqual([false])
    w.unmount()
  })

  it('shows a swatch before a colour literal and rewrites it from the picker', async () => {
    const w = mount(CodeEditor, { props: { grammar: 'css', doc: 'a { color: #f008 }' }, global: { plugins: [i18n, createPinia()] }, attachTo: document.body })
    const swatch = (w.element as HTMLElement).querySelector<HTMLElement>('.cm-color-swatch')
    expect(swatch).not.toBeNull()
    swatch!.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
    const input = document.body.querySelector<HTMLInputElement>('input[type=color]')!
    expect(input.value).toBe('#ff0000')
    input.value = '#00ff00'
    input.dispatchEvent(new Event('input'))
    expect((w.vm as unknown as Api).text()).toBe('a { color: #0f08 }')
    w.unmount()
    expect(document.body.querySelector('input[type=color]')).toBeNull()
  })

  it('starts dirty when re-entered with unsaved text (base = disk text)', () => {
    const w = editorOf({ doc: 'mine', base: 'disk' })
    expect(w.emitted('dirty')?.at(-1)).toEqual([true])
    w.unmount()
  })

  it('exposes the selected lines (a selection ending at column 0 stops on the line before) and sends on Ctrl+Shift+L', async () => {
    const w = editorOf({ doc: 'one\ntwo\nthree\nfour' })
    const api = w.vm as unknown as { selection: () => { from: number; to: number; text: string } | null }
    const view = EditorView.findFromDOM(w.element.querySelector('.cm-editor') as HTMLElement)!
    expect(api.selection()).toBeNull()
    const press = () => view.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key: 'L', code: 'KeyL', keyCode: 76, ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))
    press()
    expect(w.emitted('send')).toBeUndefined()
    view.dispatch({ selection: { anchor: 0, head: 9 } })
    expect(api.selection()).toEqual({ from: 1, to: 3, text: 'one\ntwo\nt' })
    view.dispatch({ selection: { anchor: 4, head: 8 } })
    expect(api.selection()).toEqual({ from: 2, to: 2, text: 'two\n' })
    press()
    expect(w.emitted('send')).toHaveLength(1)
    w.unmount()
  })

  it('opens the send menu on right click only when something is selected', async () => {
    const w = editorOf({ doc: 'one\ntwo' })
    const view = EditorView.findFromDOM(w.element.querySelector('.cm-editor') as HTMLElement)!
    const host = w.element.querySelector('[data-file-editor]') as HTMLElement
    const none = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 5, clientY: 6 })
    host.dispatchEvent(none)
    expect(none.defaultPrevented).toBe(false)
    expect(w.emitted('menu')).toBeUndefined()
    view.dispatch({ selection: { anchor: 0, head: 3 } })
    const some = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 5, clientY: 6 })
    host.dispatchEvent(some)
    expect(some.defaultPrevented).toBe(true)
    expect(w.emitted('menu')?.[0]).toEqual([{ left: 5, right: 5, top: 6, bottom: 6 }])
    w.unmount()
  })
})
