import { join } from 'node:path'
import { wait } from './ctx.mjs'

export async function go(page, path) {
  await page.evaluate((p) => {
    history.pushState({ idx: (history.state?.idx ?? 0) + 1 }, '', p)
    window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }))
  }, path)
}

export async function uiSteps(c, page, peer) {
  const { step, byId, expectText, shoot } = c
  const id = peer.info.machine
  const cards = () => page.locator('[data-testid="monitor-list"] button[data-testid^="session-"]')

  const settleCount = async (n, label) => {
    await page.waitForFunction((k) => document.querySelectorAll('[data-testid="monitor-list"] button[data-testid^="session-"]:not([class*="leave"] *):not([class*="leave"])').length === k, n, { timeout: 4000 }).catch(() => { throw new Error(label) })
  }

  await step('Home card opens Monitor A: cards sorted by state, tool line, filters', async () => {
    await byId(page, `machine-${id}`).locator('[data-testid="machine-open"]').click()
    await byId(page, 'monitor-view').waitFor({ timeout: 5000 })
    if ((await byId(page, 'monitor-view').getAttribute('data-mode')) !== 'cards') throw new Error('Monitor A is not the default')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="monitor-list"] button[data-testid^="session-"]').length === 4)
    const first = await c.text(cards().first())
    if (!/docs/.test(first) || !/Chờ bạn/.test(first)) throw new Error(`first card ${first}`)
    const t2 = await c.text(byId(page, 'session-t2'))
    if (!/Edit · src\/app\.ts/.test(t2) || !/Đang chạy/.test(t2)) throw new Error(`t2 card ${t2}`)
    if ((await byId(page, 'session-t2').locator('[data-testid="status-chip"] svg').count()) !== 1) throw new Error('chip icon missing')
    await byId(page, 'filter-running').click()
    await settleCount(2, 'running filter')
    await byId(page, 'filter-waiting').click()
    await settleCount(1, 'waiting filter')
    await byId(page, 'filter-all').click()
    if ((await byId(page, 'tab-badge').textContent())?.trim() !== '1') throw new Error('waiting badge')
  })

  await step('Monitor B groups by state; the mode survives a reload', async () => {
    await byId(page, 'mode-groups').click()
    await byId(page, 'monitor-groups').waitFor()
    const groups = await page.locator('[data-testid="monitor-groups"] > section').evaluateAll((els) => els.map((e) => e.getAttribute('data-rail')))
    if (groups.join(',') !== 'asked,working,idle') throw new Error(`groups ${groups}`)
    await c.reload(page)
    await byId(page, 'monitor-groups').waitFor({ timeout: 10000 })
    if ((await page.locator('[data-testid="monitor-groups"] button').count()) !== 4) throw new Error('rows')
  })

  await step('Session detail: sub-agents, last activity, terminal button opens that pane in the Workspace', async () => {
    await byId(page, 'row-t2').click()
    await byId(page, 'session-view').waitFor({ timeout: 5000 })
    await expectText(page, '[data-testid="session-subagents"]', /Build the shell/)
    if ((await page.locator('[data-testid="session-subagents"] > div').count()) !== 2) throw new Error('sub-agents')
    if ((await page.locator('[data-testid="session-activity"] > li').count()) !== 3) throw new Error('activity rows')
    await expectText(page, '[data-testid="session-activity"]', /Công cụ lỗi/)
    await byId(page, 'open-terminal').click()
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    if ((await byId(page, 'terminal-view').getAttribute('data-pane')) !== 'p2') throw new Error('wrong pane from the session')
    await byId(page, 'back').click()
    await byId(page, 'session-view').waitFor()
    await byId(page, 'back').click()
    await byId(page, 'monitor-view').waitFor()
    await byId(page, 'mode-cards').click()
    await byId(page, 'monitor-list').waitFor()
  })

  await step('Settings: language and theme apply at once and persist after reload', async () => {
    await byId(page, 'tab-settings').click()
    await byId(page, 'settings-view').waitFor()
    await byId(page, 'language-select').selectOption('en')
    await expectText(page, '[data-testid="page-title"]', /^Settings$/)
    await byId(page, 'theme-select').selectOption('dark')
    await page.waitForFunction(() => document.documentElement.classList.contains('dark'))
    await c.reload(page)
    await expectText(page, '[data-testid="page-title"]', /^Settings$/)
    if (!(await page.evaluate(() => document.documentElement.classList.contains('dark')))) throw new Error('theme lost')
    if ((await page.evaluate(() => document.documentElement.lang)) !== 'en') throw new Error('lang attr')
    await byId(page, 'language-select').selectOption('vi')
    await byId(page, 'theme-select').selectOption('auto')
    await expectText(page, '[data-testid="page-title"]', /^Cài đặt$/)
    await page.waitForFunction(() => !document.documentElement.classList.contains('dark'))
  })

  await step('Settings: palette cards and Reduce transparency apply at once and persist; the phone blur stays at 16px or less', async () => {
    await byId(page, 'tab-settings').click()
    await byId(page, 'palette-cards').waitFor()
    const root = () => page.evaluate(() => ({ ui: document.documentElement.dataset.ui, theme: document.documentElement.dataset.nextTheme, dark: document.documentElement.classList.contains('dark'), glass: document.documentElement.dataset.glass }))
    const first = await root()
    if (first.ui !== 'next' || first.theme !== 'mist' || first.glass !== 'full') throw new Error(`default appearance ${JSON.stringify(first)}`)
    const blur = await page.evaluate(() => {
      const s = getComputedStyle(document.querySelector('[data-testid="tab-bar"]'))
      return s.backdropFilter || s.webkitBackdropFilter || ''
    })
    const px = Number(/blur\((\d+)px/.exec(blur)?.[1] ?? 99)
    if (px > 16) throw new Error(`tab bar blur ${blur}`)
    for (const pal of ['mist', 'nebula', 'aurora', 'graphite', 'dawn', 'galaxy', 'orrery', 'console']) {
      await byId(page, `palette-${pal}`).click()
      await page.waitForFunction((p) => document.documentElement.dataset.nextTheme === p, pal)
      await wait(480)
      await c.noOverflow(page, `settings ${pal}`)
      await shoot(page, join(c.env.qaDirF, `settings-palette-${pal}-phone-${c.browserName}.png`))
    }
    await page.waitForFunction(() => document.documentElement.classList.contains('dark'))
    await byId(page, 'palette-mist').click()
    await byId(page, 'reduce-transparency').click()
    await page.waitForFunction(() => document.documentElement.dataset.glass === 'reduced')
    const none = await page.evaluate(() => {
      const s = getComputedStyle(document.querySelector('[data-testid="tab-bar"]'))
      return (s.backdropFilter || s.webkitBackdropFilter || 'none') === 'none'
    })
    if (!none) throw new Error('reduce transparency still blurs')
    await wait(480)
    await shoot(page, join(c.env.qaDirF, `settings-reduced-phone-${c.browserName}.png`))
    await c.reload(page)
    await byId(page, 'palette-cards').waitFor()
    const after = await root()
    if (after.theme !== 'mist' || after.glass !== 'reduced') throw new Error(`appearance lost ${JSON.stringify(after)}`)
    await byId(page, 'reduce-transparency').click()
    await page.waitForFunction(() => document.documentElement.dataset.glass === 'full')
  })

  await step('Bell is saved per machine in IndexedDB; display name can be changed and cleared', async () => {
    await c.home(page)
    const bell = byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]')
    if ((await bell.getAttribute('aria-pressed')) !== 'true') throw new Error('bell default on')
    await c.expectStatus(page, id, 'ready')
    await bell.click()
    await page.waitForFunction((sel) => document.querySelector(sel)?.getAttribute('aria-pressed') === 'false', `[data-testid="machine-${id}"]:not(.mlist-leave-active) [data-testid="machine-bell"]`, { timeout: 4000 }).catch(() => { throw new Error('bell click ignored') })
    await wait(800)
    await c.reload(page)
    if ((await byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]').getAttribute('aria-pressed')) !== 'false') throw new Error('bell not saved')
    await c.expectStatus(page, id, 'ready')
    await wait(400)
    await byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]').click()
    await page.waitForFunction((sel) => document.querySelector(sel)?.getAttribute('aria-pressed') === 'true', `[data-testid="machine-${id}"]:not(.mlist-leave-active) [data-testid="machine-bell"]`, { timeout: 4000 }).catch(() => { throw new Error('bell not restored') })
    await wait(800)
    await byId(page, 'home-settings').click()
    await byId(page, 'rename-start').click()
    await byId(page, 'rename-input').fill('Máy văn phòng')
    await byId(page, 'rename-save').click()
    await expectText(page, '[data-testid="settings-machine-name"]', /Máy văn phòng/)
    await c.home(page)
    await expectText(page, `[data-testid="machine-${id}"] [data-testid="machine-name"]`, /Máy văn phòng/)
    await byId(page, 'home-settings').click()
    await byId(page, 'rename-start').click()
    await byId(page, 'rename-input').fill('')
    await byId(page, 'rename-save').click()
    await expectText(page, '[data-testid="settings-machine-name"]', /Orrery Desk/)
    await c.home(page)
  })
}
