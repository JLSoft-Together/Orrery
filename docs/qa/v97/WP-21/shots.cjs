const os = require('node:os')
const path = require('node:path')
const fs = require('node:fs')
const REPO = path.resolve(__dirname, '../../../..')
const OUT_DIR = path.join(os.tmpdir(), 'ccm-e2e-wp21')
process.env.CCM_E2E_SHELL = 'tauri'
process.env.CCM_E2E_PORT = '4431'
process.env.CCM_E2E_DBG = '9351'
process.env.CCM_E2E_OUT = OUT_DIR
const { launch } = require(path.join(REPO, 'apps/desktop/e2e/cdp.cjs'))
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const UI = `${S}.get('ui')`
const ST = `${S}.get('settings')`
const SHOTS = __dirname

const MOCK = `(() => {
  for (const k of ['ccm.setupDone', 'ccm.tourDone', 'ccm.tourWorkspaceDone']) localStorage.setItem(k, '1')
  const m = (window.__m = { relay: { state: 'none', updateAvailable: false, hasToken: false }, push: { configured: false, tokens: 0 }, token: { ok: true, accounts: [{ id: 'a1', name: 'Jin Vu Personal' }, { id: 'a2', name: 'Orrery Team' }] }, emit: null })
  window.__ccmE2eSetupBridge = {
    remoteRelayStatus: async () => m.relay,
    remoteRelayToken: async () => m.token,
    remoteRelayDeploy: () => new Promise(() => {}),
    remoteRelayRemove: async () => ({ ok: true, workerDeleted: true }),
    onRemoteRelayProgress: (h) => { m.emit = h; return () => {} },
    remotePushStatus: async () => m.push,
    remotePushSetup: async () => ({ ok: false, error: 'no_android_client' }),
    remotePushTest: async () => ({ ok: true, sent: 1 }),
    remotePushRemove: async () => ({ ok: true }),
  }
})()`

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

;(async () => {
  const app = await launch({ env: { CCM_REMOTE_PORT: '4331', CCM_KEYRING_SERVICE: 'Orrery E2E WP21' } })
  const { evaluate, waitFor, shot, cmd } = app
  const names = []
  try {
    await waitFor(`!!window.ccmDesktop && !!${UI}`, 40000, 'app ready')
    await cmd('Emulation.setDeviceMetricsOverride', { width: 1180, height: 900, deviceScaleFactor: 1, mobile: false })
    console.log('hasRemote', await evaluate(`typeof window.ccmDesktop.remoteStatus`), 'relayIpc', await evaluate(`typeof window.ccmDesktop.remoteRelayStatus`), 'pushIpc', await evaluate(`typeof window.ccmDesktop.remotePushStatus`))
    await cmd('Page.addScriptToEvaluateOnNewDocument', { source: MOCK })
    await cmd('Page.reload')
    await waitFor(`!!window.ccmDesktop && !!${UI}`, 40000, 'reloaded')
    if ((await evaluate(`typeof window.__ccmE2eSetupBridge`)) !== 'object') throw new Error('mock not applied, refusing to run against the real shell')
    await evaluate(`[...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Để sau')?.click()`)
    await sleep(400)
    const mode = async (theme) => {
      await evaluate(`${ST}.setTheme('${theme}')`)
      await sleep(300)
    }
    const open = async () => {
      await evaluate(`${UI}.settingsPage = 'general'; ${UI}.settingsOpen = true`)
      await sleep(200)
      await evaluate(`${UI}.settingsPage = 'remote'`)
      await sleep(600)
    }
    const enable = async () => {
      await evaluate(`(async () => { await window.ccmDesktop.remoteEnable(true) })()`)
      await sleep(400)
    }
    const view = async (selector) => {
      await evaluate(`(document.querySelector('${selector}')?.closest('[data-setting-row]') || document.querySelector('${selector}'))?.scrollIntoView({ block: 'start' })`)
      await sleep(450)
    }
    const take = async (name, selector) => {
      await view(selector)
      await shot(name)
      fs.copyFileSync(path.join(OUT_DIR, name + '.png'), path.join(SHOTS, name + '.png'))
      names.push(name)
      console.log('shot', name)
    }
    const setText = (sel, v) => evaluate(`(() => { const e = document.querySelector('${sel}'); e.value = ${JSON.stringify(v)}; e.dispatchEvent(new Event('input', { bubbles: true })); return true })()`)
    const click = async (sel) => {
      await evaluate(`document.querySelector('${sel}').click()`)
      await sleep(400)
    }
    const drop = async (files) => {
      await evaluate(`(() => {
        const dt = new DataTransfer()
        for (const [n, t] of ${JSON.stringify(files)}) dt.items.add(new File([t], n, { type: 'application/json' }))
        const el = document.querySelector('[data-push-drop]')
        el.dispatchEvent(new DragEvent('dragover', { dataTransfer: dt, bubbles: true, cancelable: true }))
        el.dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true }))
      })()`)
      await sleep(500)
    }
    const GS = ['google-services.json', JSON.stringify({ project_info: { project_id: 'orrery-demo', project_number: '123' }, client: [] })]
    const SA = ['orrery-demo-firebase-adminsdk.json', JSON.stringify({ type: 'service_account', project_id: 'orrery-demo', client_email: 'x@orrery-demo.iam.gserviceaccount.com' })]
    const SA2 = ['other-key.json', JSON.stringify({ type: 'service_account', project_id: 'other-proj' })]
    const BAD = ['notes.json', JSON.stringify({ hello: 'world' })]
    const reset = (relay, push) => evaluate(`Object.assign(window.__m, { relay: ${JSON.stringify(relay)}, push: ${JSON.stringify(push)} })`)

    for (const theme of ['orrery-light', 'galaxy']) {
      const tag = theme === 'galaxy' ? 'dark' : 'light'
      await mode(theme)
      await reset({ state: 'none', updateAvailable: false, hasToken: false }, { configured: false, tokens: 0 })
      await open()
      await take(`relay-1-none-${tag}`, '[data-relay-setup]')
      await setText('[data-relay-token]', 'cfut_EXAMPLEEXAMPLEEXAMPLE')
      console.log('btn', await evaluate(`JSON.stringify({d: document.querySelector('[data-relay-check]')?.disabled, v: document.querySelector('[data-relay-token]')?.value})`))
      await click('[data-relay-check]')
      console.log('after', await evaluate(`JSON.stringify({err: document.querySelector('[data-relay-error]')?.dataset.code, ok: !!document.querySelector('[data-relay-token-ok]'), tok: document.querySelector('[data-relay-token]')?.value})`))
      await sleep(600)
      console.log('accounts', await evaluate(`document.querySelectorAll('[data-account]').length`))
      await click('[data-account="a1"] input')
      await take(`relay-2-token-ok-${tag}`, '[data-relay-steps]')
      await click('[data-relay-deploy]')
      await evaluate(`window.__m.emit({ step: 'upload' })`)
      await sleep(500)
      await take(`relay-3-deploying-${tag}`, '[data-relay-setup]')

      await reset({ state: 'error', updateAvailable: false, hasToken: true, error: 'token_scope' }, { configured: false, tokens: 0 })
      await open()
      await take(`relay-4-error-${tag}`, '[data-relay-setup]')

      await reset({ state: 'ready', url: 'https://orrery-4f2a9c.jinvu.workers.dev', account: 'Jin Vu Personal', updateAvailable: true, hasToken: true, warning: 'health' }, { configured: false, tokens: 0 })
      await open()
      await take(`relay-5-ready-${tag}`, '[data-relay-setup]')
      await click('[data-relay-remove]')
      await take(`relay-6-remove-ask-${tag}`, '[data-relay-setup]')

      await reset({ state: 'none', updateAvailable: false, hasToken: false }, { configured: false, tokens: 0 })
      await open()
      await take(`push-1-none-${tag}`, '[data-push-setup]')
      await drop([GS])
      await take(`push-2-one-file-${tag}`, '[data-push-setup]')
      await drop([SA2, BAD])
      await take(`push-3-mismatch-${tag}`, '[data-push-setup]')
      await drop([SA])
      await take(`push-4-both-${tag}`, '[data-push-setup]')
      await click('[data-push-send]')
      await take(`push-5-error-${tag}`, '[data-push-setup]')

      await reset({ state: 'none', updateAvailable: false, hasToken: false }, { configured: true, projectId: 'orrery-demo', tokens: 2 })
      await open()
      await click('[data-push-test]')
      await take(`push-6-ready-${tag}`, '[data-push-setup]')
    }
    fs.writeFileSync(path.join(SHOTS, 'shots.txt'), names.join('\n') + '\n')
  } finally {
    app.stop()
  }
})().catch((e) => {
  console.error(e)
  process.exit(1)
})
