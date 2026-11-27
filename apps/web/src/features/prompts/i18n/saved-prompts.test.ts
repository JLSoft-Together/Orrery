import { describe, expect, it } from 'vitest'
import messages from './saved-prompts'

type Tree = { [key: string]: Tree | string }

function leaves(tree: Tree, prefix = ''): Record<string, string> {
  return Object.fromEntries(Object.entries(tree).flatMap(([k, v]) => (typeof v === 'string' ? [[`${prefix}${k}`, v]] : Object.entries(leaves(v, `${prefix}${k}.`)))))
}

const holes = (s: string): string[] => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]!).sort()

describe('saved prompts messages', () => {
  const en = leaves(messages.en)
  const vi = leaves(messages.vi)

  it('has the same keys in en and vi', () => {
    expect(Object.keys(vi).sort()).toEqual(Object.keys(en).sort())
  })
  it('uses the same placeholders in both languages', () => {
    for (const key of Object.keys(en)) expect(holes(vi[key]!), key).toEqual(holes(en[key]!))
  })
  it('has no key with a dot and no em dash or middle dot in the text', () => {
    for (const [key, text] of [...Object.entries(en), ...Object.entries(vi)]) {
      expect(key.split('.').every((part) => part && !part.includes('.'))).toBe(true)
      expect(text, key).not.toMatch(/—|·/)
    }
  })
  it('covers every error code the core and the file dialogs can return', () => {
    for (const code of ['invalid-op', 'invalid-id', 'invalid-title', 'invalid-body', 'invalid-scope', 'too-many', 'not-found', 'io', 'unavailable', 'invalid', 'too-large', 'read', 'write', 'failed']) {
      expect(en[`savedPrompts.error.${code}`], code).toBeTruthy()
    }
  })
})
