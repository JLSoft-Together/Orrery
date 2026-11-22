import { describe, expect, it } from 'vitest'
import en from './en'
import vi from './vi'

type Tree = { [key: string]: Tree | string }

function leaves(tree: Tree, prefix = ''): string[] {
  return Object.entries(tree).flatMap(([k, v]) => (typeof v === 'string' ? [`${prefix}${k}`] : leaves(v, `${prefix}${k}.`)))
}

function keyList(messages: unknown): string {
  return leaves(messages as Tree).sort().join('\n') + '\n'
}

describe('i18n key snapshot', () => {
  it('en keys are unchanged', async () => {
    await expect(keyList(en)).toMatchFileSnapshot('./__snapshots__/keys.en.txt')
  })
  it('vi keys are unchanged', async () => {
    await expect(keyList(vi)).toMatchFileSnapshot('./__snapshots__/keys.vi.txt')
  })
})
