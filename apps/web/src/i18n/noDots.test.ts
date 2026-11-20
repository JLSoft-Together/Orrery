import { describe, expect, it } from 'vitest'
import en from './en'
import vi from './vi'

type Tree = { [key: string]: Tree | string }

const values = (tree: Tree): string[] => Object.values(tree).flatMap((v) => (typeof v === 'string' ? [v] : values(v)))

describe('message separators', () => {
  for (const [name, messages] of [['en', en], ['vi', vi]] as const) {
    it(`${name} messages have no middle dot or em dash`, () => {
      const bad = values(messages as unknown as Tree).filter((v) => /[·—]/.test(v))
      expect(bad).toEqual([])
    })
  }
})
