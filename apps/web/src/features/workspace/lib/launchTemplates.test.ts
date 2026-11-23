import { describe, expect, it } from 'vitest'
import { chooseFor, cleanDefaults, cleanTemplate, cleanTemplates, isModelId, launchOptionsOf, rememberChoice, templateArgsPreview, type LaunchTemplate } from './launchTemplates'

const plan: LaunchTemplate = { id: 'a', name: 'Plan', agent: 'claude', model: 'opus', permissionMode: 'plan', effort: 'high', prompt: 'read the code' }
const sandbox: LaunchTemplate = { id: 'b', name: 'Read', agent: 'codex', permissionMode: 'read-only' }

describe('launch templates', () => {
  it('keeps only whitelisted values', () => {
    expect(cleanTemplate({ ...plan, permissionMode: 'bypassPermissions', effort: 'turbo', model: 'a; rm -rf /' })).toEqual({ id: 'a', name: 'Plan', agent: 'claude', prompt: 'read the code' })
    expect(cleanTemplate({ ...sandbox, effort: 'high' })).toEqual(sandbox)
    expect(cleanTemplate({ ...plan, agent: 'shell' })).toBeNull()
    expect(cleanTemplate({ ...plan, name: '  ' })).toBeNull()
    expect(cleanTemplates([plan, plan, 5, { ...sandbox, id: 'c' }]).map((t) => t.id)).toEqual(['a', 'c'])
    expect(cleanTemplates('x')).toEqual([])
  })

  it('validates model ids like the host', () => {
    expect(['opus', 'claude-opus-4.1', 'gpt-5.1', 'o3:high'].every(isModelId)).toBe(true)
    expect(['', '-x', 'a b', 'a;b', '$(x)', 'a'.repeat(65), 5].some(isModelId)).toBe(false)
  })

  it('builds host options and a readable preview', () => {
    expect(launchOptionsOf(plan)).toEqual({ model: 'opus', permission: 'plan', effort: 'high' })
    expect(launchOptionsOf({ id: 'z', name: 'n', agent: 'claude' })).toBeUndefined()
    expect(templateArgsPreview(plan)).toBe('claude --model opus --permission-mode plan --effort high')
    expect(templateArgsPreview(sandbox)).toBe('codex -s read-only')
  })

  it('picks the default of the project, else the fallback', () => {
    const defaults = cleanDefaults({ 'c:\\repo': { agent: 'claude', template: 'a' }, 'c:\\gone': { agent: 'codex', template: 'x' }, bad: { agent: 'nope' } })
    expect(Object.keys(defaults)).toEqual(['c:\\repo', 'c:\\gone'])
    expect(chooseFor('C:/Repo/', defaults, [plan, sandbox], 'shell')).toEqual({ agent: 'claude', template: plan })
    expect(chooseFor('C:/gone', defaults, [plan, sandbox], 'shell')).toEqual({ agent: 'codex', template: null })
    expect(chooseFor('C:/new', defaults, [plan], 'codex')).toEqual({ agent: 'codex', template: null })
    expect(chooseFor(undefined, defaults, [plan], 'claude')).toEqual({ agent: 'claude', template: null })
    expect(chooseFor('C:/repo', defaults, [plan], 'shell', (p) => p !== 'claude')).toEqual({ agent: 'shell', template: null })
  })

  it('remembers the last choice per project and caps the map', () => {
    const next = rememberChoice({}, 'C:/Repo', { agent: 'claude', template: plan })
    expect(next).toEqual({ 'c:\\repo': { agent: 'claude', template: 'a' } })
    expect(rememberChoice(next, 'c:\\repo\\', { agent: 'codex', template: null })).toEqual({ 'c:\\repo': { agent: 'codex' } })
    let many = {}
    for (let i = 0; i < 60; i++) many = rememberChoice(many, `C:/p${i}`, { agent: 'claude', template: null })
    expect(Object.keys(many)).toHaveLength(50)
    expect(Object.keys(many).at(-1)).toBe('c:\\p59')
  })
})
