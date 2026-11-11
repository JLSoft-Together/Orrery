export type ErrorKind = 'error' | 'unhandledrejection' | 'console.error'

export interface ErrorEntry {
  at: string
  kind: ErrorKind
  message: string
  stack: string[]
}

export interface UncaughtError {
  kind: 'error' | 'unhandledrejection'
  message: string
}

export const MAX_ENTRIES = 300
export const MAX_MESSAGE = 2000
export const MAX_STACK_LINES = 15

let entries: ErrorEntry[] = []
let installed = false
let originalConsoleError: typeof console.error | null = null
const listeners = new Set<(e: UncaughtError) => void>()

function clip(text: string): string {
  return text.length > MAX_MESSAGE ? `${text.slice(0, MAX_MESSAGE)}…` : text
}

function stackOf(error: unknown, message: string): string[] {
  const raw = error instanceof Error ? error.stack : undefined
  if (!raw) return []
  const lines = raw.split('\n').map((l) => l.trim()).filter(Boolean)
  const first = message.split('\n')[0] ?? ''
  const body = lines[0] && first && lines[0].includes(first.slice(0, 40)) ? lines.slice(1) : lines
  return body.slice(0, MAX_STACK_LINES).map((l) => (l.length > 300 ? `${l.slice(0, 300)}…` : l))
}

function describe(value: unknown): string {
  if (value instanceof Error) return value.message || value.name
  if (typeof value === 'string') return value
  if (value === undefined) return 'undefined'
  try {
    return JSON.stringify(value) ?? String(value)
  } catch {
    return String(value)
  }
}

export function recordError(kind: ErrorKind, message: string, stack: string[] = []): void {
  entries.push({ at: new Date().toISOString(), kind, message: clip(message), stack: stack.slice(0, MAX_STACK_LINES) })
  if (entries.length > MAX_ENTRIES) entries = entries.slice(entries.length - MAX_ENTRIES)
}

function emit(e: UncaughtError): void {
  for (const fn of [...listeners]) {
    try {
      fn(e)
    } catch {
      continue
    }
  }
}

function onError(ev: Event): void {
  if (!(ev instanceof ErrorEvent)) return
  const message = ev.message || describe(ev.error)
  recordError('error', message, stackOf(ev.error, message))
  emit({ kind: 'error', message })
}

function onRejection(ev: PromiseRejectionEvent): void {
  const message = describe(ev.reason)
  recordError('unhandledrejection', message, stackOf(ev.reason, message))
  emit({ kind: 'unhandledrejection', message })
}

export function installErrorLog(): void {
  if (installed || typeof window === 'undefined') return
  installed = true
  window.addEventListener('error', onError)
  window.addEventListener('unhandledrejection', onRejection)
  originalConsoleError = console.error
  const original = originalConsoleError
  console.error = (...args: unknown[]): void => {
    try {
      const first = args.find((a) => a instanceof Error)
      recordError('console.error', args.map(describe).join(' '), stackOf(first, ''))
    } catch {
      void 0
    }
    original.apply(console, args)
  }
}

export function uninstallErrorLog(): void {
  if (!installed) return
  installed = false
  window.removeEventListener('error', onError)
  window.removeEventListener('unhandledrejection', onRejection)
  if (originalConsoleError) console.error = originalConsoleError
  originalConsoleError = null
}

export function onUncaughtError(fn: (e: UncaughtError) => void): () => void {
  listeners.add(fn)
  return () => void listeners.delete(fn)
}

export function clearErrorLog(): void {
  entries = []
}

export const errorLogCount = (): number => entries.length

export function errorLogText(): string {
  return entries.map((e) => [`${e.at} [${e.kind}] ${e.message}`, ...e.stack.map((l) => `    ${l}`)].join('\n')).join('\n')
}
