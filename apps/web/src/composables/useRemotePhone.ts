import {
  cleanNoteText,
  cleanPanePrompt,
  parseRemoteKeys,
  REMOTE_RID_KEEP,
  REMOTE_RID_TTL_MS,
  type RemoteErrorCode,
  type RemoteNeedsConfirm,
  type RemotePhoneCall,
  type SessionOpenError,
} from '@ccm/shared'
import { i18n } from '../i18n'
import { baseName } from '../lib/preview'
import { useInboxStore } from '../stores/inbox'
import { useRemoteStore } from '../stores/remote'
import { isTerm, presetOf, useWorkspaceStore, workCwd, type Pane } from '../stores/workspace'
import { useCompact } from './useCompact'
import { handleOrrery, hasOrrery } from './useOrreryApi'
import { usePaneInfo } from './usePaneInfo'
import { useSessionActions, type ActFail } from './useSessionActions'

export type PhoneResult = { ok: true; [key: string]: unknown } | { ok: false; error: RemoteErrorCode } | { needsConfirm: RemoteNeedsConfirm }

const AGENT_PRESETS = new Set(['claude', 'codex'])
let installed = false

const fail = (error: RemoteErrorCode): PhoneResult => ({ ok: false, error })

const OPEN_ERRORS: Record<SessionOpenError, RemoteErrorCode> = {
  unavailable: 'internal',
  limit: 'limit',
  cwd: 'invalid',
  preset: 'invalid',
  rate: 'rate',
  failed: 'internal',
  resume: 'invalid',
  live: 'busy',
  setup: 'denied',
}

const ACT_ERRORS: Record<ActFail['error'], RemoteErrorCode> = {
  none: 'notFound',
  gone: 'notFound',
  many: 'invalid',
  busy: 'busy',
  state: 'busy',
  unavailable: 'internal',
}

export function onceByRid(recent: Map<string, { at: number; out: Promise<PhoneResult> }>, rid: string | undefined, run: () => PhoneResult | Promise<PhoneResult>, now = Date.now()): Promise<PhoneResult> {
  for (const [k, v] of recent) if (now - v.at >= REMOTE_RID_TTL_MS) recent.delete(k)
  if (!rid) return Promise.resolve(run())
  const hit = recent.get(rid)
  if (hit) return hit.out
  const out = Promise.resolve(run())
  recent.set(rid, { at: now, out })
  while (recent.size > REMOTE_RID_KEEP) {
    const first = recent.keys().next().value
    if (first === undefined) break
    recent.delete(first)
  }
  return out
}

export function installRemotePhone(): void {
  if (installed || !hasOrrery()) return
  installed = true
  const workspace = useWorkspaceStore()
  const remote = useRemoteStore()
  const inbox = useInboxStore()
  const info = usePaneInfo()
  const act = useSessionActions()
  const t = (key: string, params: Record<string, unknown> = {}): string => i18n.global.t(key, params)

  const paneOf = (key: string): Pane | undefined => workspace.panes.find((p) => p.key === key)
  const agentPane = (key: string): Pane | undefined => {
    const pane = paneOf(key)
    return pane && isTerm(pane) && pane.state === 'running' && AGENT_PRESETS.has(presetOf(pane)) ? pane : undefined
  }
  const nameOf = (pane: Pane): string => act.nameOf(pane, { title: true })

  function keepFocus<T>(run: () => T): T {
    const before = workspace.activeKey
    const out = run()
    if (before && workspace.activeKey !== before && paneOf(before)) workspace.activate(before)
    return out
  }

  function log(deviceId: string, key: string, params: Record<string, unknown>, pane?: Pane): void {
    const device = remote.devices.find((d) => d.deviceId === deviceId)?.name ?? deviceId
    const text = t(`remoteV97.log.${key}`, params)
    remote.record(device, text, true, pane?.key)
    const session = pane ? act.sessionOf(pane) : undefined
    const note = cleanNoteText(t('remoteV97.log.note', { device, text }))
    if (session && note) inbox.addNote(session.id, { text: note, level: 'info', at: new Date().toISOString(), merged: 0 })
  }

  function open(call: RemotePhoneCall<'ws.open'>): PhoneResult {
    const { cmd, device } = call
    const res = keepFocus(() => workspace.openSession({ preset: cmd.preset, cwd: cmd.cwd, prompt: cmd.prompt, submit: cmd.submit, resume: cmd.resume }, { source: 'phone' }))
    if (!res.ok) return fail(OPEN_ERRORS[res.error] ?? 'internal')
    log(device, 'open', { name: baseName(cmd.cwd) || cmd.preset }, paneOf(res.paneKey))
    return { ok: true, paneKey: res.paneKey }
  }

  async function close(call: RemotePhoneCall<'ws.close'>): Promise<PhoneResult> {
    const { cmd, device, confirmed } = call
    const pane = paneOf(cmd.paneKey)
    if (!pane) return fail('notFound')
    const dirty = !!pane.dirty
    const busy = info.isBusy(pane)
    const name = nameOf(pane)
    if ((busy || dirty) && !confirmed) {
      const detail = [busy ? t('remoteV97.confirm.closeBusy') : '', dirty ? t('remoteV97.confirm.closeDirty') : ''].filter(Boolean).join(' ')
      return { needsConfirm: { title: t('remoteV97.confirm.closeTitle', { name }), detail, ok: t('remoteV97.confirm.closeOk'), cancel: t('remoteV97.confirm.cancel') } }
    }
    if (dirty && (await workspace.saveFiles([pane.key])).length) return fail('busy')
    const pending = workspace.pendingClose
    workspace.requestClose([pane.key], () => false)
    if (pending?.includes(pane.key)) {
      const rest = pending.filter((k) => k !== pane.key)
      if (rest.length) workspace.pendingClose = rest
      else workspace.cancelClose()
    }
    log(device, 'close', { name }, pane)
    return { ok: true }
  }

  function rename(call: RemotePhoneCall<'ws.rename'>): PhoneResult {
    const { cmd, device } = call
    if (!workspace.tabs.some((x) => x.key === cmd.tabKey)) return fail('notFound')
    workspace.renameTab(cmd.tabKey, cmd.name ?? '')
    log(device, 'rename', { name: cmd.name ?? '' })
    return { ok: true }
  }

  function move(call: RemotePhoneCall<'ws.move'>): PhoneResult {
    const { cmd, device } = call
    const pane = paneOf(cmd.paneKey)
    if (!pane || !workspace.tabs.some((x) => x.key === cmd.tabKey)) return fail('notFound')
    keepFocus(() => workspace.movePaneToTab(cmd.paneKey, cmd.tabKey))
    log(device, 'move', { name: nameOf(pane) }, pane)
    return { ok: true }
  }

  function reopen(call: RemotePhoneCall<'ws.reopen'>): PhoneResult {
    const out = keepFocus(() => workspace.reopenClosed())
    if (!out) return fail('notFound')
    log(call.device, 'reopen', { n: out.keys.length })
    return { ok: true, paneKeys: out.keys }
  }

  function compact(call: RemotePhoneCall<'ws.compact'>): PhoneResult {
    const pane = agentPane(call.cmd.paneKey)
    if (!pane) return fail('notFound')
    if (pane.prefill || pane.compact) return fail('busy')
    if (!useCompact().start(pane.key)) return fail('busy')
    log(call.device, 'compact', { name: nameOf(pane) }, pane)
    return { ok: true }
  }

  function send(call: RemotePhoneCall<'ws.send'>): PhoneResult {
    const pane = agentPane(call.cmd.paneKey)
    if (!pane) return fail('notFound')
    const text = cleanPanePrompt(call.cmd.text)
    if (!text) return fail('invalid')
    if (pane.prefill || pane.compact) return fail('busy')
    const idle = act.sessionOf(pane)?.status === 'idle'
    workspace.update(pane.key, { prefill: text, prefillSubmit: idle ? true : undefined })
    log(call.device, idle ? 'send' : 'type', { name: nameOf(pane) }, pane)
    return { ok: true, submitted: idle }
  }

  async function keys(call: RemotePhoneCall<'ws.keys'>): Promise<PhoneResult> {
    const press = parseRemoteKeys(call.cmd.keys)
    const pane = agentPane(call.cmd.paneKey)
    const dir = pane && (workCwd(pane) ?? pane.cwd)
    if (!press || !pane || !dir) return fail(press ? 'notFound' : 'invalid')
    const out = await act.keys({ project: dir, pane: pane.key }, { ...press, since: call.cmd.since })
    if (!out.ok) return fail(ACT_ERRORS[out.error])
    log(call.device, 'keys', { name: nameOf(pane), key: press.key === 'option' ? String(press.option) : press.key }, pane)
    return { ok: true }
  }

  function typed(sessionId: string): PhoneResult {
    const pane = workspace.panes.find((p) => isTerm(p) && p.sessionId === sessionId)
    if (!pane || !isTerm(pane)) return fail('notFound')
    if (presetOf(pane) === 'shell' && !pane.ranCommand) workspace.update(pane.key, { ranCommand: true })
    return { ok: true }
  }

  const recent = new Map<string, { at: number; out: Promise<PhoneResult> }>()
  handleOrrery('remote.typed', (call) => typed(call.sessionId))
  handleOrrery('ws.open', (call) => onceByRid(recent, call.rid, () => open(call)))
  handleOrrery('ws.close', (call) => onceByRid(recent, call.rid, () => close(call)))
  handleOrrery('ws.rename', (call) => onceByRid(recent, call.rid, () => rename(call)))
  handleOrrery('ws.move', (call) => onceByRid(recent, call.rid, () => move(call)))
  handleOrrery('ws.reopen', (call) => onceByRid(recent, call.rid, () => reopen(call)))
  handleOrrery('ws.compact', (call) => onceByRid(recent, call.rid, () => compact(call)))
  handleOrrery('ws.send', (call) => onceByRid(recent, call.rid, () => send(call)))
  handleOrrery('ws.keys', (call) => onceByRid(recent, call.rid, () => keys(call)))
}
