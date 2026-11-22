import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import type { Component } from 'vue'
import en from '../../../i18n/en'
import vi_ from '../../../i18n/vi'
import { useExtrasStore } from '../../../core/stores/extras'
import { useUiGenStore } from '../../next/stores/uiGen'
import { SETTINGS_PAGES, entries, loadPage, nextEntries, pageAvailable, pageVisible, searchSettings, type SettingsEntry, type SettingsEnv, type SettingsPage } from './index'

vi.mock('../../../core/lib/desktop', async (orig) => ({ ...(await orig<typeof import('../../../core/lib/desktop')>()), hasTerminals: true }))
vi.mock('../../disk/lib/diskMap', async (orig) => ({
  ...(await orig<typeof import('../../disk/lib/diskMap')>()),
  hasDiskMap: true,
  diskMapStatus: async () => ({ scanning: false, paused: false, indexed: true, roots: [], entries: 0 }),
  diskMapVolumes: async () => [],
  diskMapHistory: async () => [],
  diskMapChildren: async () => null,
  onDiskMapProgress: () => () => {},
  onDiskMapDone: () => () => {},
}))
vi.mock('../../disk/lib/diskSnap', async (orig) => ({
  ...(await orig<typeof import('../../disk/lib/diskSnap')>()),
  hasDiskSnap: true,
  hasDiskSchedule: true,
  diskSnapList: async () => [],
  diskScheduleGet: async () => null,
  diskScheduleSet: async () => null,
  onDiskSnapNew: () => () => {},
  onDiskSchedule: () => () => {},
}))

import SettingsAbout from './SettingsAbout.vue'
import SettingsAppearance from './SettingsAppearance.vue'
import SettingsGeneral from './SettingsGeneral.vue'
import PageData from './PageData.vue'
import PageIntegrations from './PageIntegrations.vue'
import PageNotify from './PageNotify.vue'
import PageWorkspace from './PageWorkspace.vue'

const PAGES: Partial<Record<SettingsPage, Component>> = {
  general: SettingsGeneral,
  appearance: SettingsAppearance,
  notify: PageNotify,
  integrations: PageIntegrations,
  data: PageData,
  about: SettingsAbout,
  workspace: PageWorkspace,
}

const MOUNTABLE = new Set<keyof SettingsEnv>(['terminals', 'toast', 'notifyData'])

const env: SettingsEnv = {
  terminals: true,
  mac: false,
  loginItem: false,
  zoom: false,
  awake: false,
  github: false,
  run: false,
  capture: false,
  agentData: false,
  phone: false,
  telegram: false,
  remote: false,
  disk: false,
  updater: false,
  backup: false,
  grants: false,
  statusLine: false,
  toast: true,
  notifyData: true,
  deepseek: false,
  xmlPreview: false,
  bugReport: false,
  desktop: false,
}

const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages: { en, vi: vi_ }, missingWarn: false, fallbackWarn: false })
let wrapper: VueWrapper | null = null

async function rowsOf(page: SettingsPage, next: boolean): Promise<string[]> {
  const pinia = createPinia()
  setActivePinia(pinia)
  useUiGenStore().setGeneration(next ? 'next' : 'classic')
  const extras = useExtrasStore()
  extras.notify = { toast: 'on', kinds: {}, quiet: { enabled: false, from: '22:00', to: '07:00' }, idleMinutes: 0, health: 'ok' } as never
  wrapper = mount(PAGES[page]!, { global: { plugins: [pinia, i18n] }, attachTo: document.body })
  await flushPromises()
  return [...document.body.querySelectorAll<HTMLElement>('[data-setting-row]')].map((el) => el.dataset.settingRow ?? '')
}

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
  document.body.innerHTML = ''
})

describe('every Settings entry has a row on its page', () => {
  beforeEach(() => localStorage.clear())

  for (const next of [false, true]) {
    const list: SettingsEntry[] = next ? nextEntries(false) : entries(false)
    const pages = [...new Set(list.map((e) => e.page))].filter((p) => PAGES[p] && pageAvailable(p, env, next))
    for (const page of pages) {
      it(`${next ? 'next' : 'classic'}: ${page}`, async () => {
        const rows = await rowsOf(page, next)
        const wanted = list.filter((e) => e.page === page && (!e.needs || MOUNTABLE.has(e.needs)))
        const missing = wanted.filter((e) => !rows.includes(e.id)).map((e) => e.id)
        expect(missing).toEqual([])
        for (const e of wanted) expect(rows.filter((r) => r === e.id).length).toBe(1)
      })
    }

    it(`${next ? 'next' : 'classic'}: no entry points at a page that is hidden`, () => {
      const hidden = list.filter((e) => !pageAvailable(e.page, env, next) && pageVisible(e.page, env) && !e.needs)
      expect(hidden.map((e) => `${e.page}:${e.id}`)).toEqual([])
    })
  }

  it('search finds file icons, dust and reminders on their pages', () => {
    const t = (k: string) => i18n.global.t(k)
    for (const next of [false, true]) {
      const list = next ? nextEntries(false) : entries(false)
      const page = (q: string, id: string) => searchSettings(q, list, env, t).find((h) => h.id === id)?.page
      expect(page('file icons', 'fileIcons')).toBe('workspace')
      expect(page('dust', 'dust')).toBe('workspace')
      expect(page('break', 'break')).toBe('notify')
      expect(page('budget', 'budget')).toBe('notify')
    }
  })

  it('has no reminders page and maps the old stored key to notify', () => {
    expect(SETTINGS_PAGES as readonly string[]).not.toContain('reminders')
    localStorage.setItem('ccm.settings.page', 'reminders')
    expect(loadPage()).toBe('notify')
  })
})
