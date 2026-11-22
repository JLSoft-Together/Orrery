import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '../../i18n/en'
import viMessages from '../../i18n/vi'
import type { SecretBlock } from '../../lib/preview'
import SecretGuardDialog from './SecretGuardDialog.vue'
import { groupHits, isEmptyBlock, isSecretBlock, kindKey, laterNotice, withoutPath } from './secretGuard'

const preview = vi.hoisted(() => ({
  gitCommit: vi.fn(),
  gitUnstage: vi.fn(),
}))
vi.mock('../../lib/preview', async (orig) => ({ ...(await orig<typeof import('../../lib/preview')>()), gitCommit: preview.gitCommit, gitUnstage: preview.gitUnstage }))

const { default: CommitBox } = await import('../../components/workspace/CommitBox.vue')
const { useSettingsStore } = await import('../../stores/settings')

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, vi: viMessages } })

const block: SecretBlock = {
  ok: false,
  error: 'secrets',
  hits: [
    { path: 'src/api.ts', line: 12, kind: 'github', masked: 'ghp_••••' },
    { path: '.env', line: 3, kind: 'env', masked: 'STRIPE_SECRET=••••' },
    { path: 'src/api.ts', line: 4, kind: 'mystery', masked: 'x=••••' },
  ],
}

const status = {
  ok: true as const,
  root: 'C:/repo',
  branch: 'main',
  upstream: null,
  ahead: 0,
  behind: 0,
  truncated: false,
  entries: [
    { path: '.env', x: 'A', y: ' ', kind: 'added', staged: true },
    { path: 'src/api.ts', x: 'M', y: ' ', kind: 'modified', staged: true },
  ],
}

describe('secretGuard helpers', () => {
  it('recognises a secret block and maps kinds to labels', () => {
    expect(isSecretBlock(block)).toBe(true)
    expect(isSecretBlock({ ok: false, error: 'failed' })).toBe(false)
    expect(isSecretBlock(null)).toBe(false)
    expect(kindKey('aws')).toBe('secretGuard.kind.aws')
    expect(kindKey('mystery')).toBe('secretGuard.kind.other')
  })

  it('groups hits per file sorted by line and drops a removed file', () => {
    const files = groupHits(block.hits)
    expect(files.map((f) => f.path)).toEqual(['src/api.ts', '.env'])
    expect(files[0]!.hits.map((h) => h.line)).toEqual([4, 12])
    const rest = withoutPath(block, 'src/api.ts')
    expect(rest.hits).toHaveLength(1)
    expect(isEmptyBlock(withoutPath(rest, '.env'))).toBe(true)
    expect(isEmptyBlock({ ...block, hits: [], scanError: 'timeout' })).toBe(false)
  })

  it('shows the scanning notice only after the delay', () => {
    vi.useFakeTimers()
    const cb = vi.fn()
    laterNotice(cb, 300)
    vi.advanceTimersByTime(299)
    expect(cb).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(cb).toHaveBeenCalledOnce()
    const cb2 = vi.fn()
    const stop = laterNotice(cb2, 300)
    stop()
    vi.advanceTimersByTime(400)
    expect(cb2).not.toHaveBeenCalled()
    vi.useRealTimers()
  })
})

describe('SecretGuardDialog', () => {
  it('lists file:line, kind and masked value, never more', () => {
    const w = mount(SecretGuardDialog, { props: { block, mode: 'commit' }, global: { plugins: [i18n] }, attachTo: document.body })
    const where = [...document.querySelectorAll('[data-secret-where]')].map((n) => n.textContent?.trim())
    expect(where).toEqual(['api.ts:4', 'api.ts:12', '.env:3'])
    expect(document.querySelector('[data-secret-title]')?.textContent).toContain('3 possible secrets')
    expect(document.body.textContent).toContain('GitHub token')
    expect(document.body.textContent).toContain('STRIPE_SECRET=••••')
    expect(document.querySelectorAll('[data-secret-unstage]')).toHaveLength(2)
    w.unmount()
  })

  it('push mode has no unstage buttons and reports a scan error', () => {
    const w = mount(SecretGuardDialog, { props: { block: { ...block, hits: [], scanError: 'timeout' }, mode: 'push' }, global: { plugins: [i18n] }, attachTo: document.body })
    expect(document.querySelectorAll('[data-secret-unstage]')).toHaveLength(0)
    expect(document.body.textContent).toContain('did not finish')
    expect(document.querySelector('[data-secret-proceed]')?.textContent).toContain('Push anyway')
    w.unmount()
  })
})

describe('CommitBox secret guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    preview.gitCommit.mockReset()
    preview.gitUnstage.mockReset()
    document.body.innerHTML = ''
  })

  const mountBox = () => mount(CommitBox, { props: { status: status as never }, global: { plugins: [i18n] }, attachTo: document.body })

  it('asks Rust to scan, holds the commit, unstages a file and commits anyway on an explicit click', async () => {
    preview.gitCommit.mockResolvedValueOnce(block).mockResolvedValueOnce({ ok: true, hash: 'abcdef1234', subject: 'x' })
    preview.gitUnstage.mockResolvedValue({ ok: true })
    const w = mountBox()
    await w.find('textarea').setValue('fix api')
    await w.find('[data-commit-submit]').trigger('click')
    await flushPromises()
    expect(preview.gitCommit).toHaveBeenCalledWith('C:/repo', 'fix api', false, { scan: true })
    expect(document.querySelector('[data-secret-guard]')).not.toBeNull()
    ;(document.querySelector('[data-secret-file][data-path=".env"] [data-secret-unstage]') as HTMLButtonElement).click()
    await flushPromises()
    expect(preview.gitUnstage).toHaveBeenCalledWith('C:/repo', ['.env'])
    expect(document.querySelectorAll('[data-secret-file]')).toHaveLength(1)
    ;(document.querySelector('[data-secret-proceed]') as HTMLButtonElement).click()
    await flushPromises()
    expect(preview.gitCommit).toHaveBeenLastCalledWith('C:/repo', 'fix api', false, { scan: false })
    expect(document.querySelector('[data-secret-guard]')).toBeNull()
    w.unmount()
  })

  it('cancel keeps the message and does not commit; setting off skips the scan', async () => {
    preview.gitCommit.mockResolvedValueOnce(block)
    const w = mountBox()
    await w.find('textarea').setValue('fix api')
    await w.find('[data-commit-submit]').trigger('click')
    await flushPromises()
    ;(document.querySelector('[data-secret-cancel]') as HTMLButtonElement).click()
    await flushPromises()
    expect(document.querySelector('[data-secret-guard]')).toBeNull()
    expect((w.find('textarea').element as HTMLTextAreaElement).value).toBe('fix api')
    expect(preview.gitCommit).toHaveBeenCalledOnce()
    useSettingsStore().secretGuard = false
    preview.gitCommit.mockResolvedValueOnce({ ok: true, hash: 'abcdef1234', subject: 'x' })
    await w.find('[data-commit-submit]').trigger('click')
    await flushPromises()
    expect(preview.gitCommit).toHaveBeenLastCalledWith('C:/repo', 'fix api', false, { scan: false })
    w.unmount()
  })

  it('closes the warning when every flagged file is unstaged', async () => {
    preview.gitCommit.mockResolvedValueOnce({ ...block, hits: [block.hits[1]!] })
    preview.gitUnstage.mockResolvedValue({ ok: true })
    const w = mountBox()
    await w.find('textarea').setValue('fix api')
    await w.find('[data-commit-submit]').trigger('click')
    await flushPromises()
    ;(document.querySelector('[data-secret-unstage]') as HTMLButtonElement).click()
    await flushPromises()
    expect(document.querySelector('[data-secret-guard]')).toBeNull()
    expect(w.find('[data-commit-notice]').text()).toContain('Press Commit again')
    w.unmount()
  })
})
