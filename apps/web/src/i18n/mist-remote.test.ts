import { describe, expect, it } from 'vitest'
import { createI18n } from 'vue-i18n'
import remote from './mist-remote'

type Tree = { [key: string]: Tree | string }

const leaves = (tree: Tree, prefix = ''): string[] => Object.entries(tree).flatMap(([k, v]) => (typeof v === 'string' ? [`${prefix}${k}`] : leaves(v, `${prefix}${k}.`)))

describe('phone status messages in UI next', () => {
  it('have the same keys in both languages', () => {
    expect(leaves(remote.vi as unknown as Tree)).toEqual(leaves(remote.en as unknown as Tree))
  })

  for (const locale of ['en', 'vi'] as const) {
    it(`compile in ${locale}, keep every placeholder and avoid em dash and middle dot`, () => {
      const i18n = createI18n({ legacy: false, locale, messages: { [locale]: remote[locale] }, missingWarn: false, fallbackWarn: false })
      for (const key of leaves(remote[locale] as unknown as Tree)) {
        for (const n of [1, 3]) {
          const out = i18n.global.t(key, { n, total: 4, on: 2 }, n)
          expect(out, key).not.toMatch(/\{\w+\}/)
          expect(out, key).not.toMatch(/[—·]/)
        }
      }
    })
  }
})
