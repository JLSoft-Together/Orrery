import { join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { startPeer } from './procs.mjs'
import { go } from './ui-steps.mjs'

const chipCount = (page) => page.locator('[data-testid="ws-chips"] > button').count()

async function focusMode(c, page, id) {
  const { byId } = c
  await go(page, `/app/m/${id}/ws`)
  await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
  if ((await byId(page, 'workspace-view').getAttribute('data-mode')) !== 'focus') await byId(page, 'ws-mode').click()
  await byId(page, 'ws-focus').waitFor({ timeout: 8000 })
}

async function openActions(c, page, chip) {
  const { byId } = c
  await byId(page, `chip-${chip}`).click()
  await page.waitForFunction((k) => document.querySelector('[data-testid="ws-focus"]')?.getAttribute('data-pane') === k, chip)
  await byId(page, 'ws-more').click()
  await byId(page, 'pane-actions').waitFor()
}

export async function paneSteps(c, page, peer) {
  const { step, byId, expectText } = c
  const id = peer.info.machine

  await step('Pane actions: rename tab, compact asks first, move to another tab, close asks then Undo reopens the pane', async () => {
    await focusMode(c, page, id)
    peer.send('tab2')
    await byId(page, 'ws-tab-tab2').waitFor({ timeout: 8000 })

    await openActions(c, page, 'p1')
    await byId(page, 'act-rename').click()
    await byId(page, 'rename-input').fill('Backend')
    await byId(page, 'rename-save').click()
    await peer.waitEvent((e) => e.ev === 'rename' && e.tabKey === 'tab1' && e.name === 'Backend')
    await expectText(page, '[data-testid="ws-tab-tab1"]', /Backend/)
    await page.waitForFunction(() => !document.querySelector('[data-testid="pane-actions"]'))

    await openActions(c, page, 'p1')
    await byId(page, 'act-compact').click()
    await byId(page, 'confirm').waitFor()
    await byId(page, 'confirm-cancel').click()
    await wait(300)
    if (peer.events.some((e) => e.ev === 'compact')) throw new Error('compact sent without confirm')
    await byId(page, 'act-compact').click()
    await byId(page, 'confirm-ok').click()
    await peer.waitEvent((e) => e.ev === 'compact' && e.paneKey === 'p1')
    await page.waitForFunction(() => !document.querySelector('[data-testid="pane-actions"]'))

    await openActions(c, page, 'p3')
    if (await byId(page, 'act-compact').count()) throw new Error('compact offered for a shell pane')
    await byId(page, 'act-move').click()
    await byId(page, 'move-tab2').click()
    await peer.waitEvent((e) => e.ev === 'move' && e.paneKey === 'p3' && e.tabKey === 'tab2')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="ws-chips"] > button').length === 2)

    await openActions(c, page, 'p2')
    await byId(page, 'act-close').click()
    await byId(page, 'confirm').waitFor()
    await byId(page, 'confirm-cancel').click()
    await wait(300)
    if (peer.events.some((e) => e.ev === 'close')) throw new Error('close sent without confirm')
    await byId(page, 'act-close').click()
    await byId(page, 'confirm-ok').click()
    await peer.waitEvent((e) => e.ev === 'close' && e.paneKey === 'p2')
    await byId(page, 'undo-bar').waitFor({ timeout: 5000 })
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="ws-chips"] > button').length === 1)
    await byId(page, 'undo-go').click()
    await peer.waitEvent((e) => e.ev === 'reopen')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="ws-chips"] > button').length === 2)
    await page.waitForFunction(() => !document.querySelector('[data-testid="undo-bar"]'))
    if ((await chipCount(page)) !== 2) throw new Error('chips after undo')
  })

  await step('Close confirmed on the phone: Undo shows after the answer, not before, and not when the answer is no', async () => {
    peer.send('closeask')
    await wait(150)
    peer.events.length = 0
    await openActions(c, page, 'p2')
    await byId(page, 'act-close').click()
    await byId(page, 'confirm-ok').click()
    await byId(page, 'remote-confirm').waitFor({ timeout: 8000 })
    await wait(300)
    if (await byId(page, 'undo-bar').count()) throw new Error('undo shown before the answer')
    await byId(page, 'remote-confirm-cancel').click()
    await page.waitForFunction(() => !document.querySelector('[data-testid="remote-confirm"]'))
    await wait(300)
    if (await byId(page, 'undo-bar').count()) throw new Error('undo shown after a refused close')
    if (peer.events.some((e) => e.ev === 'close')) throw new Error('closed after a refusal')

    peer.send('closeask')
    await wait(150)
    await byId(page, 'chip-p2').click()
    await byId(page, 'ws-more').click()
    await byId(page, 'act-close').click()
    await byId(page, 'confirm-ok').click()
    await byId(page, 'remote-confirm').waitFor({ timeout: 8000 })
    await byId(page, 'remote-confirm-ok').click()
    await peer.waitEvent((e) => e.ev === 'close' && e.paneKey === 'p2')
    await byId(page, 'undo-bar').waitFor({ timeout: 5000 })
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="ws-chips"] > button').length === 1)
    await c.shoot(page, join(c.env.qaDirE, `close-undo-phone-light-${c.browserName}.png`))
    await byId(page, 'undo-go').click()
    await peer.waitEvent((e) => e.ev === 'reopen')
    await page.waitForFunction(() => document.querySelectorAll('[data-testid="ws-chips"] > button').length === 2)
    await page.waitForFunction(() => !document.querySelector('[data-testid="undo-bar"]'))
  })

  await step('Pane actions screenshots: menu, phone and iPad, light and dark', async () => {
    const webkit = c.browserName === 'webkit'
    const forms = webkit ? [['phone', PHONE], ['ipad', IPAD]] : [['phone', PHONE]]
    const schemes = webkit ? ['light', 'dark'] : ['light']
    for (const [form, viewport] of forms) {
      await page.setViewportSize(viewport)
      for (const scheme of schemes) {
        await page.emulateMedia({ colorScheme: scheme })
        await focusMode(c, page, id)
        await openActions(c, page, 'p1')
        await wait(450)
        await c.noOverflow(page, `pane actions ${form} ${scheme}`)
        await c.shoot(page, join(c.env.qaDirC, `pane-actions-${form}-${scheme}-${c.browserName}.png`))
        await byId(page, 'pane-actions').click({ position: { x: 4, y: 4 } })
        await page.waitForFunction(() => !document.querySelector('[data-testid="pane-actions"]'))
      }
    }
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
  })
}

export async function helloControlStep(c, page, peer) {
  const { step, byId } = c
  const id = peer.info.machine
  await step('hello says control is off: view only shows after a reload without any denied command, and goes away when control is back on', async () => {
    await go(page, `/app/m/${id}/ws`)
    await c.reload(page)
    await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
    await byId(page, 'view-only').waitFor({ timeout: 10000 })
    if (!(await byId(page, 'ws-more').isDisabled())) throw new Error('actions enabled while view only')
    if (await byId(page, 'ws-new').count()) throw new Error('new pane offered while view only')
    peer.send('viewoff')
    await wait(200)
    await c.reload(page)
    await byId(page, 'workspace-view').waitFor({ timeout: 8000 })
    await page.waitForFunction(() => !document.querySelector('[data-testid="view-only"]'), null, { timeout: 10000 })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}

export async function pairDeniedStep(c, env) {
  const { step, byId, peers } = c
  await step('pairing refused on the computer: the phone says so with a retry button, not an endless wait', async () => {
    const third = await startPeer(env.peerExe, env.relayHttp)
    peers.push(third)
    third.send('deny')
    await wait(150)
    const { page, context } = await c.newPage(PHONE)
    try {
      await c.open(page)
      await byId(page, 'empty-pair').click()
      await byId(page, 'pair-view').waitFor({ timeout: 5000 })
      await byId(page, 'pair-tab-paste').dispatchEvent('click')
      await byId(page, 'pair-code').fill(third.info.qr)
      await byId(page, 'pair-submit').click()
      await byId(page, 'pair-error').waitFor({ timeout: 20000 })
      if ((await byId(page, 'pair-error').getAttribute('data-code')) !== 'denied') throw new Error('wrong code')
      await byId(page, 'pair-retry').waitFor()
      await wait(300)
      await c.shoot(page, join(env.qaDirC, `pair-denied-phone-light-${c.browserName}.png`))
    } finally {
      await context.close()
    }
  })
}
