import { describe, expect, it } from 'vitest'
import { createI18n } from 'vue-i18n'
import remote from './remote-v97'

type Tree = { [key: string]: Tree | string }

const leaves = (tree: Tree, prefix = ''): string[] => Object.entries(tree).flatMap(([k, v]) => (typeof v === 'string' ? [`${prefix}${k}`] : leaves(v, `${prefix}${k}.`)))

describe('remote control messages', () => {
  it('have the same keys in both languages', () => {
    expect(leaves(remote.vi as unknown as Tree)).toEqual(leaves(remote.en as unknown as Tree))
  })

  for (const locale of ['en', 'vi'] as const) {
    it(`compile in ${locale} and keep every placeholder`, () => {
      const i18n = createI18n({ legacy: false, locale, messages: { [locale]: remote[locale] }, missingWarn: false, fallbackWarn: false })
      for (const key of leaves(remote[locale] as unknown as Tree)) {
        const out = i18n.global.t(key, { name: 'NAME', device: 'DEV', text: 'TXT', when: 'WHEN', time: '1:00', n: 2, port: 4320, key: 'esc' })
        expect(out, key).not.toMatch(/\{\w+\}/)
      }
    })
  }
})
