import { wait } from './ctx.mjs'

export async function go(page, path) {
  await page.evaluate((p) => {
    history.pushState({ idx: (history.state?.idx ?? 0) + 1 }, '', p)
    window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }))
  }, path)
}

export async function uiSteps(c, page, peer) {
  const { step, byId, expectText } = c
  const id = peer.info.machine
  const cards = () => page.locator('[data-testid="monitor-list"] > button')

  await step('Home card opens Monitor A: cards sorted by state, tool line, filters', async () => {
    await byId(page, `machine-${id}`).locator('[data-testid="machine-open"]').click()
    await byId(page, 'monitor-view').waitFor({ timeout: 5000 })
    if ((await byId(page, 'monitor-view').getAttribute('data-mode')) !== 'cards') throw new Error('Monitor A is not the default')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="monitor-list"] > button').length === 4)
    const first = await c.text(cards().first())
    if (!/docs/.test(first) || !/Chờ bạn/.test(first)) throw new Error(`first card ${first}`)
    const t2 = await c.text(byId(page, 'session-t2'))
    if (!/Edit · src\/app\.ts/.test(t2) || !/Đang chạy/.test(t2)) throw new Error(`t2 card ${t2}`)
    if ((await byId(page, 'session-t2').locator('[data-testid="status-chip"] svg').count()) !== 1) throw new Error('chip icon missing')
    await byId(page, 'filter-running').click()
    if ((await cards().count()) !== 2) throw new Error('running filter')
    await byId(page, 'filter-waiting').click()
    if ((await cards().count()) !== 1) throw new Error('waiting filter')
    await byId(page, 'filter-all').click()
    if ((await byId(page, 'tab-badge').textContent())?.trim() !== '1') throw new Error('waiting badge')
  })

  await step('Monitor B groups by state; the mode survives a reload', async () => {
    await byId(page, 'mode-toggle').click()
    await byId(page, 'monitor-groups').waitFor()
    const groups = await page.locator('[data-testid="monitor-groups"] > section').evaluateAll((els) => els.map((e) => e.getAttribute('data-rail')))
    if (groups.join(',') !== 'asked,working,idle') throw new Error(`groups ${groups}`)
    await c.reload(page)
    await byId(page, 'monitor-groups').waitFor({ timeout: 10000 })
    if ((await page.locator('[data-testid="monitor-groups"] button').count()) !== 4) throw new Error('rows')
  })

  await step('Session detail: sub-agents, last activity, terminal button goes to the Workspace stub', async () => {
    await byId(page, 'row-t2').click()
    await byId(page, 'session-view').waitFor({ timeout: 5000 })
    await expectText(page, '[data-testid="session-subagents"]', /Build the shell/)
    if ((await page.locator('[data-testid="session-subagents"] > div').count()) !== 2) throw new Error('sub-agents')
    if ((await page.locator('[data-testid="session-activity"] > li').count()) !== 3) throw new Error('activity rows')
    await expectText(page, '[data-testid="session-activity"]', /Công cụ lỗi/)
    await byId(page, 'open-terminal').click()
    await byId(page, 'workspace-soon').waitFor()
    await byId(page, 'back').click()
    await byId(page, 'session-view').waitFor()
    await byId(page, 'back').click()
    await byId(page, 'monitor-view').waitFor()
    await byId(page, 'mode-toggle').click()
    await byId(page, 'monitor-list').waitFor()
  })

  await step('Settings: language and theme apply at once and persist after reload', async () => {
    await byId(page, 'tab-settings').click()
    await byId(page, 'settings-view').waitFor()
    await byId(page, 'language-select').selectOption('en')
    await expectText(page, '[data-testid="page-title"]', /^Settings$/)
    await byId(page, 'theme-select').selectOption('dark')
    await page.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'dark')
    await c.reload(page)
    await expectText(page, '[data-testid="page-title"]', /^Settings$/)
    if ((await page.evaluate(() => document.documentElement.getAttribute('data-theme'))) !== 'dark') throw new Error('theme lost')
    if ((await page.evaluate(() => document.documentElement.lang)) !== 'en') throw new Error('lang attr')
    await byId(page, 'language-select').selectOption('vi')
    await byId(page, 'theme-select').selectOption('auto')
    await expectText(page, '[data-testid="page-title"]', /^Cài đặt$/)
    await page.waitForFunction(() => !document.documentElement.hasAttribute('data-theme'))
  })

  await step('Bell is saved per machine in IndexedDB; display name can be changed and cleared', async () => {
    await c.home(page)
    const bell = byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]')
    if ((await bell.getAttribute('aria-pressed')) !== 'true') throw new Error('bell default on')
    await bell.click()
    await wait(300)
    await c.reload(page)
    if ((await byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]').getAttribute('aria-pressed')) !== 'false') throw new Error('bell not saved')
    await byId(page, `machine-${id}`).locator('[data-testid="machine-bell"]').click()
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
