import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bookmark as BookmarkIcon, BookmarkX, CircleAlert, Globe } from 'lucide-vue-next'
import type { IDecoration, IMarker, Terminal } from '@xterm/xterm'
import type { MenuEntry } from '../../../core/lib/menu'
import { BookmarkList, bookmarkLabel, type Bookmark, type BookmarkKind } from '../lib/termBookmarks'

type Mark = Bookmark<IMarker> & { deco?: IDecoration }

export interface TermBookmarksHost {
  notice: (text: string) => void
  mac: boolean
}

const TINT: Record<BookmarkKind, string> = {
  manual: 'var(--ccm-accent)',
  error: 'var(--ccm-ansi-9)',
  server: 'var(--ccm-ansi-10)',
}

export function useTermBookmarks(host: TermBookmarksHost) {
  const { t } = useI18n()
  let term: Terminal | null = null
  let jumped: { id: number; top: number } | null = null
  const version = ref(0)
  const list = new BookmarkList<IMarker>(undefined, (b) => {
    ;(b as Mark).deco?.dispose()
    version.value++
  })

  function install(terminal: Terminal): void {
    term = terminal
  }

  function currentLine(): number {
    const b = term?.buffer.active
    if (!b) return 0
    return b.viewportY >= b.baseY ? b.baseY + b.cursorY : b.viewportY
  }

  function markerAt(line: number): IMarker | null {
    const b = term?.buffer.active
    if (!term || !b) return null
    return term.registerMarker(line - (b.baseY + b.cursorY)) ?? null
  }

  function label(m: Bookmark<IMarker>): string {
    return bookmarkLabel(m.kind, m.detail, t)
  }

  function decorate(m: Mark): void {
    if (!term || m.marker.isDisposed) return
    const deco = term.registerDecoration({ marker: m.marker, x: 0, width: term.cols, height: 1 })
    if (!deco) return
    m.deco = deco
    deco.onRender((el) => {
      el.setAttribute('data-term-bookmark', m.kind)
      el.setAttribute('aria-label', label(m))
      el.title = label(m)
      Object.assign(el.style, {
        pointerEvents: 'none',
        left: '0px',
        width: '100%',
        boxSizing: 'border-box',
        borderLeft: `2px solid ${TINT[m.kind]}`,
        background: `color-mix(in srgb, ${TINT[m.kind]} 10%, transparent)`,
      })
    })
  }

  function create(kind: BookmarkKind, line: number, detail = ''): Mark | null {
    const marker = markerAt(line)
    if (!marker) return null
    const m = list.add(kind, marker, detail) as Mark | null
    if (!m) return null
    marker.onDispose(() => list.drop(m))
    decorate(m)
    version.value++
    return m
  }

  function toggle(): void {
    if (!term) return
    const line = currentLine()
    const existing = list.at(line)
    if (existing) {
      list.drop(existing)
      host.notice(t('termMarks.removed'))
      return
    }
    if (create('manual', line)) host.notice(t('termMarks.added'))
  }

  function addError(line: number, code: number): void {
    create('error', line, String(code))
  }

  function addServer(hostName: string): void {
    const b = term?.buffer.active
    if (b) create('server', b.baseY + b.cursorY, hostName)
  }

  function goTo(m: Bookmark<IMarker>): void {
    if (!term || m.marker.isDisposed) return
    term.scrollToLine(m.marker.line)
    jumped = { id: m.id, top: term.buffer.active.viewportY }
    term.focus()
  }

  function jump(dir: -1 | 1): boolean {
    const b = term?.buffer.active
    if (!term || !b || b.type !== 'normal') return false
    const skip = jumped && jumped.top === b.viewportY ? jumped.id : undefined
    const target = list.near(b.viewportY, dir, skip)
    if (target) {
      goTo(target)
      return true
    }
    host.notice(t(dir < 0 ? 'termMarks.noPrev' : 'termMarks.noNext'))
    return true
  }

  function clear(): void {
    jumped = null
    list.clear()
    version.value++
  }

  function dispose(): void {
    clear()
    term = null
  }

  const entries = computed<MenuEntry[]>(() => {
    void version.value
    const items = list.sorted()
    const out: MenuEntry[] = [{ id: 'mark:toggle', label: t('termMarks.toggle'), icon: BookmarkIcon, hint: t('termMarks.toggleHint', { key: host.mac ? '⌃⌥M' : 'Ctrl+Alt+M' }) }]
    for (const m of items) {
      const icon = m.kind === 'error' ? CircleAlert : m.kind === 'server' ? Globe : BookmarkIcon
      out.push({ id: `mark:go:${m.id}`, label: `${label(m)}  (${t('termMarks.line', { n: m.marker.line + 1 })})`, icon })
    }
    if (items.length) out.push({ id: 'mark:clear', label: t('termMarks.clear'), icon: BookmarkX })
    return out
  })

  function pick(id: string): boolean {
    if (!id.startsWith('mark:')) return false
    if (id === 'mark:toggle') toggle()
    else if (id === 'mark:clear') clear()
    else {
      const n = Number(id.slice('mark:go:'.length))
      const m = list.items.find((b) => b.id === n)
      if (m) goTo(m)
    }
    return true
  }

  const count = computed(() => {
    void version.value
    return list.items.length
  })

  return { install, toggle, jump, addError, addServer, clear, dispose, entries, pick, count }
}
