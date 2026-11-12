import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { RemoteStatus } from '@ccm/shared'
import { hasRemote, isConnected, onRemoteChanged, remoteDeviceRename, remoteEnable, remoteKeepAwake, remotePushDevice, remoteRevoke, remoteRevokeAll, remoteStatus, REMOTE_LOG_MAX, REMOTE_TOUCH_MS, type RemoteLogLine } from '../lib/remote'

let seq = 0

export const useRemoteStore = defineStore('remote', () => {
  const status = ref<RemoteStatus | null>(null)
  const log = ref<RemoteLogLine[]>([])
  const touched = ref<Record<string, { at: number; device: string }>>({})
  const sized = ref<Record<string, true>>({})
  let off: (() => void) | null = null

  const supported = hasRemote
  const enabled = computed(() => !!status.value?.enabled)
  const devices = computed(() => status.value?.devices ?? [])
  const connected = computed(() => devices.value.filter(isConnected).length)

  async function start(): Promise<void> {
    if (!hasRemote || off) return
    off = onRemoteChanged((next) => (status.value = next))
    status.value = await remoteStatus()
  }

  function stop(): void {
    off?.()
    off = null
  }

  const apply = (next: RemoteStatus | null): void => {
    if (next) status.value = next
  }

  const setEnabled = async (on: boolean): Promise<void> => apply(await remoteEnable(on))
  const setKeepAwake = async (on: boolean): Promise<void> => apply(await remoteKeepAwake(on))

  async function revoke(deviceId: string): Promise<boolean> {
    const ok = await remoteRevoke(deviceId)
    if (ok) apply(await remoteStatus())
    return ok
  }

  async function renameDevice(deviceId: string, name: string): Promise<boolean> {
    const ok = await remoteDeviceRename(deviceId, name)
    if (ok) apply(await remoteStatus())
    return ok
  }

  async function setDevicePush(deviceId: string, on: boolean): Promise<boolean> {
    const ok = await remotePushDevice(deviceId, on)
    if (ok) apply(await remoteStatus())
    return ok
  }

  async function revokeAll(): Promise<boolean> {
    const ok = await remoteRevokeAll()
    if (ok) apply(await remoteStatus())
    return ok
  }

  function record(device: string, text: string, ok: boolean, paneKey?: string): void {
    const at = Date.now()
    log.value = [{ id: ++seq, at, device, text, ok }, ...log.value].slice(0, REMOTE_LOG_MAX)
    if (paneKey) touched.value = { ...touched.value, [paneKey]: { at, device } }
  }

  function setSized(paneKey: string, on: boolean): void {
    if (on === !!sized.value[paneKey]) return
    const next = { ...sized.value }
    if (on) next[paneKey] = true
    else delete next[paneKey]
    sized.value = next
  }

  function watching(paneKey: string, at: number): string | null {
    if (!connected.value) return null
    const hit = touched.value[paneKey]
    if (hit && at - hit.at < REMOTE_TOUCH_MS) return hit.device
    return sized.value[paneKey] ? (devices.value.find(isConnected)?.name ?? '') : null
  }

  return { status, log, touched, sized, supported, enabled, devices, connected, start, stop, setEnabled, setKeepAwake, revoke, renameDevice, setDevicePush, revokeAll, record, setSized, watching }
})
