import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, nextTick, ref } from 'vue'

type Sizes = { version: string; sizes: Record<string, number | null> }

let doneCb: (() => void) | null = null
const diskMapSizes = vi.fn<(paths: string[]) => Promise<Sizes | null>>()

async function load(has: boolean) {
  vi.resetModules()
  doneCb = null
  vi.doMock('../lib/diskMap', async (orig) => {
    const real = await orig<typeof import('../lib/diskMap')>()
    return {
      ...real,
      hasDiskSizes: has,
      diskMapSizes: (paths: string[]) => diskMapSizes(paths),
      onDiskMapDone: (cb: () => void) => {
        doneCb = cb
        return () => {
          doneCb = null
        }
      },
    }
  })
  return import('./useDiskSizes')
}

const answer = (version: string, table: Record<string, number>) => (paths: string[]) =>
  Promise.resolve<Sizes>({ version, sizes: Object.fromEntries(paths.map((p) => [p, table[p.toLowerCase()] ?? null])) })

beforeEach(() => {
  vi.useFakeTimers()
  diskMapSizes.mockReset()
})
afterEach(() => {
  vi.useRealTimers()
  vi.doUnmock('../lib/diskMap')
})

async function flush(): Promise<void> {
  await vi.advanceTimersByTimeAsync(200)
  await nextTick()
}

describe('useDiskSizes', () => {
  it('does not call the bridge when the build has no disk sizes', async () => {
    const m = await load(false)
    const scope = effectScope()
    const s = scope.run(() => m.useDiskSizes(() => ['C:/a']))!
    await flush()
    expect(diskMapSizes).not.toHaveBeenCalled()
    expect(s.sizeOf('C:/a')).toBeNull()
    scope.stop()
  })

  it('batches the paths of several callers into one request and caches the answers', async () => {
    const m = await load(true)
    diskMapSizes.mockImplementation(answer('v1', { 'c:/p': 100, 'c:/p/src': 40 }))
    const scope = effectScope()
    const list = ref(['C:/p', 'C:/p/src'])
    const [a, b] = scope.run(() => [m.useDiskSizes(() => list.value), m.useDiskSizes(() => ['C:/P'])])!
    await flush()
    expect(diskMapSizes).toHaveBeenCalledTimes(1)
    expect(diskMapSizes.mock.calls[0]![0]).toHaveLength(2)
    expect(a.sizeOf('C:/p')).toBe(100)
    expect(b.sizeOf('c:/P')).toBe(100)
    expect(a.sizeOf('C:/p/src')).toBe(40)
    list.value = ['C:/p', 'C:/p/src']
    await flush()
    expect(diskMapSizes).toHaveBeenCalledTimes(1)
    list.value = ['C:/p', 'C:/p/new']
    await flush()
    expect(diskMapSizes).toHaveBeenCalledTimes(2)
    expect(diskMapSizes.mock.calls[1]![0]).toEqual(['C:/p/new'])
    expect(a.sizeOf('C:/p/new')).toBeNull()
    scope.stop()
  })

  it('splits more than the chunk size into several requests', async () => {
    const m = await load(true)
    diskMapSizes.mockImplementation(answer('v1', {}))
    const paths = Array.from({ length: m.SIZES_CHUNK + 5 }, (_, i) => `C:/d/${i}`)
    const scope = effectScope()
    scope.run(() => m.useDiskSizes(() => paths))
    await flush()
    expect(diskMapSizes).toHaveBeenCalledTimes(2)
    expect(diskMapSizes.mock.calls[0]![0]).toHaveLength(m.SIZES_CHUNK)
    scope.stop()
  })

  it('drops the cache when the index version changes and asks again', async () => {
    const m = await load(true)
    diskMapSizes.mockImplementationOnce(answer('v1', { 'c:/p': 100, 'c:/q': 7 }))
    const scope = effectScope()
    const list = ref(['C:/p'])
    const s = scope.run(() => m.useDiskSizes(() => list.value))!
    await flush()
    expect(s.sizeOf('C:/p')).toBe(100)
    diskMapSizes.mockImplementation(answer('v2', { 'c:/p': 90, 'c:/q': 7 }))
    list.value = ['C:/p', 'C:/q']
    await flush()
    await flush()
    expect(s.sizeOf('C:/q')).toBe(7)
    expect(diskMapSizes).toHaveBeenCalledTimes(3)
    expect(s.sizeOf('C:/p')).toBe(90)
    scope.stop()
  })

  it('resets on scan done and after refresh', async () => {
    const m = await load(true)
    diskMapSizes.mockImplementation(answer('v1', { 'c:/p': 100 }))
    const scope = effectScope()
    const s = scope.run(() => m.useDiskSizes(() => ['C:/p']))!
    await flush()
    expect(s.sizeOf('C:/p')).toBe(100)
    diskMapSizes.mockImplementation(answer('v2', { 'c:/p': 55 }))
    doneCb?.()
    await flush()
    expect(s.sizeOf('C:/p')).toBe(55)
    diskMapSizes.mockImplementation(answer('v3', { 'c:/p': 10 }))
    s.refresh()
    await flush()
    expect(s.sizeOf('C:/p')).toBe(10)
    scope.stop()
    expect(doneCb).toBeNull()
  })

  it('stops asking when there is no index until a scan finishes', async () => {
    const m = await load(true)
    diskMapSizes.mockResolvedValue(null)
    const scope = effectScope()
    const list = ref(['C:/p'])
    const s = scope.run(() => m.useDiskSizes(() => list.value))!
    await flush()
    expect(diskMapSizes).toHaveBeenCalledTimes(1)
    list.value = ['C:/p', 'C:/x']
    await flush()
    await flush()
    expect(diskMapSizes).toHaveBeenCalledTimes(1)
    expect(s.sizeOf('C:/p')).toBeNull()
    diskMapSizes.mockImplementation(answer('v1', { 'c:/p': 3 }))
    doneCb?.()
    await flush()
    expect(s.sizeOf('C:/p')).toBe(3)
    scope.stop()
  })

  it('ignores an answer that arrives after a reset', async () => {
    const m = await load(true)
    let release: (v: Sizes) => void = () => {}
    diskMapSizes.mockImplementationOnce(() => new Promise<Sizes>((r) => (release = r)))
    const scope = effectScope()
    const s = scope.run(() => m.useDiskSizes(() => ['C:/p']))!
    await flush()
    diskMapSizes.mockImplementation(answer('v2', { 'c:/p': 2 }))
    m.resetDiskSizes()
    await flush()
    release({ version: 'v1', sizes: { 'C:/p': 999 } })
    await flush()
    expect(s.sizeOf('C:/p')).toBe(2)
    scope.stop()
  })
})

describe('junkOf', () => {
  it('sums scanned items of the project and nothing when not scanned or scoped elsewhere', async () => {
    const m = await load(true)
    const items = [
      { path: 'C:/p/node_modules', size: 50, project: 'C:/p' },
      { path: 'C:/p/build', size: 25, project: 'C:/p' },
      { path: 'C:/q/node_modules', size: 9, project: 'C:/q' },
    ] as never
    expect(m.junkOf('C:/P', { scanned: true, project: null, items })).toBe(75)
    expect(m.junkOf('C:/p', { scanned: false, project: null, items })).toBeNull()
    expect(m.junkOf('C:/p', { scanned: true, project: 'C:/q', items })).toBeNull()
    expect(m.junkOf('C:/p', { scanned: true, project: 'c:/p/', items })).toBe(75)
    expect(m.junkOf('C:/z', { scanned: true, project: null, items })).toBeNull()
  })
})
