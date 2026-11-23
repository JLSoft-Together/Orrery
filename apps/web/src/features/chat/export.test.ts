import { describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import en from '../../i18n/en'
import viMessages from '../../i18n/vi'
import ChatExportMenu from './ChatExportMenu.vue'
import { exportChat, paneChatTarget, runExport } from './export'
import type { ChatTarget } from './feed'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: viMessages } })
const target: ChatTarget = { key: 'p1', agent: 'claude', session: 'sid-1', cwd: 'C:\work\api' }
const reply = { ok: true, text: '# Session Claude: api\n', cut: false, name: 'claude-api.md', label: 'Claude', project: 'api' }

describe('chat export', () => {
  it('builds a target only for an agent pane that has a session', () => {
    expect(paneChatTarget({ key: 'k', preset: 'shell' }, undefined)).toBeNull()
    expect(paneChatTarget({ key: 'k', preset: 'claude' }, undefined)).toBeNull()
    expect(paneChatTarget({ key: 'k', preset: 'claude', agentSessionId: 's1', cwd: '/w' }, undefined)).toEqual({ key: 'k', agent: 'claude', session: 's1', cwd: '/w' })
    expect(paneChatTarget({ key: 'k', preset: 'codex', terminalId: 't' }, { id: 'codex:abc', cwd: '/c' })).toEqual({ key: 'k', agent: 'codex', session: 'abc', cwd: '/c' })
  })

  it('asks for the text with the form and returns null on a refusal', async () => {
    const call = vi.fn(async () => reply)
    expect(await exportChat(target, 'short', call)).toEqual({ text: reply.text, name: 'claude-api.md', cut: false })
    expect(call).toHaveBeenCalledWith({ op: 'export', key: 'p1', agent: 'claude', session: 'sid-1', cwd: 'C:\work\api', form: 'short', dest: 'text' })
    expect(await exportChat(target, 'full', async () => ({ ok: false, code: 'empty' }))).toBeNull()
    expect(await exportChat(target, 'full', async () => Promise.reject(new Error('x')))).toBeNull()
  })

  it('copies, saves and sends, and tells cancelled from failed', async () => {
    const write = vi.fn(async () => undefined)
    expect(await runExport(target, 'copy', { call: async () => reply, write })).toBe('copied')
    expect(write).toHaveBeenCalledWith(reply.text)
    const call = vi.fn(async (req: Record<string, unknown>) => ({ ok: true, saved: req.dest === 'save' ? false : undefined, sent: req.dest === 'telegram' }))
    expect(await runExport(target, 'save', { call })).toBe('cancelled')
    expect(await runExport(target, 'telegram', { call })).toBe('sent')
    expect(call).toHaveBeenLastCalledWith(expect.objectContaining({ op: 'export', dest: 'telegram', form: 'full' }))
    expect(await runExport(target, 'telegram', { call: async () => ({ ok: false, code: 'no-topic' }) })).toBe('failed')
    expect(await runExport(target, 'save', { call: async () => ({ ok: true, saved: true }) })).toBe('saved')
    expect(await runExport(target, 'copy', { call: async () => ({ ok: false }), write })).toBe('failed')
  })
})

describe('ChatExportMenu', () => {
  const mountMenu = (props: Record<string, unknown>) => mount(ChatExportMenu, { props: { target, ...props }, global: { plugins: [i18n, createPinia()] }, attachTo: document.body })

  it('lists copy and save, and Telegram only when it is set up', async () => {
    const off = mountMenu({ call: vi.fn(), telegram: false })
    await off.get('[data-chat-export-button]').trigger('click')
    expect(off.get('[data-chat-export-menu]').attributes('role')).toBe('menu')
    expect(off.find('[data-chat-export-copy]').exists()).toBe(true)
    expect(off.find('[data-chat-export-save]').exists()).toBe(true)
    expect(off.find('[data-chat-export-telegram]').exists()).toBe(false)
    off.unmount()
    const on = mountMenu({ call: vi.fn(), telegram: true })
    await on.get('[data-chat-export-button]').trigger('click')
    expect(on.find('[data-chat-export-telegram]').text()).toBe('Send to Telegram')
    on.unmount()
  })

  it('copies the Markdown, closes the menu and shows the result', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
    const call = vi.fn(async () => reply)
    const w = mountMenu({ call, telegram: false })
    await w.get('[data-chat-export-button]').trigger('click')
    await w.get('[data-chat-export-copy]').trigger('click')
    await flushPromises()
    expect(writeText).toHaveBeenCalledWith(reply.text)
    expect(w.find('[data-chat-export-menu]').exists()).toBe(false)
    expect(w.get('[data-chat-export-result]').attributes('data-outcome')).toBe('copied')
    expect(w.get('[data-chat-export-result]').text()).toBe('Copied')
    w.unmount()
  })

  it('shows a failure when the export is refused and closes on Escape', async () => {
    const w = mountMenu({ call: vi.fn(async () => ({ ok: false })), telegram: false })
    await w.get('[data-chat-export-button]').trigger('click')
    await w.get('[data-chat-export-menu]').trigger('keydown', { key: 'Escape' })
    expect(w.find('[data-chat-export-menu]').exists()).toBe(false)
    await w.get('[data-chat-export-button]').trigger('click')
    await w.get('[data-chat-export-save]').trigger('click')
    await flushPromises()
    expect(w.get('[data-chat-export-result]').attributes('data-outcome')).toBe('failed')
    w.unmount()
  })
})
