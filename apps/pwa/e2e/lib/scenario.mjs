import { join } from 'node:path'
import { idbCounts, makeCtx, PHONE, wait } from './ctx.mjs'
import { lockStep } from './lock-step.mjs'
import { startPeer } from './procs.mjs'
import { screenshotSteps } from './shots.mjs'
import { uiSteps } from './ui-steps.mjs'

export const STEPS_PER_BROWSER = 19

export async function runScenario({ browserName, launch, env, record }) {
  await fetch(`${env.base}/__version?web=${env.webHash}`)
  const browser = await launch(browserName)
  const c = makeCtx({ browser, browserName, env, record, videoPath: env.videoPath })
  const { step, byId, open, reload, home, pairPaste, pairScan, status, expectStatus, shoot, text, expectText, errors, peers } = c

  try {
    const peer = await startPeer(env.peerExe, env.relayHttp)
    peers.push(peer)
    const id = peer.info.machine
    const { page, context } = await c.newPage(PHONE)

    await step('boots empty with the wasm core, no CSP violation, no horizontal scroll', async () => {
      await page.goto(`${env.base}/app/`)
      await byId(page, 'machines-empty').waitFor({ timeout: 15000 })
      await c.ready(page)
      const csp = await page.evaluate(() => window.__csp)
      if (csp.length) throw new Error(`CSP violations: ${csp.join(' | ')}`)
      await c.noOverflow(page, 'empty home')
      if ((await page.locator('link[rel="manifest"]').count()) !== 1) throw new Error('manifest link missing')
      if (await byId(page, 'boot-error').count()) throw new Error('boot error shown')
      await wait(300)
      await shoot(page, join(env.qaDir, `empty-phone-light-${browserName}.png`))
    })

    await step('service worker installs, takes control and precaches the build', async () => {
      await page.evaluate(() => navigator.serviceWorker.ready.then(() => true))
      await reload(page)
      await page.waitForFunction(() => navigator.serviceWorker.controller !== null, null, { timeout: 10000 })
      const cached = await page.evaluate(async () => {
        const keys = await caches.keys()
        const cache = await caches.open(keys[0])
        return { n: keys.length, files: (await cache.keys()).map((r) => new URL(r.url).pathname) }
      })
      if (cached.n !== 1 || !cached.files.includes('/app/') || !cached.files.some((f) => f.endsWith('.wasm'))) throw new Error(`bad precache ${JSON.stringify(cached)}`)
    })

    await step('pairing screen: camera area starts or falls back; a wrong pasted code shows the error and the way out', async () => {
      await byId(page, 'empty-pair').click()
      await byId(page, 'pair-view').waitFor()
      await byId(page, 'qr-scanner').waitFor()
      await page.waitForFunction(() => document.querySelector('[data-testid="qr-scanner"]')?.getAttribute('data-state') !== 'starting', null, { timeout: 10000 })
      const state = await byId(page, 'qr-scanner').getAttribute('data-state')
      if (browserName === 'chromium' && state !== 'live') throw new Error(`camera state ${state}`)
      if (!['live', 'denied', 'unsupported'].includes(state)) throw new Error(`camera state ${state}`)
      await shoot(page, join(env.qaDir, `pair-scan-phone-light-${browserName}.png`))
      await byId(page, 'pair-tab-paste').click()
      await byId(page, 'pair-code').fill('not a code!')
      await byId(page, 'pair-submit').click()
      await byId(page, 'pair-paste-error').waitFor()
      await byId(page, 'pair-code').fill('abc')
      await byId(page, 'pair-submit').click()
      await byId(page, 'pair-error').waitFor({ timeout: 8000 })
      if ((await byId(page, 'pair-error').getAttribute('data-code')) !== 'badCode') throw new Error('wrong error kind')
      await shoot(page, join(env.qaDir, `pair-error-phone-light-${browserName}.png`))
      await byId(page, 'pair-other').click()
      await byId(page, 'pair-tab-scan').waitFor()
      await byId(page, 'back').click()
      await byId(page, 'machines-empty').waitFor()
    })

    await step(browserName === 'chromium' ? 'pairs by scanning the QR from the camera (fake video), through the relay' : 'pairs by pasting the code through the relay', async () => {
      if (browserName === 'chromium') await pairScan(page, peer)
      else await pairPaste(page, peer)
      await byId(page, `machine-${id}`).waitFor({ timeout: 10000 })
    })

    await step('IK session comes up over the relay (chip: icon + text + colour)', async () => {
      await expectStatus(page, id, 'ready')
      if ((await status(page, id).getAttribute('data-route')) !== 'relay') throw new Error('not relay')
      const label = await text(status(page, id))
      if (!/relay/i.test(label)) throw new Error(`label ${label}`)
      if ((await status(page, id).locator('svg').count()) !== 1) throw new Error('chip icon missing')
    })

    await step('snapshot arrives: version, counts, fresh', async () => {
      await expectText(page, `[data-testid="machine-${id}"] [data-testid="machine-counts"]`, /2 đang chạy/)
      const counts = await text(byId(page, `machine-${id}`).locator('[data-testid="machine-counts"]'))
      if (!/1 chờ bạn/.test(counts) || !/1 rảnh/.test(counts)) throw new Error(`counts ${counts}`)
      if ((await byId(page, `machine-${id}`).locator('[data-testid="machine-freshness"]').getAttribute('data-stale')) !== '0') throw new Error('stale')
      if (!(await text(byId(page, `machine-${id}`))).includes('Orrery 2.3.0')) throw new Error('version missing')
    })

    await uiSteps(c, page, peer)

    await step('route policy select: LAN-only without a LAN address goes offline, automatic reconnects', async () => {
      await byId(page, `machine-${id}`).locator('[data-testid="route-select"]').selectOption('lan')
      await expectStatus(page, id, 'offline')
      await byId(page, `machine-${id}`).locator('[data-testid="route-select"]').selectOption('auto')
      await expectStatus(page, id, 'ready')
    })

    await step('live events and workspace patch update the store', async () => {
      peer.send('events')
      await expectText(page, `[data-testid="machine-${id}"] [data-testid="machine-counts"]`, /1 đang chạy/)
      peer.send('patch')
      await byId(page, 'home-settings').click()
      await byId(page, 'open-debug').click()
      await expectText(page, `[data-testid="debug-machine-${id}"]`, /seq 2 · ws rev 2 · 1 tab · 4 terminal · mới/)
      await home(page)
    })

    await screenshotSteps(c, page, peer)

    await step('desktop gone: card turns stale, reload shows the saved snapshot from IndexedDB at once, Monitor shows the offline banner', async () => {
      peer.stop()
      await expectStatus(page, id, 'reconnecting', 20000)
      await wait(500)
      await page.reload()
      await expectText(page, `[data-testid="machine-${id}"] [data-testid="machine-counts"]`, /1 đang chạy/, 5000)
      const stale = await byId(page, `machine-${id}`).locator('[data-testid="machine-freshness"]').getAttribute('data-stale')
      if (stale !== '1') throw new Error('cached view not marked stale')
      if ((await status(page, id).getAttribute('data-status')) === 'ready') throw new Error('unexpectedly ready')
      await shoot(page, join(env.qaDir, `home-stale-phone-light-${browserName}.png`))
      await byId(page, `machine-${id}`).locator('[data-testid="machine-open"]').click()
      await byId(page, 'monitor-offline').waitFor({ timeout: 5000 })
      if ((await page.locator('[data-testid="monitor-list"] > button').count()) !== 4) throw new Error('cached sessions missing')
      await shoot(page, join(env.qaDir, `monitor-offline-phone-light-${browserName}.png`))
      await byId(page, 'back').click()
      await byId(page, 'machines-view').waitFor()
    })

    await step('new web build on the server shows the update bar; Reload clears it', async () => {
      if (await byId(page, 'update-bar').count()) throw new Error('bar shown too early')
      await fetch(`${env.base}/__version?web=ffffffffffffffff`)
      await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')))
      await byId(page, 'update-bar').waitFor({ timeout: 5000 })
      await byId(page, 'update-reload').click()
      await page.waitForLoadState('load')
      await byId(page, 'machines-view').waitFor()
      await page.waitForFunction(() => !document.querySelector('[data-testid="update-bar"]'), null, { timeout: 5000 })
      await wait(500)
      if (await byId(page, 'update-bar').count()) throw new Error('bar still shown after reload')
    })

    await context.close()
    const second = await startPeer(env.peerExe, env.relayHttp)
    peers.push(second)
    const id2 = second.info.machine
    const fresh = await c.newPage(PHONE)

    await step('revoked device (close 4401): machine data wiped from the UI and IndexedDB, notice shown, stays wiped after reload', async () => {
      await open(fresh.page)
      await pairPaste(fresh.page, second)
      await expectStatus(fresh.page, id2, 'ready')
      await expectText(fresh.page, `[data-testid="machine-${id2}"] [data-testid="machine-counts"]`, /2 đang chạy/)
      await wait(2300)
      const before = await idbCounts(fresh.page)
      if (before.machines !== 1 || before.snapshots !== 1) throw new Error(`expected 1/1 before revoke, got ${JSON.stringify(before)}`)
      second.send('revoke')
      await byId(fresh.page, 'notice').waitFor({ timeout: 10000 })
      await byId(fresh.page, 'machines-empty').waitFor({ timeout: 5000 })
      await wait(500)
      const after = await idbCounts(fresh.page)
      if (after.machines !== 0 || after.snapshots !== 0) throw new Error(`not wiped ${JSON.stringify(after)}`)
      await shoot(fresh.page, join(env.qaDir, `revoked-phone-light-${browserName}.png`))
      await reload(fresh.page)
      await byId(fresh.page, 'machines-empty').waitFor({ timeout: 10000 })
      const again = await idbCounts(fresh.page)
      if (again.machines !== 0 || again.snapshots !== 0) throw new Error(`came back ${JSON.stringify(again)}`)
    })

    await lockStep(c)

    await step('no CSP violation and no unexpected console error across the whole run', async () => {
      const csp = await fresh.page.evaluate(() => window.__csp)
      const blocked = [...csp, ...errors.filter((e) => /content security policy|refused to/i.test(e))]
      if (blocked.length) throw new Error(`CSP: ${blocked.join(' | ')}`)
      const real = errors.filter((e) => !/WebSocket|websocket|Failed to load resource|net::ERR|closed|NetworkError|the server responded/i.test(e))
      if (real.length) throw new Error(`console errors: ${real.join(' | ')}`)
    })
    await fresh.context.close()
  } finally {
    for (const peer of peers) peer.stop()
    await browser.close()
  }
}
