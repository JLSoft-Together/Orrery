import { describe, expect, it } from 'vitest'
import { RemoteCommandError, type JsonMap } from '../remote/types'
import { currentStamp, isOwnChange, OWN_WRITE_MS, touchesPath, unwatchRoot, watchRoot } from './diskWatch'

describe('file changed on disk while editing', () => {
  it('matches only fs.changed events that list the open file, ignoring case and slashes', () => {
    const event = { t: 'fs.changed', root: 'D:/Code/api', paths: [String.raw`D:\Code\API\lib\key.dart`] }
    expect(touchesPath(event, 'd:/code/api/lib/key.dart')).toBe(true)
    expect(touchesPath({ ...event, paths: ['D:/Code/api/other.dart'] }, 'D:/Code/api/lib/key.dart')).toBe(false)
    expect(touchesPath({ ...event, paths: [] }, 'D:/Code/api/lib/key.dart')).toBe(false)
    expect(touchesPath({ ...event, t: 'git.changed' }, 'D:/Code/api/lib/key.dart')).toBe(false)
    expect(touchesPath({ t: 'fs.changed' }, 'x')).toBe(false)
  })

  it('treats a change during or just after our own save as ours', () => {
    expect(isOwnChange(true, null, 1000)).toBe(true)
    expect(isOwnChange(false, 1000, 1000 + OWN_WRITE_MS - 1)).toBe(true)
    expect(isOwnChange(false, 1000, 1000 + OWN_WRITE_MS)).toBe(false)
    expect(isOwnChange(false, null, 5000)).toBe(false)
  })

  it('watches and unwatches the root without throwing when the desktop refuses', async () => {
    const calls: Array<[string, JsonMap | undefined]> = []
    const run = async (type: string, fields?: JsonMap): Promise<JsonMap> => {
      calls.push([type, fields])
      throw new RemoteCommandError('limit')
    }
    watchRoot(run, '/r')
    unwatchRoot(run, '/r')
    await Promise.resolve()
    expect(calls).toEqual([
      ['fs.watch', { root: '/r' }],
      ['fs.unwatch', { root: '/r' }],
    ])
  })

  it('reads the new version stamp so Keep mine can save over the disk copy', async () => {
    const run = async (): Promise<JsonMap> => ({ data: { kind: 'text', path: '/a', size: 1, mtimeMs: 1810, text: 'x' } })
    expect(await currentStamp(run, '/a')).toBe(1810)
  })
})
