import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

const GROUPS = ['menu', 'permission', 'question', 'clash', 'error', 'note', 'done']

const groupsShown = (page) => page.locator('[data-testid="inbox-sheet"] [data-group]').evaluateAll((els) => els.map((e) => e.getAttribute('data-group')))

export async function inboxSteps(c, page, peer) {
  const { step, byId, env, browserName, shoot, noOverflow, text } = c
  const id = peer.info.machine
  const badge = (n) => page.waitForFunction((k) => document.querySelector('[data-testid="tab-badge"]')?.textContent?.trim() === k, String(n), { timeout: 8000 })
  const dir = env.qaDirH
  mkdirSync(dir, { recursive: true })

  await step('Inbox: flagged snapshot gives the Monitor badge and a sheet grouped in INBOX_GROUPS order, clash row names file and peer; light and dark', async () => {
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    peer.send('inbox')
    await go(page, `/app/m/${id}/mon`)
    await byId(page, 'monitor-view').waitFor({ timeout: 8000 })
    await badge(4)
    await byId(page, 'inbox-open').click()
    await byId(page, 'inbox-sheet').waitFor()
    const shown = await groupsShown(page)
    const want = GROUPS.filter((g) => ['menu', 'permission', 'clash', 'done'].includes(g))
    if (shown.join(',') !== want.join(',')) throw new Error(`groups ${shown}`)
    const clash = await text(byId(page, 'inbox-row-p2'))
    if (!/src\/app\.ts/.test(clash) || !/api claude/.test(clash)) throw new Error(`clash row ${clash}`)
    if ((await byId(page, 'inbox-row-p4').getAttribute('data-reason')) !== 'menu') throw new Error('menu row')
    if ((await byId(page, 'inbox-count').textContent())?.trim() !== '4') throw new Error('count chip')
    await noOverflow(page, 'inbox sheet')
    await wait(500)
    if (browserName === 'webkit') await shoot(page, join(dir, 'pwa-inbox-light.png'))
    await page.emulateMedia({ colorScheme: 'dark' })
    await wait(1200)
    if (browserName === 'webkit') await shoot(page, join(dir, 'pwa-inbox-dark.png'))
    await page.emulateMedia({ colorScheme: 'light' })
  })

  await step('Inbox: a row only navigates to that session, no keys are sent', async () => {
    const before = peer.events.length
    await byId(page, 'inbox-row-p2').click()
    await byId(page, 'session-view').waitFor({ timeout: 8000 })
    if (!page.url().endsWith(`/m/${id}/s/t2`)) throw new Error(`url ${page.url()}`)
    if (await byId(page, 'inbox-sheet').count()) throw new Error('sheet still open')
    await wait(300)
    const sent = peer.events.slice(before).filter((e) => /key|input|pty/i.test(JSON.stringify(e)))
    if (sent.length) throw new Error(`keys sent ${JSON.stringify(sent)}`)
  })

  await step('Inbox: an old desktop without the flag falls back to attention (question, done); no inbox clears the badge', async () => {
    peer.send('inboxold')
    await go(page, `/app/m/${id}/mon`)
    await byId(page, 'monitor-view').waitFor({ timeout: 8000 })
    await badge(2)
    await byId(page, 'inbox-open').click()
    await byId(page, 'inbox-sheet').waitFor()
    const shown = await groupsShown(page)
    if (shown.join(',') !== 'question,done') throw new Error(`fallback groups ${shown}`)
    await go(page, `/app/m/${id}/ws`)
    await byId(page, 'inbox-sheet').waitFor({ state: 'detached', timeout: 5000 })
    peer.send('inboxoff')
    await go(page, `/app/m/${id}/mon`)
    await byId(page, 'monitor-view').waitFor({ timeout: 8000 })
    await page.waitForFunction(() => !document.querySelector('[data-testid="tab-badge"]'), null, { timeout: 8000 })
    await byId(page, 'inbox-open').click()
    await byId(page, 'inbox-empty').waitFor()
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
