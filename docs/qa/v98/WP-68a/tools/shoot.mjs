import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { createRequire } from 'node:module'

const here = dirname(fileURLToPath(import.meta.url))
const root = join(here, '..', '..', '..', '..', '..')
const require = createRequire(join(root, 'package.json'))
const { webkit, chromium } = require('playwright')

const outDir = join(here, '..')
const page = pathToFileURL(join(here, '..', 'mockup', 'index.html')).href
mkdirSync(outDir, { recursive: true })

const VARIANTS = {
  'mist-light': ['#pal-mist', '#sc-light', '#gl-on'],
  'mist-dark': ['#pal-mist', '#sc-dark', '#gl-on'],
  'nebula-light': ['#pal-nebula', '#sc-light', '#gl-on'],
  'nebula-dark': ['#pal-nebula', '#sc-dark', '#gl-on'],
  'galaxy-dark': ['#pal-galaxy', '#sc-dark', '#gl-on'],
  'mist-light-reduced': ['#pal-mist', '#sc-light', '#gl-reduced'],
  'mist-dark-reduced': ['#pal-mist', '#sc-dark', '#gl-reduced'],
}

const only = process.argv.slice(2).find((a) => a.startsWith('--only='))?.slice(7)
const plan = [
  { name: 'webkit', type: webkit, launch: {}, variants: ['mist-light', 'mist-dark', 'nebula-light', 'nebula-dark', 'galaxy-dark', 'mist-light-reduced'] },
  { name: 'chromium', type: chromium, launch: { channel: process.env.PW_CHANNEL ?? 'msedge' }, variants: ['mist-light', 'mist-dark'] },
].filter((p) => !only || p.name === only)
const vArg = process.argv.slice(2).find((a) => a.startsWith('--variants='))?.slice(11)
if (vArg) for (const p of plan) p.variants = p.variants.filter((v) => vArg.split(',').includes(v))

let count = 0
for (const b of plan) {
  const browser = await b.type.launch(b.launch)
  const ctx = await browser.newContext({ viewport: { width: 1700, height: 1300 }, deviceScaleFactor: 2 })
  const tab = await ctx.newPage()
  await tab.goto(page)
  await tab.evaluate(() => document.fonts.ready)
  for (const v of b.variants) {
    for (const sel of VARIANTS[v]) await tab.$eval(sel, (el) => { el.checked = true })
    await tab.waitForTimeout(150)
    const shots = await tab.$$('[data-shot]')
    for (const el of shots) {
      const id = await el.getAttribute('data-shot')
      await el.screenshot({ path: join(outDir, `${id}-${v}-${b.name}.jpg`), type: 'jpeg', quality: 88, scale: id.endsWith('-ipad') ? 'css' : 'device', animations: 'disabled' })
      count++
    }
  }
  await browser.close()
}
console.log(`ok ${count} shots`)
