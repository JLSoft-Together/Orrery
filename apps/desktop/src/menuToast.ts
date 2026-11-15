import { timingSafeEqual } from 'node:crypto'
import { toastTag, type ToastContent } from '../../collector/src/toast'

export const MENU_TOAST_MAX_ANSWERS = 3
const MAX_ENTRIES = 50
const MAX_ID = 200
const MAX_TEXT = 200
const MAX_LABEL = 40

export interface MenuToastOption {
  index: number
  label: string
}

export interface MenuToastEntry {
  paneKey: string
  sig: string
  terminalId?: string
  title: string
  place: string
  options: MenuToastOption[] | null
}

export interface MenuToastSync {
  enabled: boolean
  open: string
  dismiss: string
  entries: MenuToastEntry[]
}

export interface MenuToastContent {
  tag: string
  title: string
  body: string
  persistent: boolean
  open: { label: string; url: string }
  dismiss: string
  answers: { label: string; url: string }[]
}

export interface MenuToastDrawer {
  show(content: MenuToastContent): Promise<boolean>
  hide(tag: string): Promise<boolean>
}

export function windowsMenuDrawer(drawer: { show(c: ToastContent): Promise<boolean>; hide(tag: string): Promise<boolean> }): MenuToastDrawer {
  return {
    hide: (tag) => drawer.hide(tag),
    show: (c) =>
      drawer.show({
        tag: c.tag,
        title: c.title,
        body: c.body,
        url: c.open.url,
        open: c.open.label,
        dismiss: c.dismiss,
        persistent: c.persistent,
        answers: c.answers,
      }),
  }
}

export interface NativeNote {
  on(event: 'click', cb: () => void): void
  on(event: 'action', cb: (event: unknown, index: number) => void): void
  show(): void
  close(): void
}

export type NativeNoteFactory = (opts: { title: string; body: string; actions: { type: 'button'; text: string }[]; hasReply: false }) => NativeNote

export function electronMenuDrawer(make: NativeNoteFactory, onLink: (url: string) => void): MenuToastDrawer {
  const live = new Map<string, NativeNote>()
  const close = (tag: string): void => {
    live.get(tag)?.close()
    live.delete(tag)
  }
  return {
    hide: async (tag) => {
      close(tag)
      return true
    },
    show: async (c) => {
      close(c.tag)
      const note = make({ title: c.title, body: c.body, actions: c.answers.map((a) => ({ type: 'button', text: a.label })), hasReply: false })
      note.on('click', () => onLink(c.open.url))
      note.on('action', (_e, index) => {
        const a = c.answers[index]
        if (a) onLink(a.url)
      })
      live.set(c.tag, note)
      note.show()
      return true
    },
  }
}

export type MenuToastAction = { type: 'answer'; paneKey: string; sig: string; index: number } | { type: 'open'; paneKey: string; sig: string }

export interface MenuToasts {
  sync(data: unknown): void
  settle(): void
  clear(): void
  activate(raw: string): MenuToastAction | null
  link(kind: 'open' | 'answer', paneKey: string, sig: string, index?: number): string
}

const str = (v: unknown, max: number): v is string => typeof v === 'string' && v.length <= max

function parseOption(v: unknown): MenuToastOption | null {
  if (typeof v !== 'object' || v === null) return null
  const o = v as Record<string, unknown>
  if (typeof o.index !== 'number' || !Number.isInteger(o.index) || o.index < 0 || o.index > 99) return null
  if (typeof o.label !== 'string') return null
  return { index: o.index, label: o.label.slice(0, MAX_LABEL) }
}

function parseEntry(v: unknown): MenuToastEntry | null {
  if (typeof v !== 'object' || v === null) return null
  const e = v as Record<string, unknown>
  if (!str(e.paneKey, MAX_ID) || !e.paneKey || !str(e.sig, MAX_ID) || !e.sig) return null
  if (typeof e.title !== 'string' || typeof e.place !== 'string') return null
  if (e.terminalId !== undefined && !str(e.terminalId, MAX_ID)) return null
  let options: MenuToastOption[] | null = null
  if (Array.isArray(e.options)) {
    const parsed = e.options.map(parseOption)
    const ok = parsed.length > 0 && parsed.length <= MENU_TOAST_MAX_ANSWERS && parsed.every((o): o is MenuToastOption => o !== null)
    if (ok && new Set(parsed.map((o) => o?.index)).size === parsed.length) options = parsed as MenuToastOption[]
  }
  return {
    paneKey: e.paneKey,
    sig: e.sig,
    ...(typeof e.terminalId === 'string' && e.terminalId ? { terminalId: e.terminalId } : {}),
    title: e.title.slice(0, MAX_TEXT),
    place: e.place.slice(0, MAX_TEXT),
    options,
  }
}

export function parseMenuToastSync(data: unknown): MenuToastSync | null {
  if (typeof data !== 'object' || data === null) return null
  const m = data as Record<string, unknown>
  if (typeof m.enabled !== 'boolean' || !str(m.open, MAX_LABEL) || !str(m.dismiss, MAX_LABEL) || !Array.isArray(m.entries) || m.entries.length > MAX_ENTRIES) return null
  const entries: MenuToastEntry[] = []
  for (const raw of m.entries) {
    const entry = parseEntry(raw)
    if (!entry) return null
    entries.push(entry)
  }
  return { enabled: m.enabled, open: m.open, dismiss: m.dismiss, entries }
}

export const menuToastTag = (e: Pick<MenuToastEntry, 'paneKey' | 'terminalId'>): string => toastTag(e.terminalId ? `waiting:${e.terminalId}` : `menu:${e.paneKey}`)

interface Known {
  entry: MenuToastEntry
  tag: string
  shown: boolean
  gone: ReturnType<typeof setTimeout> | null
}

export const MENU_TOAST_GRACE_MS = 2000

const sameKey = (a: string, b: string): boolean => {
  const x = Buffer.from(a)
  const y = Buffer.from(b)
  return x.length === y.length && timingSafeEqual(x, y)
}

export interface MenuToastDeps {
  drawer: MenuToastDrawer
  key: string
  scheme: string
  focused: () => boolean
  allowed?: () => boolean
  graceMs?: number
}

export function createMenuToasts(deps: MenuToastDeps): MenuToasts {
  const known = new Map<string, Known>()
  let labels = { open: '', dismiss: '' }
  let queue: Promise<unknown> = Promise.resolve()
  const run = (job: () => Promise<unknown>): void => {
    queue = queue.then(job, job).catch(() => undefined)
  }

  const link = (kind: 'open' | 'answer', paneKey: string, sig: string, index?: number): string => {
    const q = new URLSearchParams({ k: deps.key, p: paneKey, s: sig })
    if (kind === 'answer' && index !== undefined) q.set('i', String(index))
    return `${deps.scheme}://menu?${q}`
  }

  const hide = (k: Known): void => {
    if (!k.shown) return
    k.shown = false
    run(() => deps.drawer.hide(k.tag))
  }

  const show = (k: Known): void => {
    const e = k.entry
    const content: MenuToastContent = {
      tag: k.tag,
      title: e.title,
      body: e.place,
      persistent: true,
      open: { label: labels.open, url: link('open', e.paneKey, e.sig) },
      dismiss: labels.dismiss,
      answers: (e.options ?? []).map((o) => ({ label: o.label, url: link('answer', e.paneKey, e.sig, o.index) })),
    }
    k.shown = true
    run(() => deps.drawer.show(content))
  }

  const drop = (k: Known): void => {
    if (k.gone) clearTimeout(k.gone)
    k.gone = null
    hide(k)
    if (known.get(k.entry.paneKey) === k) known.delete(k.entry.paneKey)
  }

  const clear = (): void => {
    for (const k of [...known.values()]) drop(k)
  }

  return {
    link,
    clear,
    settle() {
      for (const k of known.values()) hide(k)
    },
    sync(data) {
      const parsed = parseMenuToastSync(data)
      if (!parsed) return
      if (!parsed.enabled || deps.allowed?.() === false) {
        clear()
        return
      }
      labels = { open: parsed.open, dismiss: parsed.dismiss }
      const alive = new Set(parsed.entries.map((e) => e.paneKey))
      for (const [paneKey, k] of [...known]) {
        if (alive.has(paneKey)) continue
        if (k.gone) continue
        k.gone = setTimeout(() => drop(k), deps.graceMs ?? MENU_TOAST_GRACE_MS)
        k.gone.unref?.()
      }
      const away = !deps.focused()
      for (const entry of parsed.entries) {
        const cur = known.get(entry.paneKey)
        if (cur && cur.gone) {
          clearTimeout(cur.gone)
          cur.gone = null
        }
        if (cur && cur.entry.sig === entry.sig) {
          cur.entry = entry
          continue
        }
        if (cur) drop(cur)
        const next: Known = { entry, tag: menuToastTag(entry), shown: false, gone: null }
        known.set(entry.paneKey, next)
        if (away) show(next)
      }
    },
    activate(raw) {
      let url: URL
      try {
        url = new URL(raw)
      } catch {
        return null
      }
      if (url.protocol !== `${deps.scheme}:` || url.hostname !== 'menu') return null
      const k = url.searchParams.get('k') ?? ''
      const p = url.searchParams.get('p') ?? ''
      const s = url.searchParams.get('s') ?? ''
      if (!k || !sameKey(k, deps.key) || !p || !s) return null
      const cur = known.get(p)
      if (!cur || cur.gone || cur.entry.sig !== s) return null
      const i = url.searchParams.get('i')
      if (i === null) return { type: 'open', paneKey: p, sig: s }
      if (!/^\d{1,2}$/.test(i)) return null
      const index = Number(i)
      if (!cur.entry.options?.some((o) => o.index === index)) return null
      hide(cur)
      return { type: 'answer', paneKey: p, sig: s, index }
    },
  }
}
