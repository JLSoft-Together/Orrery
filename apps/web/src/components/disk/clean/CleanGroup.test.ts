import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import type { CleanCategory, CleanItem } from '@ccm/shared'
import messages from '../../../i18n/disk-clean-v101'
import CleanGroup from './CleanGroup.vue'

const i18n = createI18n({ legacy: false, locale: 'vi', fallbackLocale: 'en', messages: { en: messages.en, vi: messages.vi } })
const DAY = 86_400_000
const NOW = 1_800_000_000_000
const cat: CleanCategory = { id: 'build', group: 'dev', preselect: true, titleKey: 'diskClean.cat.build.title', hintKey: 'diskClean.cat.build.hint' }
const items: CleanItem[] = [
  { path: 'C:\\old\\build', size: 2048, group: 'dev', category: 'build', project: 'C:\\old', lastTouched: NOW - 60 * DAY, restore: './gradlew build' },
  { path: 'C:\\busy\\build', size: 1024, group: 'dev', category: 'build', project: 'C:\\busy', lastTouched: NOW - 90 * DAY },
]

function make(selected: string[]) {
  return mount(CleanGroup, {
    props: { category: cat, items, selected: new Set(selected), open: ['C:\\busy\\src'], days: 30, now: NOW, busy: false },
    global: { plugins: [i18n] },
  })
}

const row = (w: ReturnType<typeof make>, p: string) => w.findAll('[data-clean-row]').find((x) => x.attributes('data-clean-row') === p)!

describe('CleanGroup', () => {
  it('shows a locked row with a reason and a disabled checkbox', () => {
    const w = make(['C:\\busy\\build'])
    const busy = row(w, 'C:\\busy\\build')
    expect(busy.attributes('data-clean-locked')).toBe('open')
    const box = busy.find<HTMLInputElement>('[data-clean-check]')
    expect(box.element.disabled).toBe(true)
    expect(box.element.checked).toBe(false)
    expect(busy.text()).toContain('Đang khoá')
    expect(busy.find('[data-clean-lock]').attributes('title')).toBe('Project đang mở trong Workspace hoặc có session')
  })

  it('shows age, restore command and total; header box selects only free rows', async () => {
    const w = make([])
    const old = row(w, 'C:\\old\\build')
    expect(old.find('[data-clean-age]').text()).toBe('60 ngày trước')
    expect(old.find('[data-clean-restore]').text()).toContain('./gradlew build')
    expect(w.text()).toContain('Đồ build')
    await w.find('[data-clean-all]').trigger('change')
    expect(w.emitted('all')?.[0]).toEqual(['build'])
    await old.find('[data-clean-check]').trigger('change')
    expect(w.emitted('toggle')?.[0]).toEqual(['C:\\old\\build'])
  })

  it('header box is checked when every free row is picked', () => {
    const w = make(['C:\\old\\build'])
    expect(w.find<HTMLInputElement>('[data-clean-all]').element.checked).toBe(true)
  })
})
