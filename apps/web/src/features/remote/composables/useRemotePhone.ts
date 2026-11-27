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
  waitReason,
} from '@ccm/shared'
import { i18n } from '../../../i18n'
import { workspaceBridge } from '../../next/lib/workspaceBridge'
import { baseName } from '../../../core/lib/preview'
import { pickForPhone } from '../lib/remoteMenu'
import { questionOf } from '../lib/remoteSummary'
import { clearSnoozed } from '../../next/lib/railSnooze'
import { useSnoozeStore } from '../../monitor/stores/snooze'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useMenuQueueStore } from '@features/click-menu'
import { useInboxStore } from '@features/click-menu'
import { useRemoteStore } from '../stores/remote'
import { isTerm, presetOf, useWorkspaceStore, workCwd, type Pane } from '../../workspace/stores/workspace'
import { useCompact } from '../../agent-tools/composables/useCompact'
import { handleOrrery, hasOrrery } from '../../../core/composables/useOrreryApi'
import { usePaneInfo } from '../../workspace/composables/usePaneInfo'
import { useSessionActions, type ActFail } from '../../monitor/composables/useSessionActions'

export type PhoneResult = { ok: true; [key: string]: unknown } | { ok: false; error: RemoteErrorCode } | { needsConfirm: RemoteNeedsConfirm }

export const REPLY_WAIT_MS = 3000
export const REPLY_STABLE_MS = 1500
const SNOOZE_UNDO_MS = 30_000
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
  const undoable = new Map<string, { at: number; undo: () => void }>()
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
    const change = (): void => keepFocus(() => workspace.movePaneToTab(cmd.paneKey, cmd.tabKey))
    const track = workspaceBridge.value?.track
    if (track) track('phone', change)
    else change()
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
    workspace.update(pane.key, { prefill: text, prefillSubmit: true, prefillNow: true })
    log(call.device, 'send', { name: nameOf(pane) }, pane)
    return { ok: true, submitted: true }
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

  async function menu(call: RemotePhoneCall<'ws.menu'>): Promise<PhoneResult> {
    const pane = paneOf(call.cmd.paneKey)
    if (!pane || !isTerm(pane)) return fail('notFound')
    const queue = useMenuQueueStore()
    const picked = pickForPhone(queue.entryOf(pane.key), call.cmd.sig, call.cmd.index)
    if ('error' in picked) return fail(picked.error)
    await queue.answer(pane.key, picked.item)
    log(call.device, 'keys', { name: nameOf(pane), key: String(picked.item.index) }, pane)
    return { ok: true }
  }

  function snoozeOp(call: RemotePhoneCall<'ws.snooze'>): PhoneResult {
    const pane = paneOf(call.cmd.paneKey)
    if (!pane || !isTerm(pane)) return fail('notFound')
    const { minutes } = call.cmd
    const term = pane.terminalId ? useTerminalsStore().byId[pane.terminalId] : undefined
    const snoozes = useSnoozeStore()
    if (minutes === null) {
      const last = undoable.get(pane.key)
      undoable.delete(pane.key)
      if (last && Date.now() - last.at <= SNOOZE_UNDO_MS) last.undo()
      else {
        clearSnoozed(pane.key)
        if (term && snoozes.entries[term.id]) snoozes.clear(term.id)
      }
      return { ok: true, minutes: null }
    }
    const before = term ? snoozes.entries[term.id] : undefined
    const held = inbox.snoozePane(pane.key, minutes)
    if (!held) return fail('notWaiting')
    if (term) snoozes.snooze(term, held.minutes)
    undoable.set(pane.key, {
      at: Date.now(),
      undo: () => {
        held.undo()
        if (!term) return
        const left = before ? Math.ceil((before.until - Date.now()) / 60_000) : 0
        if (left > 0) snoozes.snooze(term, left)
        else if (snoozes.entries[term.id]) snoozes.clear(term.id)
      },
    })
    return { ok: true, minutes: held.minutes, until: Date.now() + held.minutes * 60_000 }
  }

  async function replied(key: string, text: string): Promise<'sent' | 'gone' | 'replaced' | 'timeout'> {
    const end = Date.now() + REPLY_WAIT_MS
    while (Date.now() < end) {
      const now = paneOf(key)
      if (!now || now.state !== 'running') return 'gone'
      if (now.prefill === undefined) return 'sent'
      if (now.prefill !== text) return 'replaced'
      await new Promise((resolve) => setTimeout(resolve, 50))
    }
    return 'timeout'
  }

  async function reply(call: RemotePhoneCall<'ws.reply'>): Promise<PhoneResult> {
    const pane = agentPane(call.cmd.paneKey)
    if (!pane) return fail('notFound')
    const text = cleanPanePrompt(call.cmd.text)
    if (!text) return fail('invalid')
    if (useMenuQueueStore().entryOf(pane.key)) return fail('useMenu')
    const session = act.sessionOf(pane)
    if (session?.status !== 'waiting') return fail('notWaiting')
    if (waitReason(session.waitingFor) === 'permission') return fail('useMenu')
    if (!questionOf(session.waitingFor)) return fail('notWaiting')
    const since = Date.parse(session.statusSince ?? '')
    if (!Number.isFinite(since) || Date.now() - since < REPLY_STABLE_MS) return fail('busy')
    if (pane.prefill || pane.compact) return fail('busy')
    workspace.update(pane.key, { prefill: text, prefillSubmit: true, prefillWaiting: true })
    const outcome = await replied(pane.key, text)
    if (outcome === 'sent') {
      log(call.device, 'send', { name: nameOf(pane) }, pane)
      return { ok: true, submitted: true }
    }
    if (outcome === 'gone') return fail('notFound')
    if (outcome === 'replaced') return fail('busy')
    if (paneOf(pane.key)?.prefill === text) workspace.update(pane.key, { prefill: undefined, prefillSubmit: undefined, prefillWaiting: undefined })
    return fail(act.sessionOf(pane)?.status === 'waiting' ? 'busy' : 'notWaiting')
  }

  function typed(sessionId: string): PhoneResult {
    const pane = workspace.panes.find((p) => isTerm(p) && p.sessionId === sessionId)
    if (!pane || !isTerm(pane)) return fail('notFound')
    if (presetOf(pane) === 'shell' && !pane.ranCommand) workspace.update(pane.key, { ranCommand: true })
    return { ok: true }
  }

  function logged(call: { device: string; alias: string }): PhoneResult {
    const alias = call.alias.trim()
    log(call.device, alias ? 'alias' : 'aliasClear', { name: alias })
    return { ok: true }
  }

  const recent = new Map<string, { at: number; out: Promise<PhoneResult> }>()
  handleOrrery('remote.typed', (call) => typed(call.sessionId))
  handleOrrery('remote.logged', (call) => logged(call))
  handleOrrery('ws.open', (call) => onceByRid(recent, call.rid, () => open(call)))
  handleOrrery('ws.close', (call) => onceByRid(recent, call.rid, () => close(call)))
  handleOrrery('ws.rename', (call) => onceByRid(recent, call.rid, () => rename(call)))
  handleOrrery('ws.move', (call) => onceByRid(recent, call.rid, () => move(call)))
  handleOrrery('ws.reopen', (call) => onceByRid(recent, call.rid, () => reopen(call)))
  handleOrrery('ws.compact', (call) => onceByRid(recent, call.rid, () => compact(call)))
  handleOrrery('ws.send', (call) => onceByRid(recent, call.rid, () => send(call)))
  handleOrrery('ws.keys', (call) => onceByRid(recent, call.rid, () => keys(call)))
  handleOrrery('ws.menu', (call) => onceByRid(recent, call.rid, () => menu(call)))
  handleOrrery('ws.snooze', (call) => onceByRid(recent, call.rid, () => snoozeOp(call)))
  handleOrrery('ws.reply', (call) => onceByRid(recent, call.rid, () => reply(call)))
}
