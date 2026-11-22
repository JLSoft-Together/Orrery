import { describe, expect, it, vi } from 'vitest'
import { leftRail, registerWorkspaceBridge, workspaceBridge, type WorkspaceBridge } from './workspaceBridge'

const stub = (): WorkspaceBridge => ({
  scripts: vi.fn(),
  run: vi.fn(),
  saveLayout: vi.fn(),
  manageLayouts: vi.fn(),
  openLayout: vi.fn(),
  split: vi.fn(),
  explorer: vi.fn(),
  projectDrag: vi.fn(),
  paneDrag: vi.fn(),
})

describe('workspaceBridge', () => {
  it('registers and unregisters', () => {
    const b = stub()
    const off = registerWorkspaceBridge(b)
    expect(workspaceBridge.value).toBe(b)
    off()
    expect(workspaceBridge.value).toBeNull()
  })

  it('a late unregister does not clear a newer bridge', () => {
    const a = stub()
    const b = stub()
    const offA = registerWorkspaceBridge(a)
    registerWorkspaceBridge(b)
    offA()
    expect(workspaceBridge.value).toBe(b)
  })

  it('leftRail is true only outside the box', () => {
    const box = { left: 0, right: 100, top: 0, bottom: 200 }
    expect(leftRail(box, 50, 50)).toBe(false)
    expect(leftRail(box, 101, 50)).toBe(true)
    expect(leftRail(box, 50, 201)).toBe(true)
  })
})
