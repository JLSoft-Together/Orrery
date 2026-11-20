import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { PHONE, wait } from './ctx.mjs'
import { startPeer } from './procs.mjs'

const AUTH = { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true }

async function pairUntilNext(c, page, peer, base) {
  const { byId } = c
  await c.open(page, base)
  await byId(page, 'empty-pair').click({ timeout: 8000 })
  await byId(page, 'pair-view').waitFor({ timeout: 5000 })
  await byId(page, 'pair-tab-paste').dispatchEvent('click')
  await byId(page, 'pair-code').fill(peer.info.qr)
  await byId(page, 'pair-submit').click({ timeout: 8000 })
  await page.waitForFunction(() => document.querySelector('[data-testid="lock-offer"]') || document.querySelector('[data-testid="chat-home"]'), null, { timeout: 20000 })
}

const prefs = (page) => page.evaluate(() => JSON.parse(localStorage.getItem('ccm.pwa.prefs') ?? '{}'))

export async function lockOfferStep(c, env) {
  const { step, byId, browserName, peers, shoot } = c
  const base = env.base.replace('127.0.0.1', 'localhost')
  await step('lock offer after the first pairing: shown when the device can, a refused try is a light note, Later is remembered, Turn on enables the lock; WebKit without WebAuthn goes straight to Monitor', async () => {
    const first = await startPeer(env.peerExe, env.relayHttp)
    peers.push(first)
    const a = await c.newPage(PHONE)
    try {
      let cdp = null
      let authenticatorId = null
      if (browserName === 'chromium') {
        cdp = await a.context.newCDPSession(a.page)
        await cdp.send('WebAuthn.enable')
        authenticatorId = (await cdp.send('WebAuthn.addVirtualAuthenticator', { options: AUTH })).authenticatorId
      }
      await pairUntilNext(c, a.page, first, base)
      if (!(await byId(a.page, 'lock-offer').count())) {
        if (browserName === 'chromium') throw new Error('no lock offer with a platform authenticator')
        if ((await prefs(a.page)).lockOffered === true) throw new Error('marked as asked without asking')
        return
      }
      const mist = join(dirname(env.qaDirE), 'WP-68b3')
      mkdirSync(mist, { recursive: true })
      await wait(400)
      await shoot(a.page, join(env.qaDirE, `lock-offer-phone-light-${browserName}.png`))
      await shoot(a.page, join(mist, `lock-phone-light-${browserName}.png`))
      await a.page.emulateMedia({ colorScheme: 'dark' })
      await wait(300)
      await shoot(a.page, join(env.qaDirE, `lock-offer-phone-dark-${browserName}.png`))
      await shoot(a.page, join(mist, `lock-phone-dark-${browserName}.png`))
      await a.page.emulateMedia({ colorScheme: 'light' })

      await cdp.send('WebAuthn.setUserVerified', { authenticatorId, isUserVerified: false })
      await byId(a.page, 'lock-offer-enable').click({ timeout: 8000 })
      await byId(a.page, 'lock-offer-fail').waitFor({ timeout: 8000 })
      await shoot(a.page, join(env.qaDirE, `lock-offer-fail-phone-light-${browserName}.png`))
      if ((await prefs(a.page)).lockEnabled === true) throw new Error('lock enabled after a refused try')
      await byId(a.page, 'lock-offer-later').click({ timeout: 8000 })
      await byId(a.page, 'chat-home').waitFor({ timeout: 8000 })
      const saved = await prefs(a.page)
      if (saved.lockOffered !== true || saved.lockEnabled === true) throw new Error(`prefs after Later ${JSON.stringify(saved)}`)
    } finally {
      await a.context.close()
    }

    if (browserName !== 'chromium') return
    const second = await startPeer(env.peerExe, env.relayHttp)
    peers.push(second)
    const b = await c.newPage(PHONE)
    try {
      const cdp = await b.context.newCDPSession(b.page)
      await cdp.send('WebAuthn.enable')
      await cdp.send('WebAuthn.addVirtualAuthenticator', { options: AUTH })
      await pairUntilNext(c, b.page, second, base)
      await byId(b.page, 'lock-offer').waitFor({ timeout: 5000 })
      await byId(b.page, 'lock-offer-enable').click({ timeout: 8000 })
      await byId(b.page, 'chat-home').waitFor({ timeout: 10000 })
      const saved = await prefs(b.page)
      if (saved.lockEnabled !== true || !saved.credentialId || saved.lockOffered !== true) throw new Error(`prefs after Turn on ${JSON.stringify(saved)}`)
      await byId(b.page, 'back').click({ timeout: 8000 })
      await byId(b.page, 'machines-view').waitFor({ timeout: 5000 })
      await byId(b.page, 'home-settings').click({ timeout: 8000 })
      await byId(b.page, 'lock-timeout').waitFor({ timeout: 5000 })
    } finally {
      await b.context.close()
    }
  })
}
