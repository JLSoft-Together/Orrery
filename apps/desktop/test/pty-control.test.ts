import { readFileSync } from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'
import { encodeControl, invalidCensusRows, NdjsonReader, parseCoreToMain, parseMainToCore, PtyFrameQueue, utf8Bytes } from '../../../packages/shared/src/ptyControl'

const fixture = JSON.parse(readFileSync(path.join(__dirname, 'fixtures', 'pty-control.json'), 'utf8')) as Record<string, unknown[]>

describe('pty control contract', () => {
  it('accepts every main → core fixture unchanged', () => {
    for (const msg of fixture.mainToCore!) expect(parseMainToCore(msg)).toEqual(msg)
  })

  it('accepts every core → main fixture unchanged', () => {
    for (const msg of fixture.coreToMain!) expect(parseCoreToMain(msg)).toEqual(msg)
  })

  it('refuses malformed messages either way', () => {
    for (const msg of fixture.invalidMainToCore!) expect(parseMainToCore(msg)).toBeNull()
    for (const msg of fixture.invalidCoreToMain!) expect(parseCoreToMain(msg)).toBeNull()
  })

  it('a census reply with any bad row is refused whole, and the bad rows are counted', () => {
    const good = { id: 'a', preset: 'shell', cwd: '/', pid: 1, alive: true }
    const raw = { t: 'census-reply', sessions: [good, { id: 'b', preset: 'bash', cwd: '/', pid: 1, alive: true }, null] }
    expect(parseCoreToMain(raw)).toBeNull()
    expect(invalidCensusRows(raw)).toBe(2)
    expect(invalidCensusRows({ t: 'census-reply' })).toBe(-1)
    expect(invalidCensusRows({ t: 'spawned' })).toBeNull()
    expect(parseCoreToMain({ t: 'census-reply', sessions: [good] })).toEqual({ t: 'census-reply', sessions: [good] })
  })

  it('round trips through NDJSON split at any byte', () => {
    const lines = fixture.mainToCore!.map((m) => encodeControl(m as { t: string })).join('')
    for (const step of [1, 7, 64, lines.length]) {
      const got: unknown[] = []
      const reader = new NdjsonReader((v) => got.push(v))
      for (let i = 0; i < lines.length; i += step) reader.push(lines.slice(i, i + step))
      expect(got).toEqual(fixture.mainToCore)
    }
  })

  it('skips CRLF, blank and broken lines and any line over the max', () => {
    const got: unknown[] = []
    const reader = new NdjsonReader((v) => got.push(v), 32)
    reader.push('{"t":"census"}\r\n\n{broken\n')
    reader.push(`{"t":"x","pad":"${'a'.repeat(40)}`)
    reader.push(`${'b'.repeat(10)}"}\n{"t":"shutdown"}\n`)
    reader.push(`{"t":"y","pad":"${'c'.repeat(40)}"}\n{"t":"kill-all"}\n`)
    expect(got).toEqual([{ t: 'census' }, { t: 'shutdown' }, { t: 'kill-all' }])
  })

  it('counts the line max in UTF-8 bytes, like the Rust reader', () => {
    for (const t of ['a', 'é', '€', '😀', 'a�b']) expect(utf8Bytes(t)).toBe(Buffer.byteLength(t))
    const got: unknown[] = []
    const reader = new NdjsonReader((v) => got.push(v), 20)
    const viet = JSON.stringify({ t: 'ở' })
    expect(viet.length).toBeLessThanOrEqual(20)
    reader.push(`${JSON.stringify({ t: 'ởởởởởởở' })}
${viet}
`)
    reader.push(JSON.stringify({ t: 'ởởởởở' }).slice(0, 8))
    reader.push(`${JSON.stringify({ t: 'ởởởởở' }).slice(8)}
{"t":"k"}
`)
    expect(Buffer.byteLength(JSON.stringify({ t: 'ởởởởởởở' }))).toBeGreaterThan(20)
    expect(Buffer.byteLength(JSON.stringify({ t: 'ởởởởở' }))).toBeGreaterThan(20)
    expect(got).toEqual([{ t: 'ở' }, { t: 'k' }])
  })

  it('frame queue keeps text, drops binary past the byte max', () => {
    const q = new PtyFrameQueue<string>(10)
    expect(q.push('b1', 6, true)).toBe(true)
    expect(q.push('b2', 6, true)).toBe(false)
    expect(q.push('t1', 6, false)).toBe(true)
    expect(q.push('b3', 1, true)).toBe(false)
    expect(q.dropped).toBe(2)
    expect(q.take()).toEqual(['b1', 't1'])
    expect(q.push('b4', 10, true)).toBe(true)
  })
})
