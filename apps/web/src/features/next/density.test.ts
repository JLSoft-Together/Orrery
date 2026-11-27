import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const read = (rel: string): string => readFileSync(new URL(rel, import.meta.url), 'utf8')
const css = read('./density.css')

function block(selector: string): Record<string, number> {
  const start = css.indexOf(`${selector} {`)
  expect(start).toBeGreaterThanOrEqual(0)
  const body = css.slice(start, css.indexOf('}', start))
  const out: Record<string, number> = {}
  for (const m of body.matchAll(/--ccm-density-([a-z-]+):\s*(\d+)px/g)) out[m[1]!] = Number(m[2])
  return out
}

const regular = block(':root')
const compact = block(":root[data-ui='next'][data-density='compact']")
const comfy = block(":root[data-ui='next'][data-density='comfy']")

describe('density tokens', () => {
  it('regular keeps the pixel values that used to be hard-coded', () => {
    expect(regular).toEqual({ 'row-h': 48, 'row-py': 6, gap: 8, 'head-h': 34, 'pal-h': 52, 'set-py': 8 })
  })

  it('compact is smaller and comfy is larger than regular on every token', () => {
    for (const key of Object.keys(regular)) {
      expect(compact[key]).toBeLessThan(regular[key]!)
      expect(comfy[key]).toBeGreaterThan(regular[key]!)
    }
  })

  it('has no transition or animation, so a change is instant', () => {
    expect(css).not.toMatch(/transition|animation/)
  })

  it('is imported once from tokens.css', () => {
    expect(read('./tokens.css').match(/@import '\.\/density\.css';/g)).toHaveLength(1)
  })
})

describe('components read the tokens', () => {
  it('rail row', () => {
    const src = read('./components/rail/RailRowItem.vue')
    expect(src).toContain('min-h-(--ccm-density-row-h)')
    expect(src).toContain('py-(--ccm-density-row-py)')
    expect(src).toContain('gap-x-(--ccm-density-gap)')
    expect(src).not.toContain('min-h-[48px]')
  })

  it('pane header', () => {
    const src = read('./components/ui/PaneHeader.vue')
    expect(src).toContain('h-(--ccm-density-head-h)')
    expect(src).not.toContain('h-[34px]')
  })

  it('palette row and settings rows', () => {
    expect(read('../palette/components/CommandPalette.vue')).toContain('min-h-(--ccm-density-pal-h)')
    expect(read('../settings/components/SettingRow.vue')).toContain('py-(--ccm-density-set-py)')
    expect(read('../../core/components/ToggleSwitch.vue')).toContain('py-(--ccm-density-set-py)')
  })

  it('terminal and editor code are untouched by density', () => {
    expect(read('../workspace/components/TermPane.vue')).not.toContain('--ccm-density')
  })
})
