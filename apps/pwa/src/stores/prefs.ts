import { defineStore } from 'pinia'
import { ref } from 'vue'
import { Emitter } from '../remote/emitter'
import type { Db } from '../storage/idb'
import { clearLocalPrefs, DEFAULT_LOCAL_PREFS, loadLocalPrefs, saveLocalPrefs, type LanguagePref, type LocalPrefs, type MonitorMode, type ThemePref, type WorkspaceMode } from '../storage/localPrefs'
import { bellOn, DEFAULT_NOTIFY, loadNotify, saveNotify, type NotifyPrefs } from '../storage/notifyPrefs'

export type NotifyKey = 'waiting' | 'done' | 'error' | 'content'

export type PrefEvent = { kind: 'bell'; machineId: string; on: boolean } | { kind: 'notify'; key: NotifyKey; on: boolean }

export const usePrefsStore = defineStore('prefs', () => {
  const local = ref<LocalPrefs>(loadLocalPrefs())
  const notify = ref<NotifyPrefs>({ ...DEFAULT_NOTIFY, bells: {} })
  const events = new Emitter<PrefEvent>()
  let db: Db | null = null

  function patchLocal(patch: Partial<LocalPrefs>): void {
    local.value = { ...local.value, ...patch }
    saveLocalPrefs(local.value)
  }

  async function attach(next: Db): Promise<void> {
    db = next
    notify.value = await loadNotify(next)
  }

  async function persistNotify(): Promise<void> {
    if (db) await saveNotify(db, notify.value)
  }

  function setLanguage(language: LanguagePref): void {
    patchLocal({ language })
  }

  function setTheme(theme: ThemePref): void {
    patchLocal({ theme })
  }

  function setMonitorMode(monitorMode: MonitorMode): void {
    patchLocal({ monitorMode })
  }

  function setWorkspaceMode(workspaceMode: WorkspaceMode): void {
    patchLocal({ workspaceMode })
  }

  function setLastMachine(id: string | null): void {
    if (local.value.lastMachine !== id) patchLocal({ lastMachine: id })
  }

  async function setNotify(key: NotifyKey, on: boolean): Promise<void> {
    notify.value = { ...notify.value, [key]: on }
    events.emit({ kind: 'notify', key, on })
    await persistNotify()
  }

  function bell(machineId: string): boolean {
    return bellOn(notify.value, machineId)
  }

  async function setBell(machineId: string, on: boolean): Promise<void> {
    notify.value = { ...notify.value, bells: { ...notify.value.bells, [machineId]: on } }
    events.emit({ kind: 'bell', machineId, on })
    await persistNotify()
  }

  async function dropMachine(machineId: string): Promise<void> {
    if (!(machineId in notify.value.bells)) return
    const bells = { ...notify.value.bells }
    delete bells[machineId]
    notify.value = { ...notify.value, bells }
    await persistNotify()
  }

  function resetAll(): void {
    clearLocalPrefs()
    local.value = { ...DEFAULT_LOCAL_PREFS }
    notify.value = { ...DEFAULT_NOTIFY, bells: {} }
  }

  return { local, notify, events, patchLocal, attach, setLanguage, setTheme, setMonitorMode, setWorkspaceMode, setLastMachine, setNotify, bell, setBell, dropMachine, resetAll }
})
