import { join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

export async function workspaceShots(c, page, peer) {
  const { step, byId, env, browserName, shoot, noOverflow } = c
  const id = peer.info.machine
  const webkit = browserName === 'webkit'
  const forms = webkit ? [['phone', PHONE], ['ipad', IPAD]] : [['phone', PHONE]]
  const schemes = webkit ? ['light', 'dark'] : ['light']
  const live = () => page.waitForFunction(() => document.querySelector('[data-testid="terminal"]')?.getAttribute('data-state') === 'live', null, { timeout: 12000 })

  await step('screenshots: Workspace A / B, terminal, pane picker, file, diff, confirm; light and dark; phone and iPad', async () => {
    for (const [form, viewport] of forms) {
      await page.setViewportSize(viewport)
      for (const scheme of schemes) {
        await page.emulateMedia({ colorScheme: scheme })
        const file = (name) => join(env.qaDirB, `${name}-${form}-${scheme}-${browserName}.png`)
        const label = (name) => `${name} ${form} ${scheme}`
        const run = async (name, fn) => {
          try {
            await fn()
          } catch (error) {
            throw new Error(`${label(name)}: ${String(error?.message ?? error).split('\n')[0]} @ ${page.url()}`)
          }
        }

        await run('ws-a', async () => {
          await go(page, `/app/m/${id}/ws`)
          await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
          if ((await byId(page, 'workspace-view').getAttribute('data-mode')) !== 'focus') await byId(page, 'ws-mode').click()
          await byId(page, 'ws-focus').waitFor()
          await wait(450)
          await noOverflow(page, label('ws-a'))
          await shoot(page, file('ws-a'))
        })
        if (!webkit) {
          await run('term', async () => {
            await go(page, `/app/m/${id}/p/p1`)
            await byId(page, 'terminal-view').waitFor()
            await live()
            peer.send('demo')
            await wait(700)
            await shoot(page, file('term'))
          })
          continue
        }
        await run('ws-b', async () => {
          await byId(page, 'ws-mode').click()
          await byId(page, 'ws-grid').waitFor()
          await wait(450)
          await noOverflow(page, label('ws-b'))
          await shoot(page, file('ws-b'))
          await byId(page, 'ws-mode').click()
        })
        await run('confirm', async () => {
          peer.send('confirm')
          await byId(page, 'remote-confirm').waitFor({ timeout: 8000 })
          await wait(450)
          await shoot(page, file('confirm'))
          await byId(page, 'remote-confirm-cancel').click()
          await page.waitForFunction(() => !document.querySelector('[data-testid="remote-confirm"]'))
        })
        await run('term', async () => {
          await go(page, `/app/m/${id}/p/p1`)
          await byId(page, 'terminal-view').waitFor()
          await live()
          peer.send('demo')
          await wait(700)
          await noOverflow(page, label('term'))
          await shoot(page, file('term'))
        })
        await run('term-pick', async () => {
          await byId(page, 'pane-pick').click()
          await byId(page, 'pane-sheet').waitFor()
          await wait(450)
          await shoot(page, file('term-pick'))
          await byId(page, 'pane-sheet').click({ position: { x: 4, y: 4 } })
          await page.waitForFunction(() => !document.querySelector('[data-testid="pane-sheet"]'))
        })
        await run('file', async () => {
          await byId(page, 'open-viewer').click()
          await byId(page, 'entry-key_bar.dart').click()
          await byId(page, 'code-view').waitFor({ timeout: 8000 })
          await wait(450)
          await noOverflow(page, label('file'))
          await shoot(page, file('file'))
        })
        await run('diff', async () => {
          await byId(page, 'files-up').click()
          await byId(page, 'viewer-seg-diff').click()
          await byId(page, 'change-src/app.ts').click()
          await byId(page, 'diff-view').waitFor({ timeout: 8000 })
          await wait(450)
          await noOverflow(page, label('diff'))
          await shoot(page, file('diff'))
          await byId(page, 'viewer-close').click()
        })
        await go(page, '/app/')
        await byId(page, 'machines-view').waitFor()
      }
    }
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
