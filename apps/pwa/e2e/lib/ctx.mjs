import { writeQrVideo } from './y4m.mjs'

export const wait = (ms) => new Promise((r) => setTimeout(r, ms))
export const PHONE = { width: 390, height: 844 }
export const IPAD = { width: 820, height: 1180 }

export async function idbCounts(page) {
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

export function makeCtx({ browser, browserName, env, record, videoPath }) {
  const errors = []
  const peers = []
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
  const newPage = async (viewport, scheme = 'light', base = env.base) => {
    const context = await browser.newContext({
      viewport,
      deviceScaleFactor: 2,
      isMobile: viewport.width < 600,
      hasTouch: true,
      locale: 'vi-VN',
      colorScheme: scheme,
      permissions: browserName === 'chromium' ? ['camera'] : [],
    })
    const page = await context.newPage()
    await page.addInitScript(() => {
      window.__csp = []
      document.addEventListener('securitypolicyviolation', (e) => window.__csp.push(`${e.violatedDirective} ${e.blockedURI}`))
    })
    page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
    page.on('pageerror', (e) => errors.push(String(e)))
    return { context, page, base }
  }
  const byId = (page, id) => page.locator(`[data-testid="${id}"]`)
  const ready = (page) => page.waitForSelector('.shell[data-ready="1"]', { timeout: 15000 })
  const open = async (page, base = env.base, path = '/app/') => {
    await page.goto(`${base}${path}`)
    await ready(page)
  }
  const reload = async (page) => {
    await page.reload()
    await ready(page)
  }
  const home = async (page) => {
    await page.goto(`${new URL(page.url()).origin}/app/`)
    await ready(page)
    await byId(page, 'machines-view').waitFor({ timeout: 10000 })
  }
  const failPair = async (page, peer, errorsList) => {
    const message = await byId(page, 'pair-error').getAttribute('data-code').catch(() => '(none)')
    throw new Error(`pairing did not finish: ${message} | peer: ${peer.log.slice(-6).join(' ; ')} | page: ${errorsList.slice(-8).join(' ; ')}`)
  }
  const startPair = async (page) => {
    const cta = byId(page, 'empty-pair')
    if (await cta.count()) await cta.click()
    else await byId(page, 'pair-fab').click()
    await byId(page, 'pair-view').waitFor({ timeout: 5000 })
  }
  const finishPair = async (page, peer) => {
    try {
      await byId(page, 'monitor-view').waitFor({ timeout: 20000 })
    } catch {
      await failPair(page, peer, errors)
    }
    await byId(page, 'back').click()
    await byId(page, 'machines-view').waitFor({ timeout: 5000 })
  }
  const pairPaste = async (page, peer, deviceName = 'iPhone E2E') => {
    await startPair(page)
    await byId(page, 'pair-tab-paste').click()
    await byId(page, 'pair-code').fill(peer.info.qr)
    await byId(page, 'pair-name').fill(deviceName)
    await byId(page, 'pair-submit').click()
    await finishPair(page, peer)
  }
  const pairScan = async (page, peer) => {
    writeQrVideo(videoPath, peer.info.qr)
    await startPair(page)
    await byId(page, 'qr-scanner').waitFor({ timeout: 5000 })
    await finishPair(page, peer)
  }
  const status = (page, id) => byId(page, `machine-${id}`).locator('[data-testid="link-chip"]')
  const expectStatus = async (page, id, value, timeout = 15000) => {
    await page.waitForFunction(([sel, v]) => document.querySelector(sel)?.getAttribute('data-status') === v, [`[data-testid="machine-${id}"] [data-testid="link-chip"]`, value], { timeout })
  }
  const shoot = async (page, file) => {
    const before = { csp: await page.evaluate(() => window.__csp.length), errors: errors.length }
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
  const noOverflow = async (page, label) => {
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
    if (overflow > 0) throw new Error(`${label} horizontal overflow ${overflow}`)
  }
  return { browserName, env, errors, peers, step, newPage, byId, ready, open, reload, home, pairPaste, pairScan, status, expectStatus, shoot, text, expectText, noOverflow }
}
