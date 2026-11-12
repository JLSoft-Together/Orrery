import { join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

export async function screenshotSteps(c, page, peer) {
  const { step, byId, env, browserName, shoot, noOverflow } = c
  const id = peer.info.machine
  const screens = [
    { name: 'home', path: '/app/', waitFor: 'machines-view' },
    { name: 'monitor-a', path: `/app/m/${id}`, waitFor: 'monitor-list' },
    { name: 'monitor-b', path: `/app/m/${id}`, waitFor: 'monitor-groups', mode: 'groups' },
    { name: 'detail', path: `/app/m/${id}/s/t2`, waitFor: 'session-view' },
    { name: 'settings', path: '/app/settings', waitFor: 'settings-view' },
    { name: 'pair-code', path: '/app/pair', waitFor: 'pair-view', after: async () => byId(page, 'pair-tab-paste').click() },
  ]

  await step('screenshots: Home, Monitor A/B, detail, settings, pair; light and dark; phone and iPad', async () => {
    await wait(400)
    for (const [form, viewport] of [['phone', PHONE], ['ipad', IPAD]]) {
      await page.setViewportSize(viewport)
      for (const scheme of ['light', 'dark']) {
        await page.emulateMedia({ colorScheme: scheme })
        for (const screen of screens) {
          const want = screen.mode ?? 'cards'
          const label = `${screen.name} ${form} ${scheme}`
          try {
          await go(page, screen.path)
          await byId(page, screen.name.startsWith('monitor') ? 'monitor-view' : screen.waitFor).waitFor({ timeout: 8000 })
          if (screen.name.startsWith('monitor')) {
            const mode = await byId(page, 'monitor-view').getAttribute('data-mode')
            if (mode !== want) await byId(page, 'mode-toggle').click()
            await byId(page, screen.waitFor).waitFor({ timeout: 5000 })
          }
          if (screen.after) await screen.after()
          await wait(450)
          await noOverflow(page, `${screen.name} ${form}`)
          if (form === 'ipad' && screen.name === 'detail' && !(await byId(page, 'detail-pane').locator('[data-testid="session-view"]').count())) throw new Error('iPad detail pane empty')
          await shoot(page, join(env.qaDir, `${screen.name}-${form}-${scheme}-${browserName}.png`))
          } catch (error) {
            throw new Error(`${label}: ${String(error?.message ?? error).split('\n')[0]} @ ${page.url()}`)
          }
        }
      }
    }
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    await go(page, `/app/m/${id}`)
    if ((await byId(page, 'monitor-view').getAttribute('data-mode')) === 'groups') await byId(page, 'mode-toggle').click()
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
