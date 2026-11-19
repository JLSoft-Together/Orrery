import { join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

const BLURRED = `() => {
  const out = []
  for (const el of document.querySelectorAll('body *')) {
    const cs = getComputedStyle(el)
    const v = cs.backdropFilter || cs.webkitBackdropFilter || 'none'
    if (v !== 'none') out.push(el.tagName.toLowerCase() + '.' + String(el.className).split(' ')[0])
  }
  return out
}`

const ALPHA = `(color) => {
  const m = color.match(/^rgba?\\(([^)]+)\\)$/)
  if (m) {
    const p = m[1].split(/[ ,/]+/).filter(Boolean)
    return p.length > 3 ? Number(p[3]) : 1
  }
  const c = color.match(/^color\\(.+\\/\\s*([\\d.]+%?)\\)$/)
  if (c) return c[1].endsWith('%') ? Number(c[1].slice(0, -1)) / 100 : Number(c[1])
  return color.startsWith('color(') || color.startsWith('oklab(') || color.startsWith('lab(') ? 1 : 0
}`

const PALETTES = [
  ['light', 'mist'],
  ['dark', 'mist'],
  ['dark', 'galaxy'],
]

async function skin(page, scheme, palette) {
  await page.emulateMedia({ colorScheme: scheme })
  await page.evaluate(([s, p]) => {
    document.documentElement.dataset.nextTheme = p
    document.documentElement.classList.toggle('dark', s === 'dark')
  }, [scheme, palette])
  await wait(350)
}

export async function mistWorkspaceSteps(c, page, peer) {
  const { step, byId, env, browserName, shoot, noOverflow, text } = c
  const id = peer.info.machine
  const webkit = browserName === 'webkit'
  const file = (name, form, scheme, palette) => join(env.qaDirG, `${name}-${form}-${palette}-${scheme}-${browserName}.png`)
  const live = () => page.waitForFunction(() => document.querySelector('[data-testid="terminal"]')?.getAttribute('data-state') === 'live', null, { timeout: 12000 })
  const variants = webkit ? PALETTES : [PALETTES[0]]
  const frames = (ms) =>
    page.evaluate(
      (span) =>
        new Promise((resolve) => {
          const marks = []
          let last = performance.now()
          const start = last
          const tick = (now) => {
            marks.push(now - last)
            last = now
            if (now - start < span) requestAnimationFrame(tick)
            else {
              const sorted = marks.slice(1).sort((a, b) => a - b)
              resolve({ n: sorted.length, p50: sorted[Math.floor(sorted.length * 0.5)] ?? 0, p95: sorted[Math.floor(sorted.length * 0.95)] ?? 0, max: sorted[sorted.length - 1] ?? 0 })
            }
          }
          requestAnimationFrame(tick)
        }),
      ms,
    )

  await step('Workspace groups from tabs[].groups: project label per group (tone, name), chips unchanged, grid sections fold, tablist shows attention count', async () => {
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    peer.send('groups')
    await go(page, `/app/m/${id}/ws`)
    await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
    if ((await byId(page, 'workspace-view').getAttribute('data-mode')) !== 'focus') await byId(page, 'ws-mode').click()
    await byId(page, 'ws-focus').waitFor()
    await page.waitForFunction(() => document.querySelectorAll('[data-testid^="chip-group-"]').length === 3, null, { timeout: 8000 })
    if ((await page.locator('[data-testid="ws-chips"] > button').count()) !== 3) throw new Error('chip count changed by groups')
    const names = await page.locator('[data-testid^="chip-group-"]').evaluateAll((els) => els.map((e) => e.textContent.trim()))
    if (names.join(',') !== 'Aapi,Wweb,Iinfra') throw new Error(`group labels ${names}`)
    const tones = await page.locator('[data-testid^="chip-group-"] .tone-mono').evaluateAll((els) => els.map((e) => e.getAttribute('data-tone')))
    if (tones.join(',') !== '0,2,4') throw new Error(`tones ${tones}`)
    const colours = await page.locator('[data-testid^="chip-group-"] .tone-mono').evaluateAll((els) => els.map((e) => getComputedStyle(e).color))
    if (new Set(colours).size !== 3) throw new Error(`tone colours not distinct ${colours}`)
    const attn = byId(page, 'ws-attn-tab1')
    if ((await attn.count()) > 0 && !/^\d+$/.test((await text(attn)).trim())) throw new Error('attention badge text')
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await noOverflow(page, `ws-a groups ${scheme}`)
      await shoot(page, file('ws-a-groups', 'phone', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    await byId(page, 'ws-mode').click()
    await byId(page, 'ws-grid').waitFor()
    const sections = page.locator('section[data-testid^="ws-group-"]')
    if ((await sections.count()) !== 3) throw new Error('grid sections')
    const folded = await sections.evaluateAll((els) => els.map((e) => e.getAttribute('data-folded')))
    if (folded.join(',') !== '0,0,1') throw new Error(`folded ${folded}`)
    await byId(page, 'pane-p3').waitFor({ state: 'hidden', timeout: 3000 })
    const toggle = page.locator('[data-testid^="ws-group-toggle-"]').nth(2)
    if ((await toggle.getAttribute('aria-expanded')) !== 'false') throw new Error('aria-expanded of a folded group')
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('ws-b-groups', 'phone', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    await toggle.click()
    await byId(page, 'pane-p3').waitFor({ state: 'visible', timeout: 3000 })
    await toggle.click()
    await byId(page, 'pane-p3').waitFor({ state: 'hidden', timeout: 3000 })
    await page.locator('[data-testid^="ws-group-toggle-"]').first().click()
    await byId(page, 'pane-p1').waitFor({ state: 'hidden', timeout: 3000 })
    await page.locator('[data-testid^="ws-group-toggle-"]').first().click()
    await byId(page, 'pane-p1').waitFor({ state: 'visible', timeout: 3000 })
    await byId(page, 'more-p1').waitFor()
    await byId(page, 'ws-mode').click()
    await byId(page, 'ws-focus').waitFor()
    peer.send('ungroups')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid^="chip-group-"]').length === 0, null, { timeout: 8000 })
  })

  await step('Terminal dock: keys Esc Tab Ctrl Alt ⇧Tab ^C arrows Enter on a solid dock, no blur anywhere on the terminal page, 64 KiB burst keeps painting', async () => {
    await page.setViewportSize(PHONE)
    await go(page, `/app/m/${id}/p/p1`)
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    await live()
    for (const key of ['esc', 'tab', 'ctrl', 'alt', 'shiftTab', 'ctrlC', 'up', 'down', 'left', 'right', 'enter']) {
      if ((await byId(page, `key-${key}`).count()) !== 1) throw new Error(`key ${key} missing`)
      const box = await byId(page, `key-${key}`).boundingBox()
      if (!box || box.height < 39.5 || box.width < 43.5) throw new Error(`key ${key} too small ${JSON.stringify(box)}`)
    }
    const dock = byId(page, 'dock')
    const blur = await dock.evaluate((el) => getComputedStyle(el).backdropFilter || 'none')
    if (blur !== 'none') throw new Error(`dock blur ${blur}`)
    for (const [scheme, palette] of PALETTES) {
      await skin(page, scheme, palette)
      const alpha = await dock.evaluate(new Function('el', `return (${ALPHA})(getComputedStyle(el).backgroundColor)`))
      if (alpha < 1) throw new Error(`dock is translucent (${alpha}) in ${scheme} ${palette}`)
      const xtermBg = await page.locator('.xterm').first().evaluate(new Function('el', `let n = el; while (n && (${ALPHA})(getComputedStyle(n).backgroundColor) < 1) n = n.parentElement; return !!n`))
      if (!xtermBg) throw new Error('terminal has no opaque background')
      const blurred = await page.evaluate(new Function(`return (${BLURRED})()`))
      if (blurred.length) throw new Error(`blur on the terminal page: ${blurred.join(',')}`)
    }
    await skin(page, 'light', 'mist')
    await noOverflow(page, 'terminal phone')
    const before = await page.evaluate(new Function(`return (${BLURRED})()`))
    peer.send('big')
    const burst = await frames(1800)
    const after = await page.evaluate(new Function(`return (${BLURRED})()`))
    if (before.join() !== after.join()) throw new Error('blur layers changed while the terminal printed')
    c.perf = c.perf ?? {}
    c.perf.terminalBurst = burst
    if (burst.p50 > 60) throw new Error(`terminal burst median frame ${burst.p50} ms`)
    const csp = await page.evaluate(() => window.__csp.filter((x) => !/stylesheet/.test(x)))
    if (csp.length) throw new Error(`CSP: ${csp.join(' | ')}`)
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('term', 'phone', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    await byId(page, 'open-viewer').click()
    await byId(page, 'viewer').waitFor()
    for (const seg of ['files', 'diff', 'git', 'run']) {
      await byId(page, `viewer-seg-${seg}`).click()
      await wait(550)
      const blurred = await page.evaluate(new Function(`return (${BLURRED})()`))
      if (blurred.length) throw new Error(`blur in viewer ${seg}: ${blurred.join(',')}`)
      await noOverflow(page, `viewer ${seg}`)
      for (const [scheme, palette] of variants.slice(0, 2)) {
        await skin(page, scheme, palette)
        await shoot(page, file(`viewer-${seg}`, 'phone', scheme, palette))
      }
      await skin(page, 'light', 'mist')
    }
    await byId(page, 'viewer-close').click()
    await go(page, `/app/m/${id}/ws`)
    await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
  })

  await step('iPad rail: replaces the tab bar, machine + project-indented sessions + workspace panes, collapses and remembers it, phone keeps the floating tab bar, no blur on lists, scroll stays smooth', async () => {
    await page.setViewportSize(IPAD)
    await page.emulateMedia({ colorScheme: 'light' })
    await go(page, `/app/m/${id}`)
    await byId(page, 'ipad-rail').waitFor({ timeout: 8000 })
    if ((await byId(page, 'tab-bar').count()) !== 0) throw new Error('tab bar on iPad')
    if ((await byId(page, 'ipad-rail').getAttribute('data-collapsed')) !== '0') throw new Error('rail starts collapsed')
    if (!(await c.text(byId(page, 'rail-machine'))).length) throw new Error('machine name missing')
    if ((await byId(page, 'rail-seg-monitor').getAttribute('aria-selected')) !== 'true') throw new Error('monitor segment')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid^="rail-session-"]').length === 4, null, { timeout: 8000 })
    const head = await page.locator('[data-testid^="rail-group-"] .gh .tone-mono').first().boundingBox()
    const row = await byId(page, 'rail-session-t1').boundingBox()
    if (!head || !row || !(row.x > head.x)) throw new Error(`sessions are not indented under the project ${JSON.stringify({ head, row })}`)
    if ((await byId(page, 'rail-session-t1').locator('svg').count()) < 1) throw new Error('row status icon missing')
    if (!/Chờ bạn/.test(await c.text(byId(page, 'rail-session-t1'))) && !/Chờ bạn/.test(await c.text(page.locator('[data-rail="asked"]').first()))) throw new Error('row status text')
    if ((await byId(page, 'rail-quota').count()) !== 1) throw new Error('rail quota missing with limits')
    await noOverflow(page, 'ipad monitor')
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('rail-monitor', 'ipad', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    await byId(page, 'rail-session-t2').click()
    await byId(page, 'detail-pane').locator('[data-testid="session-view"]').waitFor({ timeout: 8000 })
    if ((await byId(page, 'rail-session-t2').getAttribute('aria-current')) !== 'true') throw new Error('selected row not marked')
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('detail', 'ipad', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    peer.send('groups')
    await byId(page, 'rail-seg-workspace').click()
    await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
    await page.waitForFunction(() => document.querySelectorAll('[data-testid^="rail-pgroup-"]').length === 3, null, { timeout: 8000 })
    const tab = await page.locator('[data-testid="rail-tab-tab1"] .gh .tone-mono').first().boundingBox()
    const sub = await page.locator('[data-testid^="rail-pgroup-"] .sh .dotc').first().boundingBox()
    const pane = await byId(page, 'rail-pane-p1').boundingBox()
    if (!tab || !sub || !pane || !(sub.x > tab.x) || !(pane.x > sub.x - 2)) throw new Error(`workspace rail is not indented tab > project > pane ${JSON.stringify({ tab, sub, pane })}`)
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('rail-workspace', 'ipad', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    const mode = await byId(page, 'workspace-view').getAttribute('data-mode')
    if (mode !== 'grid') await byId(page, 'ws-mode').click()
    await byId(page, 'ws-grid').waitFor()
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('ws-b-groups', 'ipad', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    await byId(page, 'ws-mode').click()
    await byId(page, 'ws-focus').waitFor()
    await byId(page, 'rail-pane-p1').click()
    await byId(page, 'detail-pane').locator('[data-testid="terminal-view"]').waitFor({ timeout: 8000 })
    await live()
    peer.send('demo')
    await wait(700)
    for (const [scheme, palette] of variants) {
      await skin(page, scheme, palette)
      await shoot(page, file('term', 'ipad', scheme, palette))
    }
    await skin(page, 'light', 'mist')
    const blurred = await page.evaluate(new Function(`return (${BLURRED})()`))
    if (blurred.some((b) => !/^(nav|header)\.(rail|glass|bar)/.test(b) && !/rail/.test(b))) throw new Error(`blur outside the rail: ${blurred.join(',')}`)
    if (blurred.length > 2) throw new Error(`more than two blur layers (rail + top bar) on the iPad terminal: ${blurred.join(',')}`)
    peer.send('ungroups')
    await byId(page, 'rail-collapse').click()
    await page.waitForFunction(() => document.querySelector('[data-testid="ipad-rail"]')?.getAttribute('data-collapsed') === '1')
    await wait(400)
    const width = (await byId(page, 'ipad-rail').boundingBox())?.width ?? 999
    if (width > 90) throw new Error(`collapsed rail ${width}px`)
    await shoot(page, file('rail-collapsed', 'ipad', 'light', 'mist'))
    await c.reload(page)
    if ((await byId(page, 'ipad-rail').getAttribute('data-collapsed')) !== '1') throw new Error('collapsed state lost after reload')
    await byId(page, 'rail-collapse').click()
    await page.waitForFunction(() => document.querySelector('[data-testid="ipad-rail"]')?.getAttribute('data-collapsed') === '0')
    await byId(page, 'rail-settings').click()
    await byId(page, 'settings-view').waitFor({ timeout: 8000 })
    if ((await byId(page, 'ipad-rail').count()) !== 1 || (await byId(page, 'tab-bar').count()) !== 0) throw new Error('settings on iPad: rail missing or tab bar shown')
    await go(page, `/app/m/${id}`)
    await byId(page, 'monitor-view').waitFor({ timeout: 8000 })
    await page.evaluate(() => {
      const list = document.querySelector('[data-testid="monitor-list"]') ?? document.querySelector('[data-testid="monitor-groups"]')
      const first = list?.firstElementChild
      if (!list || !first) return
      for (let i = 0; i < 60; i++) list.appendChild(first.cloneNode(true))
    })
    const beforeBlur = await page.evaluate(new Function(`return (${BLURRED})()`))
    const scrolled = await page.evaluate(
      () =>
        new Promise((resolve) => {
          const box = document.querySelector('main.scroll')
          const marks = []
          let last = performance.now()
          const start = last
          let dir = 1
          const tick = (now) => {
            marks.push(now - last)
            last = now
            box.scrollTop += dir * 90
            if (box.scrollTop <= 0 || box.scrollTop + box.clientHeight >= box.scrollHeight - 1) dir = -dir
            if (now - start < 1800) requestAnimationFrame(tick)
            else {
              const sorted = marks.slice(1).sort((a, b) => a - b)
              resolve({ n: sorted.length, p50: sorted[Math.floor(sorted.length * 0.5)] ?? 0, p95: sorted[Math.floor(sorted.length * 0.95)] ?? 0, max: sorted[sorted.length - 1] ?? 0, range: box.scrollHeight - box.clientHeight })
            }
          }
          requestAnimationFrame(tick)
        }),
    )
    const afterBlur = await page.evaluate(new Function(`return (${BLURRED})()`))
    if (beforeBlur.join() !== afterBlur.join()) throw new Error('blur layers changed while scrolling')
    c.perf = c.perf ?? {}
    c.perf.listScroll = scrolled
    if (scrolled.range < 200) throw new Error(`long list did not overflow (${scrolled.range})`)
    console.log(`perf [${browserName}] terminal burst ${JSON.stringify(c.perf.terminalBurst)} list scroll ${JSON.stringify(c.perf.listScroll)}`)
    if (scrolled.p50 > (browserName === 'webkit' ? 400 : 60)) throw new Error(`list scroll median frame ${scrolled.p50} ms`)
    await go(page, `/app/m/${id}`)
    await c.reload(page)
    await page.setViewportSize(PHONE)
    await byId(page, 'tab-bar').waitFor({ timeout: 8000 })
    if ((await byId(page, 'ipad-rail').count()) !== 0) throw new Error('rail on phone')
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
