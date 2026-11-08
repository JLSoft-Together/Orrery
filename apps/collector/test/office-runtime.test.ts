import { describe, expect, it, vi } from 'vitest'
import { OfficeRuntime, type OfficeParts } from '../src/office/runtime'

function setup() {
  const made: Array<{ parts: OfficeParts; hub: Record<string, ReturnType<typeof vi.fn>>; recorder: Record<string, ReturnType<typeof vi.fn>>; flush: ReturnType<typeof vi.fn> }> = []
  const runtime = new OfficeRuntime(() => {
    const hub = { start: vi.fn(), stop: vi.fn(), subscribe: vi.fn(), dropClient: vi.fn() }
    const recorder = { start: vi.fn(), stop: vi.fn() }
    const flush = vi.fn()
    const parts = { hub, recorder, flush } as unknown as OfficeParts
    made.push({ parts, hub, recorder, flush })
    return parts
  })
  return { runtime, made }
}

describe('OfficeRuntime', () => {
  it('creates nothing by default, even after ready', () => {
    const { runtime, made } = setup()
    runtime.setReady()
    expect(made).toHaveLength(0)
    expect(runtime.active).toBe(false)
  })

  it('waits for ready before creating', () => {
    const { runtime, made } = setup()
    runtime.setEnabled('c1', true)
    expect(made).toHaveLength(0)
    runtime.setReady()
    expect(made).toHaveLength(1)
    expect(made[0]!.hub.start).toHaveBeenCalledTimes(1)
    expect(made[0]!.recorder.start).toHaveBeenCalledTimes(1)
  })

  it('stops hub and recorder when the last enabled client turns it off', () => {
    const { runtime, made } = setup()
    runtime.setReady()
    runtime.setEnabled('c1', true)
    runtime.setEnabled('c2', true)
    runtime.setEnabled('c1', false)
    expect(runtime.active).toBe(true)
    runtime.setEnabled('c2', false)
    expect(runtime.active).toBe(false)
    expect(made[0]!.hub.stop).toHaveBeenCalledTimes(1)
    expect(made[0]!.recorder.stop).toHaveBeenCalledTimes(1)
    expect(made[0]!.flush).toHaveBeenCalledTimes(1)
  })

  it('stops when the only enabled client disconnects', () => {
    const { runtime, made } = setup()
    runtime.setReady()
    runtime.setEnabled('c1', true)
    runtime.dropClient('c1')
    expect(runtime.active).toBe(false)
    expect(made[0]!.recorder.stop).toHaveBeenCalledTimes(1)
  })

  it('ignores repeated enables and builds fresh parts after a quick off/on', () => {
    const { runtime, made } = setup()
    runtime.setReady()
    runtime.setEnabled('c1', true)
    runtime.setEnabled('c1', true)
    expect(made).toHaveLength(1)
    runtime.setEnabled('c1', false)
    runtime.setEnabled('c1', true)
    expect(made).toHaveLength(2)
    expect(made[0]!.hub.stop).toHaveBeenCalledTimes(1)
    expect(made[1]!.hub.start).toHaveBeenCalledTimes(1)
  })

  it('forwards subscribes and replays them into a hub created later', () => {
    const { runtime, made } = setup()
    runtime.setReady()
    runtime.subscribe('c1', true)
    expect(made).toHaveLength(0)
    runtime.setEnabled('c1', true)
    expect(made[0]!.hub.subscribe).toHaveBeenCalledWith('c1', true)
    runtime.subscribe('c1', false)
    expect(made[0]!.hub.subscribe).toHaveBeenLastCalledWith('c1', false)
  })

  it('close stops everything and blocks later enables', () => {
    const { runtime, made } = setup()
    runtime.setReady()
    runtime.setEnabled('c1', true)
    runtime.close()
    expect(made[0]!.recorder.stop).toHaveBeenCalledTimes(1)
    runtime.setEnabled('c2', true)
    expect(made).toHaveLength(1)
  })
})
