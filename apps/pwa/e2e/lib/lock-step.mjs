import { join } from 'node:path'
import { PHONE, wait } from './ctx.mjs'

export async function lockStep(c) {
  const { step, byId, env, browserName, newPage, open, reload, shoot } = c
  const base = env.base.replace('127.0.0.1', 'localhost')
  const lock = await newPage(PHONE, 'light', base)
  const page = lock.page
  let cdp = null
  let authenticatorId = null

  await step('app lock: settings toggle, reload shows the lock screen without app data, unlock opens it; WebKit without WebAuthn only checks the unsupported UI', async () => {
    if (browserName === 'chromium') {
      cdp = await lock.context.newCDPSession(page)
      await cdp.send('WebAuthn.enable')
      const added = await cdp.send('WebAuthn.addVirtualAuthenticator', {
        options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true },
      })
      authenticatorId = added.authenticatorId
    }
    await open(page, base)
    await byId(page, 'home-settings').click()
    await byId(page, 'settings-view').waitFor()
    const supported = await page.evaluate(async () => (await window.PublicKeyCredential?.isUserVerifyingPlatformAuthenticatorAvailable?.()) === true)
    if (!supported) {
      await page.waitForFunction(() => document.querySelector('[data-testid="lock-toggle"]')?.disabled === true)
      const body = await byId(page, 'settings-security').textContent()
      if (!/không hỗ trợ/.test(body ?? '')) throw new Error('missing unsupported message')
      await shoot(page, join(env.qaDir, `settings-lock-unsupported-phone-light-${browserName}.png`))
      return
    }
    await byId(page, 'lock-toggle').click()
    await byId(page, 'lock-timeout').waitFor({ timeout: 8000 })
    await byId(page, 'lock-timeout-900000').click()
    if ((await byId(page, 'lock-timeout-900000').getAttribute('aria-checked')) !== 'true') throw new Error('timeout not chosen')
    await cdp.send('WebAuthn.setUserVerified', { authenticatorId, isUserVerified: false })
    await page.reload()
    await byId(page, 'lock-screen').waitFor({ timeout: 10000 })
    await wait(800)
    if (await page.locator('.shell').count()) throw new Error('app shell rendered while locked')
    if (await byId(page, 'settings-view').count()) throw new Error('settings rendered while locked')
    await shoot(page, join(env.qaDir, `lock-phone-light-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'dark' })
    await wait(250)
    await shoot(page, join(env.qaDir, `lock-phone-dark-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'light' })
    await cdp.send('WebAuthn.setUserVerified', { authenticatorId, isUserVerified: true })
    await byId(page, 'lock-unlock').click()
    await byId(page, 'settings-view').waitFor({ timeout: 10000 })
    if (await byId(page, 'lock-screen').count()) throw new Error('lock screen stayed')
    if ((await byId(page, 'lock-timeout-900000').getAttribute('aria-checked')) !== 'true') throw new Error('timeout lost after unlock')
    await byId(page, 'lock-toggle').click()
    await byId(page, 'confirm-ok').click()
    await byId(page, 'lock-timeout').waitFor({ state: 'detached', timeout: 5000 })
    await reload(page)
    if (await byId(page, 'lock-screen').count()) throw new Error('lock came back after turning it off')
  })
  await lock.context.close()
}
