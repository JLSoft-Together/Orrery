import { join } from 'node:path'
import { startPeer } from './procs.mjs'

const wait = (ms) => new Promise((r) => setTimeout(r, ms))
const PHONE = { width: 390, height: 844 }
const IPAD = { width: 820, height: 1180 }

async function idbCounts(page) {
  return page.evaluate(
    () =>
      new Promise((resolve, reject) => {
        const open = indexedDB.open('orrery-pwa')
        open.onerror = () => reject(open.error)
        open.onsuccess = () => {
          const db = open.result
          const out = {}
          const tx = db.transaction(['machines', 'snapshots'])
          let left = 2
          for (const store of ['machines', 'snapshots']) {
            const q = tx.objectStore(store).count()
            q.onsuccess = () => {
              out[store] = q.result
              if (--left === 0) resolve(out)
            }
          }
        }
      }),
  )
}

export async function runScenario({ browserName, launch, env, record }) {
  await fetch(`${env.base}/__version?web=${env.webHash}`)
  const browser = await launch(browserName)
  const peers = []
  const errors = []
  const step = async (name, fn) => {
    const started = Date.now()
    try {
      await fn()
      record({ browser: browserName, name, ok: true, ms: Date.now() - started })
    } catch (error) {
      record({ browser: browserName, name, ok: false, error: String(error?.message ?? error).split('\n')[0] })
      throw error
    }
  }
  const newPage = async (viewport, scheme = 'light') => {
    const context = await browser.newContext({ viewport, deviceScaleFactor: 2, isMobile: viewport.width < 600, hasTouch: true, locale: 'vi-VN', colorScheme: scheme })
    const page = await context.newPage()
    await page.addInitScript(() => {
      window.__csp = []
      document.addEventListener('securitypolicyviolation', (e) => window.__csp.push(`${e.violatedDirective} ${e.blockedURI}`))
    })
    page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
    page.on('pageerror', (e) => errors.push(String(e)))
    return { context, page }
  }
  const byId = (page, id) => page.locator(`[data-testid="${id}"]`)
  const ready = (page) => page.waitForSelector('.shell[data-ready="1"]', { timeout: 15000 })
  const open = async (page) => {
    await page.goto(`${env.base}/app/`)
    await byId(page, 'machines-view').waitFor({ timeout: 15000 })
    await ready(page)
  }
  const reload = async (page) => {
    await page.reload()
    await byId(page, 'machines-view').waitFor({ timeout: 15000 })
    await ready(page)
  }
  const pair = async (page, peer) => {
    await byId(page, 'tab-debug').click()
    await byId(page, 'pair-qr').fill(peer.info.qr)
    await byId(page, 'pair-go').click()
    try {
      await page.waitForFunction(() => document.querySelector('[data-testid="debug-message"]')?.textContent?.includes('Orrery Desk'), null, { timeout: 15000 })
    } catch (error) {
      const message = await byId(page, 'debug-message').textContent().catch(() => '(none)')
      throw new Error(`pairing did not finish: ${message} | peer: ${peer.log.slice(-6).join(' ; ')} | page: ${errors.slice(-8).join(' ; ')}`)
    }
    await byId(page, 'tab-machines').click()
  }
  const status = (page, id) => byId(page, `machine-${id}`).locator('[data-testid="link-chip"]')
  const expectStatus = async (page, id, value, timeout = 15000) => {
    await page.waitForFunction(([sel, v]) => document.querySelector(sel)?.getAttribute('data-status') === v, [`[data-testid="machine-${id}"] [data-testid="link-chip"]`, value], { timeout })
  }
  const shoot = async (page, file) => {
    const before = { csp: (await page.evaluate(() => window.__csp.length)), errors: errors.length }
    await page.screenshot({ caret: 'initial', path: file })
    await wait(250)
    const after = await page.evaluate(() => window.__csp.length)
    const added = errors.splice(before.errors).filter((e) => !/Refused to apply a stylesheet/.test(e))
    errors.push(...added)
    if (after - before.csp <= 1) await page.evaluate((n) => window.__csp.splice(n), before.csp)
  }
  const text = async (locator) => (await locator.textContent())?.replace(/\s+/g, ' ').trim() ?? ''
  const expectText = async (page, selector, pattern, timeout = 10000) => {
    await page.waitForFunction(([sel, src]) => new RegExp(src).test(document.querySelector(sel)?.textContent ?? ''), [selector, pattern.source], { timeout })
  }

  try {
    const peer = await startPeer(env.peerExe, env.relayHttp)
    peers.push(peer)
    const id = peer.info.machine
    const { page, context } = await newPage(PHONE)

    await step('boots empty with the wasm core, no CSP violation, no horizontal scroll', async () => {
      await open(page)
      await byId(page, 'machines-empty').waitFor()
      const csp = await page.evaluate(() => window.__csp)
      if (csp.length) throw new Error(`CSP violations: ${csp.join(' | ')}`)
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
      if (overflow > 0) throw new Error(`horizontal overflow ${overflow}`)
      if ((await page.locator('link[rel="manifest"]').count()) !== 1) throw new Error('manifest link missing')
      if (await byId(page, 'boot-error').count()) throw new Error('boot error shown')
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

    await step('pairs with the QR ticket through the relay', async () => {
      await pair(page, peer)
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
      await byId(page, 'tab-debug').click()
      await expectText(page, `[data-testid="debug-machine-${id}"]`, /seq 2 · ws rev 2 · 1 tab · 4 terminal · mới/)
      await byId(page, 'tab-machines').click()
    })

    await step('screenshots: Machines, light and dark, phone and iPad', async () => {
      await wait(400)
      for (const [form, viewport] of [['phone', PHONE], ['ipad', IPAD]]) {
        for (const scheme of ['light', 'dark']) {
          await page.setViewportSize(viewport)
          await page.emulateMedia({ colorScheme: scheme })
          await wait(250)
          const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
          if (overflow > 0) throw new Error(`${form} overflow ${overflow}`)
          const detail = await byId(page, 'detail-pane').isVisible()
          if (detail !== (form === 'ipad')) throw new Error(`detail pane visibility ${detail} on ${form}`)
          await shoot(page, join(env.qaDir, `machines-${form}-${scheme}-${browserName}.png`))
        }
      }
      await page.setViewportSize(PHONE)
      await page.emulateMedia({ colorScheme: 'light' })
    })

    await step('desktop gone: card turns stale, reload shows the saved snapshot from IndexedDB at once', async () => {
      peer.stop()
      await expectStatus(page, id, 'reconnecting', 20000)
      await wait(500)
      await page.reload()
      await expectText(page, `[data-testid="machine-${id}"] [data-testid="machine-counts"]`, /1 đang chạy/, 5000)
      const stale = await byId(page, `machine-${id}`).locator('[data-testid="machine-freshness"]').getAttribute('data-stale')
      if (stale !== '1') throw new Error('cached view not marked stale')
      if ((await status(page, id).getAttribute('data-status')) === 'ready') throw new Error('unexpectedly ready')
      await byId(page, 'tab-debug').click()
      await expectText(page, `[data-testid="debug-machine-${id}"]`, /seq 2 · ws rev 2 · 1 tab · 4 terminal · cũ/, 5000)
      await shoot(page, join(env.qaDir, `machines-stale-phone-light-${browserName}.png`))
      await byId(page, 'tab-machines').click()
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
    const fresh = await newPage(PHONE)

    await step('revoked device (close 4401): machine data wiped from the UI and IndexedDB, notice shown, stays wiped after reload', async () => {
      await open(fresh.page)
      await pair(fresh.page, second)
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
