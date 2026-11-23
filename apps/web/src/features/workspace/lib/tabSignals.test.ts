import { describe, expect, it } from 'vitest'
import { tabExtras, tabSignal, tabSignalLines } from './tabSignals'

describe('tabSignal', () => {
  it('ranks waiting over error over working', () => {
    expect(tabSignal({ status: 'working', attention: 'waiting' })).toBe('waiting')
    expect(tabSignal({ status: 'waiting' })).toBe('waiting')
    expect(tabSignal({ status: 'error', dirty: true })).toBe('error')
    expect(tabSignal({ status: 'working', telegram: true, dirty: true, zoom: true })).toBe('working')
  })
  it('treats a failed shell command as an error', () => {
    expect(tabSignal({ status: 'idle', attention: 'done', shellFailed: true })).toBe('error')
  })
  it('stays quiet for idle, done and plain tabs', () => {
    expect(tabSignal({ status: 'idle' })).toBeNull()
    expect(tabSignal({ attention: 'done' })).toBeNull()
    expect(tabSignal({})).toBeNull()
  })
})

describe('tabExtras', () => {
  it('collects the secondary signals in a stable order', () => {
    expect(tabExtras({ attention: 'done', peer: true, telegram: true, dirty: true, zoom: true, ended: true })).toEqual(['done', 'peer', 'telegram', 'dirty', 'zoom', 'ended'])
  })
  it('does not repeat a failed shell run as done', () => {
    expect(tabExtras({ attention: 'done', shellFailed: true })).toEqual([])
  })
  it('is empty when nothing else applies', () => {
    expect(tabExtras({ status: 'working' })).toEqual([])
  })
})

describe('tabSignalLines', () => {
  it('puts the main signal first and drops empty text', () => {
    const text = (k: string): string => (k === 'peer' ? '' : `t:${k}`)
    expect(tabSignalLines('error', ['peer', 'dirty'], text)).toEqual(['t:error', 't:dirty'])
    expect(tabSignalLines(null, ['zoom'], text)).toEqual(['t:zoom'])
    expect(tabSignalLines(null, [], text)).toEqual([])
  })
})
