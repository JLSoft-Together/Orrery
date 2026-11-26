import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Undo2 } from 'lucide-vue-next'
import { hashSerial, installGet, installUseSuggested, remoteCheck, remoteRevoke, remoteRevokeUndo, remoteStatus, useRemoteStore } from '@features/remote'
import { useToastsStore } from '../../../core/stores/toasts'
import { useUiStore } from '../../../core/stores/ui'
import { devInstall, devLaunch, devPkgVersion, hasDeviceInstall, type DevPkgVersion } from '../lib/devices'
import { rowAction, type RowAction } from '../lib/deviceActions'
import { choose, countFilters, forgetChoice, loadChoices, matchesFilter, mergeDevices, REMOTE_PACKAGE, saveChoices, type DeviceRow, type RowFilter } from '../lib/mergeDevices'
import { useDevicesStore } from '../stores/devices'
import { useEmulatorsStore } from '../stores/emulators'

export const REVOKE_GRACE_S = 5
const APP_URI = 'orrery-remote://m'

export interface LinkMs {
  lan?: number
  relay?: number
}

type Obj = Record<string, unknown>
const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null

export function parseCheck(v: unknown): { latency: LinkMs; push: 'ok' | 'rate' | 'none' | 'fail' } | null {
  if (!isObj(v)) return null
  const lan = isObj(v.lan) && v.lan.ok === true && typeof v.lan.ms === 'number' ? Math.round(v.lan.ms) : undefined
  const relay = isObj(v.relay) && typeof v.relay.ms === 'number' ? Math.round(v.relay.ms) : undefined
  const p = isObj(v.push) ? v.push : {}
  const push = p.ok === true ? 'ok' : p.error === 'rate' ? 'rate' : p.error === 'none' ? 'none' : 'fail'
  return { latency: { lan, relay }, push }
}

export function useSettingsDevices() {
  const { t } = useI18n()
  const remote = useRemoteStore()
  const adb = useDevicesStore()
  const emulators = useEmulatorsStore()
  const toasts = useToastsStore()
  const ui = useUiStore()

  const filter = ref<RowFilter>('all')
  const openKey = ref('')
  const hashes = ref<Record<string, string>>({})
  const pkg = ref<Record<string, DevPkgVersion | undefined>>({})
  const desktopVersion = ref('')
  const choices = ref(loadChoices())
  const busy = ref<Record<string, RowAction | 'test' | undefined>>({})
  const latency = ref<Record<string, LinkMs | undefined>>({})
  let gone = false

  const merged = computed(() => mergeDevices({ remote: remote.devices, adb: adb.devices, avds: emulators.avds, hashes: hashes.value, choices: choices.value }))
  const rows = computed(() => merged.value.rows)
  const shown = computed(() => rows.value.filter((r) => matchesFilter(r, filter.value)))
  const counts = computed(() => countFilters(rows.value))
  const ctx = computed(() => ({ desktopVersion: desktopVersion.value, pkg: pkg.value, canInstall: hasDeviceInstall }))
  const actionOf = (row: DeviceRow): RowAction | null => (busy.value[row.key] ? null : rowAction(row, ctx.value))
  const hasAdb = computed(() => adb.adbFound)

  const setBusy = (key: string, what: RowAction | 'test' | undefined): void => {
    const next = { ...busy.value }
    if (what) next[key] = what
    else delete next[key]
    busy.value = next
  }

  const say = (text: string, kind: 'info' | 'warning' = 'info'): void => void toasts.push({ id: 'devices-note', kind, text, timeoutMs: kind === 'warning' ? 8000 : 4000 })

  async function syncHashes(): Promise<void> {
    const missing = adb.devices.filter((d) => !hashes.value[d.id])
    if (!missing.length) return
    const entries = await Promise.all(missing.map(async (d) => [d.id, await hashSerial(d.id).catch(() => '')] as const))
    if (gone) return
    hashes.value = { ...hashes.value, ...Object.fromEntries(entries.filter(([, h]) => h)) }
  }

  async function loadPkg(ids: string[]): Promise<void> {
    const out = await Promise.all(ids.map(async (id) => [id, await devPkgVersion(id, REMOTE_PACKAGE)] as const))
    if (gone) return
    pkg.value = { ...pkg.value, ...Object.fromEntries(out) }
  }

  async function syncPkg(): Promise<void> {
    const online = adb.devices.filter((d) => d.online && d.state === 'ready').map((d) => d.id)
    const kept = Object.fromEntries(Object.entries(pkg.value).filter(([id]) => online.includes(id)))
    if (Object.keys(kept).length !== Object.keys(pkg.value).length) pkg.value = kept
    const need = online.filter((id) => !kept[id])
    if (need.length && hasDeviceInstall) await loadPkg(need)
  }

  async function readDesktopVersion(): Promise<void> {
    let out = await installGet()
    if (out.ok && !out.value.apk && out.value.suggested) out = await installUseSuggested()
    if (!gone && out.ok) desktopVersion.value = out.value.version
  }

  async function apkPath(): Promise<string | null> {
    let out = await installGet()
    if (out.ok && !out.value.apk && out.value.suggested) out = await installUseSuggested()
    return out.ok ? (out.value.apk?.path ?? null) : null
  }

  const errorText = (code: string): string => {
    const known = ['restricted', 'storage', 'signature', 'downgrade', 'abi', 'offline', 'unauthorized', 'timeout']
    return t(`v113Devices.err.${known.includes(code) ? code : 'other'}`)
  }

  async function install(row: DeviceRow, kind: 'install' | 'update'): Promise<void> {
    if (!row.adb || busy.value[row.key]) return
    setBusy(row.key, kind)
    try {
      const path = await apkPath()
      if (!path) return say(t('v113Devices.err.apk'), 'warning')
      const r = await devInstall(row.adb.id, path)
      if (!r.ok) return say(`${t(kind === 'update' ? 'v113Devices.toast.updateFail' : 'v113Devices.toast.installFail', { name: row.name })} ${errorText(r.error)}`, 'warning')
      await loadPkg([row.adb.id])
      say(kind === 'update' ? t('v113Devices.toast.updated', { name: row.name, version: desktopVersion.value }) : t('v113Devices.toast.installed', { name: row.name }))
    } finally {
      setBusy(row.key, undefined)
    }
  }

  async function boot(row: DeviceRow): Promise<void> {
    if (!row.avd || busy.value[row.key]) return
    setBusy(row.key, 'boot')
    try {
      await emulators.start(row.avd.name, 'normal')
      if (emulators.lastError) say(t('v113Devices.toast.bootFail', { name: row.name }), 'warning')
      else say(t('v113Devices.toast.booting', { name: row.name }))
    } finally {
      setBusy(row.key, undefined)
    }
  }

  const run = (row: DeviceRow, action: RowAction): Promise<void> => (action === 'boot' ? boot(row) : action === 'blocked' ? Promise.resolve() : install(row, action))

  async function sendTest(row: DeviceRow): Promise<void> {
    const id = row.remote?.deviceId
    if (!id || busy.value[row.key]) return
    setBusy(row.key, 'test')
    try {
      const out = parseCheck(await remoteCheck(id))
      if (!out) return say(t('v113Devices.toast.testFail', { name: row.name }), 'warning')
      latency.value = { ...latency.value, [id]: out.latency }
      if (out.push === 'ok') say(t('v113Devices.toast.testSent', { name: row.name }))
      else say(t(`v113Devices.toast.test_${out.push}`, { name: row.name }), 'warning')
    } finally {
      setBusy(row.key, undefined)
    }
  }

  async function openApp(row: DeviceRow): Promise<void> {
    if (!row.adb) return
    const r = await devLaunch(row.adb.id, { kind: 'uri', uri: APP_URI, pkg: REMOTE_PACKAGE })
    say(r.ok ? t('v113Devices.toast.opened', { name: row.name }) : t('v113Devices.toast.openFail', { name: row.name }), r.ok ? 'info' : 'warning')
  }

  const pushTo = async (row: DeviceRow, on: boolean): Promise<void> => void (row.remote && (await remote.setDevicePush(row.remote.deviceId, on)))

  async function rename(row: DeviceRow, name: string): Promise<void> {
    if (row.remote) await remote.renameDevice(row.remote.deviceId, name)
    if (row.adb) await adb.setAlias(row.adb.id, name || null)
  }

  async function refreshStatus(): Promise<void> {
    const next = await remoteStatus()
    if (next) remote.status = next
  }

  async function undoRevoke(row: DeviceRow): Promise<void> {
    const id = row.remote?.deviceId
    if (!id) return
    const ok = await remoteRevokeUndo(id)
    await refreshStatus()
    toasts.remove(`devices-revoke-${id}`)
    say(ok ? t('v113Devices.toast.restored', { name: row.name }) : t('v113Devices.toast.undoLate', { name: row.name }), ok ? 'info' : 'warning')
  }

  async function revoke(row: DeviceRow): Promise<void> {
    const id = row.remote?.deviceId
    if (!id) return
    const ok = await remoteRevoke(id, REVOKE_GRACE_S)
    await refreshStatus()
    if (!ok) return say(t('v113Devices.toast.revokeFail', { name: row.name }), 'warning')
    if (openKey.value === row.key) openKey.value = ''
    toasts.push({
      id: `devices-revoke-${id}`,
      kind: 'undo',
      text: t('v113Devices.toast.revoked', { name: row.name }),
      timeoutMs: REVOKE_GRACE_S * 1000,
      attrs: { 'data-devices-revoked': id },
      actions: [{ label: t('v113Devices.undo'), icon: Undo2, primary: true, attrs: { 'data-devices-undo': id }, run: () => void undoRevoke(row) }],
    })
  }

  function mergeWith(key: string, yes: boolean): void {
    choices.value = choose(choices.value, key, yes)
    saveChoices(choices.value)
  }

  function unmerge(row: DeviceRow): void {
    if (!row.remote || !row.adb) return
    const key = `${row.remote.deviceId}|${row.adb.id}`
    choices.value = row.link === 'chosen' ? choose(forgetChoice(choices.value, key), key, false) : choices.value
    saveChoices(choices.value)
  }

  function toggle(key: string): void {
    openKey.value = openKey.value === key ? '' : key
  }

  watch(
    () => adb.devices.map((d) => `${d.id}:${d.state}:${d.online}`).join('|'),
    () => {
      void syncHashes()
      void syncPkg()
    },
    { immediate: true },
  )

  onMounted(() => {
    void remote.start()
    void adb.open()
    void emulators.load()
    void readDesktopVersion()
  })

  onBeforeUnmount(() => {
    gone = true
    if (ui.view !== 'devices') void adb.close()
  })

  return { filter, openKey, rows, shown, counts, merged, latency, busy, desktopVersion, pkg, hasAdb, actionOf, run, sendTest, openApp, pushTo, rename, undoRevoke, revoke, mergeWith, unmerge, toggle, say }
}
