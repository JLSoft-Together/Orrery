import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MAX_ENTRIES, MAX_MESSAGE, MAX_STACK_LINES, clearErrorLog, errorLogCount, errorLogText, installErrorLog, onUncaughtError, recordError, uninstallErrorLog } from './errorLog'

describe('errorLog', () => {
  let original: typeof console.error
  beforeEach(() => {
    original = console.error
    console.error = vi.fn()
    clearErrorLog()
  })
  afterEach(() => {
    uninstallErrorLog()
    console.error = original
  })

  it('skips benign browser noise', () => {
    recordError('error', 'ResizeObserver loop completed with undelivered notifications.')
    recordError('error', 'Script error.')
    expect(errorLogCount()).toBe(0)
  })

  it('records window errors with stack lines', () => {
    installErrorLog()
    const error = new Error('boom')
    window.dispatchEvent(new ErrorEvent('error', { message: 'boom', error }))
    expect(errorLogCount()).toBe(1)
    const text = errorLogText()
    expect(text).toMatch(/^\d{4}-\d\d-\d\dT[\d:.]+Z \[error\] boom/)
  })

  it('records unhandled rejections', () => {
    installErrorLog()
    const ev = new Event('unhandledrejection') as Event & { reason: unknown }
    ev.reason = new Error('nope')
    window.dispatchEvent(ev)
    expect(errorLogText()).toContain('[unhandledrejection] nope')
  })

  it('wraps console.error and still calls the original', () => {
    const spy = vi.fn()
    console.error = spy
    installErrorLog()
    console.error('bad', { a: 1 })
    expect(spy).toHaveBeenCalledWith('bad', { a: 1 })
    expect(errorLogText()).toContain('[console.error] bad {"a":1}')
  })

  it('is idempotent', () => {
    installErrorLog()
    installErrorLog()
    window.dispatchEvent(new ErrorEvent('error', { message: 'once' }))
    expect(errorLogCount()).toBe(1)
  })

  it('caps entries, message length and stack lines', () => {
    for (let i = 0; i < MAX_ENTRIES + 25; i++) recordError('error', `m${i}`)
    expect(errorLogCount()).toBe(MAX_ENTRIES)
    expect(errorLogText()).not.toContain('m0\n')
    expect(errorLogText()).toContain(`m${MAX_ENTRIES + 24}`)
    clearErrorLog()
    recordError('error', 'x'.repeat(MAX_MESSAGE + 500), Array.from({ length: 40 }, (_, i) => `at f${i}`))
    const lines = errorLogText().split('\n')
    expect(lines).toHaveLength(1 + MAX_STACK_LINES)
    expect(lines[0]!.length).toBeLessThan(MAX_MESSAGE + 80)
  })

  it('notifies uncaught listeners but not for console.error', () => {
    installErrorLog()
    const seen: string[] = []
    const off = onUncaughtError((e) => seen.push(`${e.kind}:${e.message}`))
    console.error('quiet')
    window.dispatchEvent(new ErrorEvent('error', { message: 'loud' }))
    off()
    window.dispatchEvent(new ErrorEvent('error', { message: 'later' }))
    expect(seen).toEqual(['error:loud'])
  })

  it('ignores resource load errors that are not ErrorEvent', () => {
    installErrorLog()
    window.dispatchEvent(new Event('error'))
    expect(errorLogCount()).toBe(0)
  })
})
