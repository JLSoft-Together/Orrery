import { watch } from 'vue'
import type { CheckState, ChecklistCheck, ChecklistItem } from '../../../core/lib/preview'
import type { ScriptGroup } from '../../workspace/lib/projects'
import type { Pane } from '../../workspace/stores/workspace'

export type CheckView = 'none' | 'idle' | 'running' | 'pass' | 'fail' | 'stale'

export interface MergeWarning {
  open: number
  texts: string[]
  failed: boolean
}

export const openItems = (items: readonly ChecklistItem[]): ChecklistItem[] => items.filter((i) => !i.done)

export function checkView(check: ChecklistCheck, head: string, live: boolean): CheckView {
  if (!check.script) return 'none'
  if (check.state === 'running') return live ? 'running' : 'idle'
  if (check.state === 'idle') return 'idle'
  if (head && check.head && head !== check.head) return 'stale'
  return check.state
}

export function mergeWarning(items: readonly ChecklistItem[], view: CheckView): MergeWarning | null {
  const open = openItems(items)
  const failed = view === 'fail'
  if (!open.length && !failed) return null
  return { open: open.length, texts: open.slice(0, 3).map((i) => i.text), failed }
}

export function scriptOptions(groups: readonly ScriptGroup[]): { run: string; name: string }[] {
  const out: { run: string; name: string }[] = []
  for (const g of groups) for (const item of g.items) if (out.length < 40) out.push({ run: item.run, name: item.name })
  return out
}

export function defaultScript(options: readonly { run: string; name: string }[]): string {
  return options.find((o) => o.name === 'qa')?.run ?? ''
}

export interface CheckRunDeps {
  head: () => Promise<string>
  report: (state: CheckState, head: string) => Promise<unknown>
  launch: (script: string) => string
  wait: (paneKey: string) => Promise<{ code: number } | null>
}

const live = new Set<string>()
const listeners = new Map<string, Set<() => void>>()

export function onCheckDone(dir: string, cb: () => void): () => void {
  const set = listeners.get(dir) ?? new Set<() => void>()
  set.add(cb)
  listeners.set(dir, set)
  return () => void set.delete(cb)
}

export const isCheckLive = (dir: string): boolean => live.has(dir)

export async function runCheck(dir: string, script: string, deps: CheckRunDeps): Promise<CheckState> {
  if (live.has(dir) || !script.trim()) return 'idle'
  live.add(dir)
  try {
    const head = await deps.head()
    if (!head) return 'idle'
    await deps.report('running', head)
    const key = deps.launch(script)
    const res = await deps.wait(key)
    const state: CheckState = res === null ? 'idle' : res.code === 0 ? 'pass' : 'fail'
    await deps.report(state, head)
    return state
  } finally {
    live.delete(dir)
    for (const cb of [...(listeners.get(dir) ?? [])]) cb()
  }
}

export type PhoneRunError = 'running' | 'script' | 'integration' | 'unavailable'
export type PhoneRunResult = { ok: true; started: true } | { ok: false; error: PhoneRunError }

export interface PhoneRunDeps extends CheckRunDeps {
  script: () => Promise<string | null>
  integration: () => boolean
}

export async function startCheckFromPhone(dir: string, deps: PhoneRunDeps): Promise<PhoneRunResult> {
  if (isCheckLive(dir)) return { ok: false, error: 'running' }
  if (!deps.integration()) return { ok: false, error: 'integration' }
  const script = await deps.script()
  if (script === null) return { ok: false, error: 'unavailable' }
  if (!script.trim()) return { ok: false, error: 'script' }
  if (isCheckLive(dir)) return { ok: false, error: 'running' }
  runCheck(dir, script, deps).catch(() => undefined)
  return { ok: true, started: true }
}

export function waitShellResult(panes: () => readonly Pane[], key: string): Promise<{ code: number } | null> {
  return new Promise((resolve) => {
    let done = false
    let stop: (() => void) | null = null
    const find = () => panes().find((p) => p.key === key)
    const finish = (value: { code: number } | null) => {
      if (done) return
      done = true
      resolve(value)
      stop?.()
    }
    const handle = () => {
      const pane = find()
      if (!pane) return finish(null)
      if (pane.shellLast) return finish({ code: pane.shellLast.code })
      if (pane.state === 'exited' || pane.state === 'gone' || pane.state === 'error') finish(null)
    }
    stop = watch(() => { const p = find(); return `${p ? p.state : 'none'}:${p?.shellLast?.code ?? ''}:${p?.shellLast?.ms ?? ''}` }, handle, { immediate: true })
    if (done) stop()
  })
}
