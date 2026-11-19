import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

export async function mistHomeSteps(c, page, peer) {
  const { step, byId, env, browserName, shoot, noOverflow, text } = c
  const id = peer.info.machine
  const dir = join(dirname(env.qaDir), 'WP-68b3')
  mkdirSync(dir, { recursive: true })

  await step('Mist screens: quota bars and OS on the card, Monitor by project, answer banner, limit banner with retry, 0 CSP violations; phone and iPad, light and dark', async () => {
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
    const card = byId(page, `machine-${id}`)
    const quota = await text(card.locator('[data-testid="machine-quota"]'))
    for (const part of [/Claude Code/, /42\s?%/, /91\s?%/, /Codex/, /7\s?%/]) if (!part.test(quota)) throw new Error(`quota text ${quota}`)
    if ((await card.locator('[data-agent="claude"] .pct.tone-bad').count()) !== 1) throw new Error('91% is not red')
    if ((await card.locator('[data-testid="machine-quota"] [role="progressbar"]').count()) !== 3) throw new Error('quota bars')
    const sub = await text(card.locator('.sub'))
    if (!/Orrery 2\.3\.0, Windows/.test(sub)) throw new Error(`subtitle ${sub}`)

    await go(page, `/app/m/${id}`)
    await byId(page, 'monitor-list').waitFor({ timeout: 8000 })
    if ((await page.locator('[data-testid^="project-"]').count()) !== 4) throw new Error('project cards')
    if ((await byId(page, 'mode-cards').getAttribute('aria-selected')) !== 'true') throw new Error('Project is not the default view')
    if ((await page.locator('[data-testid="monitor-list"] [data-testid="status-chip"] svg').count()) !== 4) throw new Error('row status icons')

    await go(page, `/app/m/${id}/s/t3`)
    await byId(page, 'session-ask').waitFor({ timeout: 8000 })
    if (!/npm test/.test(await text(byId(page, 'session-ask')))) throw new Error('ask text')
    await byId(page, 'session-answer').waitFor()

    const screens = [
      { name: 'home', path: '/app/', wait: 'machines-view' },
      { name: 'monitor-a', path: `/app/m/${id}`, wait: 'monitor-list' },
      { name: 'monitor-b', path: `/app/m/${id}`, wait: 'monitor-groups', mode: 'groups' },
      { name: 'detail', path: `/app/m/${id}/s/t3`, wait: 'session-ask' },
      { name: 'limit', path: `/app/m/${id}`, wait: 'monitor-limit', limit: true },
      { name: 'pair-scan', path: '/app/pair', wait: 'pair-view' },
      { name: 'pair-code', path: '/app/pair', wait: 'pair-view', after: async () => {
        await page.locator('[data-testid="pair-tab-paste"], [data-testid="pair-other"]').first().waitFor({ timeout: 8000 })
        if (await byId(page, 'pair-tab-paste').count()) await byId(page, 'pair-tab-paste').dispatchEvent('click')
        else await byId(page, 'pair-other').dispatchEvent('click')
        await byId(page, 'pair-code').waitFor({ timeout: 8000 })
      } },
    ]
    for (const [form, viewport] of [['phone', PHONE], ['ipad', IPAD]]) {
      await page.setViewportSize(viewport)
      for (const scheme of ['light', 'dark']) {
        await page.emulateMedia({ colorScheme: scheme })
        for (const screen of screens) {
          try {
            await go(page, screen.path)
            await byId(page, screen.mode || screen.name.startsWith('monitor') || screen.limit ? 'monitor-view' : screen.wait).waitFor({ timeout: 8000 })
            if (screen.name.startsWith('monitor') || screen.limit) {
              const want = screen.mode ?? 'cards'
              if ((await byId(page, 'monitor-view').getAttribute('data-mode')) !== want) await byId(page, want === 'groups' ? 'mode-groups' : 'mode-cards').click()
            }
            if (screen.limit) {
              peer.send('limit')
              await byId(page, 'monitor-limit').waitFor({ timeout: 8000 })
            }
            await byId(page, screen.wait).waitFor({ timeout: 8000 })
            if (screen.after) await screen.after()
            await wait(500)
            await noOverflow(page, `${screen.name} ${form}`)
            await shoot(page, join(dir, `${screen.name}-${form}-${scheme}-${browserName}.png`))
            if (screen.limit) {
              await byId(page, 'monitor-limit-retry').click()
              await byId(page, 'monitor-limit').waitFor({ state: 'detached', timeout: 8000 })
            }
          } catch (error) {
            throw new Error(`${screen.name} ${form} ${scheme}: ${String(error?.message ?? error).split('\n')[0]}`)
          }
        }
      }
    }

    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    for (const palette of ['nebula', 'galaxy']) {
      await page.evaluate((p) => document.documentElement.setAttribute('data-next-theme', p), palette)
      for (const [name, path, wait1] of [['home', '/app/', 'machines-view'], ['monitor-a', `/app/m/${id}`, 'monitor-list']]) {
        await go(page, path)
        await byId(page, wait1).waitFor({ timeout: 8000 })
        await wait(450)
        await shoot(page, join(dir, `${name}-phone-${palette}-${browserName}.png`))
      }
    }
    await page.evaluate(() => document.documentElement.setAttribute('data-next-theme', 'mist'))
    await go(page, `/app/m/${id}`)
    if ((await byId(page, 'monitor-view').getAttribute('data-mode')) === 'groups') await byId(page, 'mode-cards').click()
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
    const csp = await page.evaluate(() => window.__csp)
    if (csp.length) throw new Error(`CSP violations: ${csp.join(' | ')}`)
  })
}
