import { describe, expect, it } from 'vitest'
import { canRestartWithResume, errorKey, filterRows, showFilter, sortMcp, sortPlugins, toolsEntryVisible, withEnabled } from './sessionTools'

describe('sessionTools', () => {
  it('puts the locked row first, then by scope and name', () => {
    const rows = sortMcp([
      { name: 'claude.ai Drive', scope: 'connector', enabled: true },
      { name: 'b', scope: 'user', enabled: true },
      { name: 'a', scope: 'user', enabled: false },
      { name: 'orrery', scope: 'user', enabled: true, locked: true },
      { name: 'p', scope: 'project', enabled: true },
    ])
    expect(rows.map((r) => r.name)).toEqual(['orrery', 'p', 'a', 'b', 'claude.ai Drive'])
  })

  it('sorts plugins by name', () => {
    expect(sortPlugins([{ name: 'z', enabled: true, scope: 'user' }, { name: 'a', enabled: true, scope: 'project' }]).map((r) => r.name)).toEqual(['a', 'z'])
  })

  it('filters case-insensitively and keeps all rows for a blank query', () => {
    const rows = [{ name: 'Figma' }, { name: 'github' }]
    expect(filterRows(rows, ' FIG ')).toEqual([{ name: 'Figma' }])
    expect(filterRows(rows, '')).toHaveLength(2)
  })

  it('shows the filter past 12 rows', () => {
    expect(showFilter(6, 6)).toBe(false)
    expect(showFilter(7, 6)).toBe(true)
  })

  it('flips one row without touching the input', () => {
    const rows = [{ name: 'a', enabled: true }, { name: 'b', enabled: true }]
    expect(withEnabled(rows, 'b', false)).toEqual([{ name: 'a', enabled: true }, { name: 'b', enabled: false }])
    expect(rows[1]?.enabled).toBe(true)
  })

  it('maps error codes to i18n keys', () => {
    expect(errorKey('conflict')).toBe('sessionTools.error.conflict')
  })

  it('restarts only a running idle pane', () => {
    expect(canRestartWithResume({ state: 'running' }, 'idle')).toBe(true)
    expect(canRestartWithResume({ state: 'running' }, 'working')).toBe(false)
    expect(canRestartWithResume({ state: 'running' }, 'waiting')).toBe(false)
    expect(canRestartWithResume({ state: 'exited' }, 'idle')).toBe(false)
    expect(canRestartWithResume(undefined, 'idle')).toBe(false)
  })

  it('shows the entry for a live Claude session with a folder and the API', () => {
    const ok = { hasApi: true, hasCwd: true, claude: true, ended: false }
    expect(toolsEntryVisible(ok)).toBe(true)
    expect(toolsEntryVisible({ ...ok, hasApi: false })).toBe(false)
    expect(toolsEntryVisible({ ...ok, hasCwd: false })).toBe(false)
    expect(toolsEntryVisible({ ...ok, claude: false })).toBe(false)
    expect(toolsEntryVisible({ ...ok, ended: true })).toBe(false)
  })
})
