import { join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

const fresh = (peer) => {
  peer.events.length = 0
}

async function openViewer(c, page, id) {
  const { byId } = c
  await go(page, `/app/m/${id}/p/p1`)
  await byId(page, 'terminal-view').waitFor({ timeout: 10000 })
  await byId(page, 'open-viewer').click()
  await byId(page, 'viewer-files').waitFor({ timeout: 8000 })
}

async function closeViewer(c, page) {
  await c.byId(page, 'viewer-close').click()
  await page.waitForFunction(() => !document.querySelector('[data-testid="viewer"]'))
}

export async function editSteps(c, page, peer) {
  const { step, byId, expectText } = c
  const id = peer.info.machine

  await step('Viewer edit: edit and save writes with the version stamp; leaving a dirty draft asks first', async () => {
    peer.send('vwreset')
    await wait(200)
    await openViewer(c, page, id)
    await byId(page, 'entry-key_bar.dart').click()
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="file-foot"]', /dòng/)
    await byId(page, 'edit-start').click()
    await byId(page, 'editor').waitFor()
    if ((await byId(page, 'editor-view').getAttribute('data-dirty')) !== '0') throw new Error('dirty before typing')
    if (!(await byId(page, 'edit-save').isDisabled())) throw new Error('save enabled with no change')
    await byId(page, 'editor').fill('class KeyBar {}\n')
    if ((await byId(page, 'editor-view').getAttribute('data-dirty')) !== '1') throw new Error('not dirty after typing')

    await byId(page, 'viewer-seg-diff').click()
    await byId(page, 'confirm').waitFor()
    await byId(page, 'confirm-cancel').click()
    await page.waitForFunction(() => !document.querySelector('[data-testid="confirm"]'))
    if (!(await byId(page, 'editor').count())) throw new Error('editor lost after keep editing')

    await byId(page, 'edit-cancel').click()
    await byId(page, 'confirm').waitFor()
    await byId(page, 'confirm-ok').click()
    await byId(page, 'code-view').waitFor()
    if (await byId(page, 'editor').count()) throw new Error('editor still open after discard')

    fresh(peer)
    await byId(page, 'edit-start').click()
    await byId(page, 'editor').fill('class KeyBar {}\nclass Other {}\n')
    await byId(page, 'edit-save').click()
    const sent = await peer.waitEvent((e) => e.ev === 'write')
    if (sent.base !== 1 || sent.text !== 'class KeyBar {}\nclass Other {}\n') throw new Error(`write ${JSON.stringify(sent)}`)
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="action-note"]', /Đã lưu/)
    if (Number(await byId(page, 'code-view').getAttribute('data-lines')) !== 3) throw new Error('saved text not shown')
  })

  await step('Viewer edit: a file changed on the computer shows the conflict banner, Reload loads the disk version', async () => {
    peer.send('diskchange')
    await wait(150)
    fresh(peer)
    await byId(page, 'edit-start').click()
    await byId(page, 'editor').fill('mine\n')
    await byId(page, 'edit-save').click()
    await peer.waitEvent((e) => e.ev === 'write' && e.base === 2)
    await byId(page, 'edit-conflict').waitFor({ timeout: 8000 })
    if (!(await byId(page, 'editor').count())) throw new Error('draft thrown away on conflict')
    await c.shoot(page, join(c.env.qaDirD, `edit-conflict-phone-light-${c.browserName}.png`))
    await byId(page, 'edit-reload').click()
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="code-view"]', /edited on disk/)
    if (await byId(page, 'editor').count()) throw new Error('editor open after reload')
  })

  await step('Viewer edit: a file changed on the computer while editing shows the banner at once, Keep mine saves over it, Reload drops the draft', async () => {
    await byId(page, 'edit-start').click()
    await byId(page, 'editor').fill('mine\n')
    await wait(300)
    if (await byId(page, 'edit-disk-changed').count()) throw new Error('banner before any change')
    peer.send('diskchange')
    await byId(page, 'edit-disk-changed').waitFor({ timeout: 8000 })
    if ((await byId(page, 'editor').inputValue()) !== 'mine\n') throw new Error('draft lost when the banner showed')
    await c.shoot(page, join(c.env.qaDirE, `edit-disk-changed-phone-light-${c.browserName}.png`))
    await page.emulateMedia({ colorScheme: 'dark' })
    await wait(300)
    await c.shoot(page, join(c.env.qaDirE, `edit-disk-changed-phone-dark-${c.browserName}.png`))
    await page.emulateMedia({ colorScheme: 'light' })

    await byId(page, 'edit-disk-keep').click()
    await byId(page, 'edit-disk-changed').waitFor({ state: 'detached', timeout: 8000 })
    fresh(peer)
    await byId(page, 'edit-save').click()
    const sent = await peer.waitEvent((e) => e.ev === 'write')
    if (sent.text !== 'mine\n' || sent.base !== 22) throw new Error(`write after keep ${JSON.stringify(sent)}`)
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="action-note"]', /Đã lưu/)
    if (await byId(page, 'edit-conflict').count()) throw new Error('conflict after keep mine')

    await byId(page, 'edit-start').click()
    await byId(page, 'editor').fill('again\n')
    peer.send('diskchange')
    await byId(page, 'edit-disk-changed').waitFor({ timeout: 8000 })
    await byId(page, 'edit-disk-reload').click()
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="code-view"]', /edited on disk/)
    if (await byId(page, 'editor').count()) throw new Error('editor open after reload')
  })

  await step('Viewer git: stage, unstage, stage all, discard asks twice, commit with a message', async () => {
    await byId(page, 'viewer-seg-diff').click()
    await byId(page, 'change-src/app.ts').waitFor({ timeout: 8000 })
    await byId(page, 'row-staged-lib/util.ts').waitFor()
    if (!(await byId(page, 'commit-go').isDisabled())) throw new Error('commit enabled without a message')

    fresh(peer)
    await byId(page, 'stage-src/app.ts').click()
    const staged = await peer.waitEvent((e) => e.ev === 'git.stage')
    if (staged.paths.join() !== 'src/app.ts') throw new Error(`stage ${JSON.stringify(staged)}`)
    await byId(page, 'row-staged-src/app.ts').waitFor({ timeout: 8000 })
    await page.waitForFunction(() => !document.querySelector('[data-testid="row-changes-src/app.ts"]'))

    fresh(peer)
    await byId(page, 'unstage-src/app.ts').click()
    await peer.waitEvent((e) => e.ev === 'git.unstage' && e.paths.join() === 'src/app.ts')
    await byId(page, 'row-changes-src/app.ts').waitFor({ timeout: 8000 })

    await byId(page, 'discard-notes.txt').click()
    await byId(page, 'confirm').waitFor()
    await byId(page, 'confirm-cancel').click()
    await wait(300)
    if (peer.events.some((e) => e.ev === 'discard')) throw new Error('discard sent without confirm')
    await byId(page, 'discard-notes.txt').click()
    await byId(page, 'confirm-ok').click()
    await peer.waitEvent((e) => e.ev === 'discard' && e.path === 'notes.txt')
    await byId(page, 'remote-confirm').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="remote-confirm-title"]', /notes\.txt/)
    await byId(page, 'remote-confirm-ok').click()
    await page.waitForFunction(() => !document.querySelector('[data-testid="row-changes-notes.txt"]'), null, { timeout: 8000 })

    fresh(peer)
    await byId(page, 'stage-all').click()
    await peer.waitEvent((e) => e.ev === 'git.stage' && e.paths.join() === 'src/app.ts')
    await byId(page, 'row-staged-src/app.ts').waitFor({ timeout: 8000 })
    await byId(page, 'commit-message').fill('Thêm thanh phím phụ')
    if (await byId(page, 'commit-go').isDisabled()) throw new Error('commit disabled with staged files and a message')
    await c.shoot(page, join(c.env.qaDirD, `changes-phone-light-${c.browserName}.png`))
    await byId(page, 'commit-go').click()
    const commit = await peer.waitEvent((e) => e.ev === 'commit')
    if (commit.message !== 'Thêm thanh phím phụ') throw new Error(`commit ${JSON.stringify(commit)}`)
    await expectText(page, '[data-testid="action-note"]', /Đã commit/)
    await page.waitForFunction(() => !document.querySelector('[data-testid="staged-head"]'), null, { timeout: 8000 })
    await byId(page, 'async-empty').waitFor({ timeout: 8000 })
    await byId(page, 'viewer-seg-git').click()
    await byId(page, 'commit-c000000').waitFor({ timeout: 8000 })
  })

  await step('Viewer run: start with config and device, log lines, reload, restart and stop', async () => {
    await byId(page, 'viewer-seg-run').click()
    await byId(page, 'run-launch').waitFor({ timeout: 8000 })
    if ((await byId(page, 'run-device').inputValue()) !== 'd1') throw new Error('first ready device not picked')
    await byId(page, 'run-config').selectOption('c2')
    fresh(peer)
    await byId(page, 'run-start').click()
    const started = await peer.waitEvent((e) => e.ev === 'run.start')
    if (started.configId !== 'c2' || started.deviceId !== 'd1') throw new Error(`start ${JSON.stringify(started)}`)
    await byId(page, 'run-r2').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="run-r2"] [data-testid="run-log"]', /Launching lib\/main\.dart/)

    await byId(page, 'run-reload-r2').click()
    const hot = await peer.waitEvent((e) => e.ev === 'run.reload' && e.hot === true)
    if (hot.sessionId !== 'r2') throw new Error('reload session')
    await expectText(page, '[data-testid="run-r2"] [data-testid="run-log"]', /Reloaded 1 of 10/)
    await byId(page, 'run-restart-r2').click()
    await peer.waitEvent((e) => e.ev === 'run.reload' && e.hot === false)
    await expectText(page, '[data-testid="run-r2"] [data-testid="run-log"]', /Restarted application/)
    await wait(300)
    await c.shoot(page, join(c.env.qaDirD, `run-phone-light-${c.browserName}.png`))

    await byId(page, 'run-stop-r2').click()
    await peer.waitEvent((e) => e.ev === 'run.stop' && e.sessionId === 'r2')
    await page.waitForFunction(() => document.querySelector('[data-testid="run-r2"]')?.getAttribute('data-state') === 'stopped')
    if (await byId(page, 'run-stop-r2').count()) throw new Error('stop still offered after stopping')
    await closeViewer(c, page)
  })

  await step('Viewer in view only mode: no edit, stage, commit, discard or run controls', async () => {
    peer.send('vwreset')
    peer.send('viewonly')
    await wait(150)
    await go(page, `/app/m/${id}/ws`)
    await c.reload(page)
    await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
    await byId(page, 'view-only').waitFor({ timeout: 10000 })
    await openViewer(c, page, id)
    await byId(page, 'entry-key_bar.dart').click()
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="file-foot"]', /chỉ đọc/)
    if (await byId(page, 'edit-start').count()) throw new Error('edit offered while view only')
    await byId(page, 'viewer-seg-diff').click()
    await byId(page, 'change-src/app.ts').waitFor({ timeout: 8000 })
    for (const hidden of ['stage-all', 'stage-src/app.ts', 'discard-src/app.ts', 'commit-box']) {
      if (await byId(page, hidden).count()) throw new Error(`${hidden} offered while view only`)
    }
    await byId(page, 'viewer-seg-run').click()
    await byId(page, 'run-r1').waitFor({ timeout: 8000 })
    if ((await byId(page, 'run-launch').count()) || (await byId(page, 'run-stop-r1').count())) throw new Error('run controls offered while view only')
    await closeViewer(c, page)
    peer.send('viewoff')
    await wait(150)
    await go(page, `/app/m/${id}/ws`)
    await c.reload(page)
    await page.waitForFunction(() => !document.querySelector('[data-testid="view-only"]'), null, { timeout: 10000 })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })

  await step('Viewer edit, git and run screenshots: phone and iPad, light and dark', async () => {
    const webkit = c.browserName === 'webkit'
    const forms = webkit ? [['phone', PHONE], ['ipad', IPAD]] : [['phone', PHONE]]
    const schemes = webkit ? ['light', 'dark'] : ['light']
    for (const [form, viewport] of forms) {
      await page.setViewportSize(viewport)
      for (const scheme of schemes) {
        await page.emulateMedia({ colorScheme: scheme })
        peer.send('vwreset')
        await wait(200)
        await openViewer(c, page, id)
        await byId(page, 'entry-key_bar.dart').click()
        await byId(page, 'code-view').waitFor({ timeout: 8000 })
        await byId(page, 'edit-start').click()
        await byId(page, 'editor').fill('class KeyBar extends StatelessWidget {\n  const KeyBar({super.key});\n}\n')
        await wait(300)
        await c.noOverflow(page, `editor ${form} ${scheme}`)
        await c.shoot(page, join(c.env.qaDirD, `editor-${form}-${scheme}-${c.browserName}.png`))
        await byId(page, 'edit-cancel').click()
        await byId(page, 'confirm-ok').click()
        await byId(page, 'viewer-seg-diff').click()
        await byId(page, 'row-staged-lib/util.ts').waitFor({ timeout: 8000 })
        await byId(page, 'commit-message').fill('Sửa thanh phím')
        await wait(300)
        await c.noOverflow(page, `changes ${form} ${scheme}`)
        await c.shoot(page, join(c.env.qaDirD, `changes-${form}-${scheme}-${c.browserName}.png`))
        await byId(page, 'viewer-seg-run').click()
        await byId(page, 'run-launch').waitFor({ timeout: 8000 })
        await wait(300)
        await c.noOverflow(page, `run ${form} ${scheme}`)
        await c.shoot(page, join(c.env.qaDirD, `run-${form}-${scheme}-${c.browserName}.png`))
        await closeViewer(c, page)
      }
    }
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
