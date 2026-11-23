import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { desktop, type HooksState, type StatusLineState } from '../../../core/lib/desktop'
import { playChime } from '../../../core/lib/sound'
import { useConnectionStore } from '../../../app/stores/connection'
import { useExtrasStore } from '../../../core/stores/extras'
import { useSettingsStore } from '../../../core/stores/settings'

export type SetupItemId = 'notify' | 'toast' | 'sound' | 'statusLine' | 'hooks' | 'folder'

export const SETUP_ORDER: SetupItemId[] = ['notify', 'toast', 'sound', 'statusLine', 'hooks', 'folder']
export const SETUP_COUNTED: readonly SetupItemId[] = ['notify', 'toast', 'statusLine', 'hooks']

const DONE_KEY = 'ccm.setupDone'
const HIDDEN_KEY = 'ccm.setupChip.hidden'

export interface SetupItemState {
  id: SetupItemId
  available: boolean
  on: boolean
}

export function pendingSetup(items: readonly SetupItemState[]): SetupItemId[] {
  return items.filter((i) => i.available && !i.on && SETUP_COUNTED.includes(i.id)).map((i) => i.id)
}

function readFlag(key: string): boolean {
  try {
    return localStorage.getItem(key) === '1'
  } catch {
    return false
  }
}

function writeFlag(key: string, on: boolean): void {
  try {
    if (on) localStorage.setItem(key, '1')
    else localStorage.removeItem(key)
  } catch {
    return
  }
}

export const useSetupChecklistStore = defineStore('setupChecklist', () => {
  const settings = useSettingsStore()
  const extras = useExtrasStore()
  const connection = useConnectionStore()

  const statusLine = ref<StatusLineState | null>(null)
  const statusLineNote = ref<'failed' | null>(null)
  const hooks = ref<HooksState | null>(null)
  const hooksNote = ref<'failed' | null>(null)
  const folder = ref<string | null>(null)
  const busy = ref<SetupItemId | null>(null)
  const hidden = ref(readFlag(HIDDEN_KEY) || readFlag(DONE_KEY))
  let loaded = false

  const available = (id: SetupItemId): boolean => {
    if (id === 'notify') return settings.notifyPermission !== 'unsupported'
    if (id === 'toast') return extras.toast === 'on' || extras.toast === 'off'
    if (id === 'statusLine') return !!desktop?.enableStatusLine && statusLine.value !== null
    if (id === 'hooks') return !!desktop?.setHooks && hooks.value !== null
    if (id === 'folder') return !!desktop?.grantFolder
    return true
  }

  const isOn = (id: SetupItemId): boolean => {
    if (id === 'notify') return settings.notifyEnabled
    if (id === 'toast') return extras.toast === 'on'
    if (id === 'sound') return settings.sound
    if (id === 'statusLine') return statusLine.value === 'ours'
    if (id === 'hooks') return hooks.value === 'ours'
    return folder.value !== null
  }

  const items = computed<SetupItemState[]>(() => SETUP_ORDER.map((id) => ({ id, available: available(id), on: isOn(id) })))
  const pending = computed(() => (desktop ? pendingSetup(items.value) : []))
  const chipVisible = computed(() => pending.value.length > 0 && !hidden.value)

  async function refresh(): Promise<void> {
    statusLine.value = (await desktop?.statusLineState?.().catch(() => null)) ?? null
    hooks.value = (await desktop?.hooksState?.().catch(() => null)) ?? null
  }

  function ensureLoaded(): void {
    if (loaded || !desktop) return
    loaded = true
    void refresh()
  }

  async function setHooks(on: boolean): Promise<void> {
    hooksNote.value = null
    const res = await desktop?.setHooks?.(on)
    if (res === 'ok') hooks.value = on ? 'ours' : 'none'
    else if (res === 'unreadable') hooks.value = 'unreadable'
    else if (res === 'failed') hooksNote.value = 'failed'
  }

  async function enable(id: SetupItemId): Promise<void> {
    if (busy.value) return
    busy.value = id
    try {
      if (id === 'notify') await settings.toggleNotifications()
      else if (id === 'toast') connection.updateNotify({ toast: true })
      else if (id === 'sound') {
        settings.sound = true
        playChime('waiting')
      } else if (id === 'statusLine') {
        statusLineNote.value = null
        const res = await desktop?.enableStatusLine?.()
        if (res === 'ok' || res === 'ours') statusLine.value = 'ours'
        else if (res === 'unreadable') statusLine.value = 'unreadable'
        else if (res === 'failed') statusLineNote.value = 'failed'
      } else if (id === 'hooks') await setHooks(true)
      else if (id === 'folder') {
        const picked = await desktop?.grantFolder?.()
        if (picked) folder.value = picked
      }
    } finally {
      busy.value = null
    }
  }

  async function disableHooks(): Promise<void> {
    if (busy.value) return
    busy.value = 'hooks'
    try {
      await setHooks(false)
    } finally {
      busy.value = null
    }
  }

  function hideChip(): void {
    hidden.value = true
    writeFlag(HIDDEN_KEY, true)
  }

  function showChip(): void {
    hidden.value = false
    writeFlag(HIDDEN_KEY, false)
    writeFlag(DONE_KEY, false)
  }

  return { statusLine, statusLineNote, hooks, hooksNote, folder, busy, hidden, items, pending, chipVisible, available, isOn, ensureLoaded, refresh, enable, disableHooks, hideChip, showChip }
})
