import { describe, expect, it } from 'vitest'
import { canRestartWithResume, draftOf, isUnused, mcpCalls, pluginCalls, toolKey, unusedDraft, errorKey, filterRows, groupTarget, pendingChanges, setDraft, setGroup, showFilter, shownEnabled, sortMcp, sortPlugins, toolsEntryVisible, withChanges, withEnabled } from './sessionTools'

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

  it('shows the filter past 8 rows', () => {
    expect(showFilter(4, 4)).toBe(false)
    expect(showFilter(5, 4)).toBe(true)
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

describe('sessionTools draft', () => {
  const mcp = [
    { name: 'orrery', scope: 'user' as const, enabled: true, locked: true as const },
    { name: 'a', scope: 'user' as const, enabled: true },
    { name: 'b', scope: 'local' as const, enabled: false },
  ]
  const plugins = [{ name: 'p@m', scope: 'user' as const, enabled: true }]

  it('keeps only real differences from the saved state', () => {
    let d = setDraft({}, 'mcp', mcp[1]!, false)
    expect(d).toEqual({ 'mcp:a': false })
    expect(shownEnabled(d, 'mcp', mcp[1]!)).toBe(false)
    d = setDraft(d, 'mcp', mcp[1]!, true)
    expect(d).toEqual({})
    d = setDraft(setDraft(d, 'plugin', plugins[0]!, false), 'mcp', mcp[2]!, true)
    expect(pendingChanges(d, mcp, plugins)).toEqual([
      { kind: 'mcp', name: 'b', enabled: true },
      { kind: 'plugin', name: 'p@m', enabled: false },
    ])
  })

  it('drops draft entries that the reloaded list already matches or no longer has', () => {
    const d = { 'mcp:a': false, 'mcp:gone': true, 'mcp:orrery': false }
    expect(pendingChanges(d, [{ ...mcp[1]!, enabled: false }, mcp[0]!], [])).toEqual([])
  })

  it('bulk switch skips the locked row and turns all on when one is off', () => {
    expect(groupTarget({}, 'mcp', mcp)).toBe(true)
    const on = setGroup({}, 'mcp', mcp, true)
    expect(on).toEqual({ 'mcp:b': true })
    expect(groupTarget(on, 'mcp', mcp)).toBe(false)
    expect(setGroup(on, 'mcp', mcp, false)).toEqual({ 'mcp:a': false })
    expect(groupTarget({}, 'mcp', [mcp[0]!])).toBeNull()
  })

  it('applies saved changes to rows and rebuilds a draft from them', () => {
    const changes = [{ kind: 'mcp' as const, name: 'a', enabled: false }]
    expect(withChanges(mcp, 'mcp', changes).map((r) => r.enabled)).toEqual([true, false, false])
    expect(withChanges(plugins, 'plugin', changes)).toEqual(plugins)
    expect(draftOf(changes)).toEqual({ 'mcp:a': false })
  })
})

describe('sessionTools usage', () => {
  const usage = { ok: true as const, total: 9, servers: { figma: 3, claude_ai_Gmail: 1, plugin_github_github: 2, 'claude-in-chrome': 1 }, namespaces: { github: 1, caveman: 4 } }

  it('maps MCP server names to the tool name Claude Code builds', () => {
    expect(toolKey('claude.ai Gmail')).toBe('claude_ai_Gmail')
    expect(toolKey('plugin:github:github')).toBe('plugin_github_github')
    expect(mcpCalls(usage, 'claude.ai Gmail')).toBe(1)
    expect(mcpCalls(usage, 'plugin:github:github')).toBe(2)
    expect(mcpCalls(usage, 'claude-in-chrome')).toBe(1)
    expect(mcpCalls(usage, 'meshy')).toBe(0)
  })

  it('adds plugin skills / agents and its own MCP servers', () => {
    expect(pluginCalls(usage, 'github@claude-plugins-official')).toBe(3)
    expect(pluginCalls(usage, 'caveman@x')).toBe(4)
    expect(pluginCalls(usage, 'figma@x')).toBe(0)
  })

  it('marks unused MCP rows only when the session already called tools', () => {
    const rows = [
      { name: 'orrery', scope: 'user' as const, enabled: true, locked: true as const },
      { name: 'figma', scope: 'user' as const, enabled: true },
      { name: 'meshy', scope: 'user' as const, enabled: true },
      { name: 'off', scope: 'user' as const, enabled: false },
    ]
    expect(rows.map((r) => isUnused(usage, r))).toEqual([false, false, true, false])
    expect(isUnused({ ...usage, total: 0 }, rows[2]!)).toBe(false)
    expect(isUnused(null, rows[2]!)).toBe(false)
    expect(unusedDraft({ 'mcp:figma': false }, usage, rows)).toEqual({ 'mcp:figma': false, 'mcp:meshy': false })
  })
})
