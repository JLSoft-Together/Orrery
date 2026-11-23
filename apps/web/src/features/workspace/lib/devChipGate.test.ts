import { describe, expect, it } from 'vitest'
import { DevChipGate } from './devChipGate'

describe('DevChipGate', () => {
  it('hides the chip when the command that printed the URL ends', () => {
    const gate = new DevChipGate()
    gate.bind(7)
    expect(gate.endedBy(6)).toBe(false)
    expect(gate.endedBy(7)).toBe(true)
    expect(gate.endedBy(7)).toBe(false)
  })

  it('keeps the chip when the URL came from no running command (background output)', () => {
    const gate = new DevChipGate()
    gate.bind(null)
    expect(gate.endedBy(1)).toBe(false)
  })

  it('follows the newest command that printed a URL', () => {
    const gate = new DevChipGate()
    gate.bind(3)
    gate.bind(4)
    expect(gate.endedBy(3)).toBe(false)
    expect(gate.endedBy(4)).toBe(true)
  })

  it('reset forgets the binding', () => {
    const gate = new DevChipGate()
    gate.bind(2)
    gate.reset()
    expect(gate.endedBy(2)).toBe(false)
  })
})
