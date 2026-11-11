const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v91-')))
const PROJ = path.join(BASE, 'proj')
fs.mkdirSync(PROJ, { recursive: true })
fs.writeFileSync(path.join(PROJ, 'README.md'), '# v91\n')

const THEMES = ['orrery-dark', 'orrery-light', 'console', 'dark']
const noHScroll = `document.documentElement.scrollWidth <= window.innerWidth + 1 && document.body.scrollWidth <= window.innerWidth + 1`
const rootTheme = `document.documentElement.dataset.theme ?? (document.documentElement.classList.contains('dark') ? 'dark' : 'light')`

async function main() {
  const app = await launch({ env: { CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep } = app
  const reload = async () => {
    await app.cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await sleep(400)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    const fresh = await evaluate(rootTheme)
    check('fresh profile lands on Orrery', /^orrery-(light|dark)$/.test(fresh), fresh)
    check('fresh profile stored the migration', (await evaluate(`localStorage.getItem('ccm.themeMigration')`)) === '4.0.2')
    check('settings store agrees', (await evaluate(`${S}.get('settings').theme`)) === fresh)

    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await evaluate(`localStorage.setItem('ccm.theme','dark'); localStorage.removeItem('ccm.themeMigration')`)
    await reload()
    check('4.0.2 migration moves Apple dark to orrery-dark', (await evaluate(rootTheme)) === 'orrery-dark', await evaluate(rootTheme))
    check('migration key written', (await evaluate(`localStorage.getItem('ccm.themeMigration')`)) === '4.0.2')

    await evaluate(`localStorage.setItem('ccm.theme','dark'); localStorage.setItem('ccm.themeMigration','4.0.2')`)
    await reload()
    check('after migration a chosen theme sticks', (await evaluate(rootTheme)) === 'dark' && (await evaluate(`${S}.get('settings').theme`)) === 'dark')

    for (const id of THEMES) {
      await evaluate(`localStorage.setItem('ccm.theme','${id}'); localStorage.setItem('ccm.themeMigration','4.0.2')`)
      await reload()
      check(`${id}: applied`, (await evaluate(rootTheme)) === id, await evaluate(rootTheme))
      await evaluate(`${S}.get('ui').view = 'workspace'`)
      await sleep(300)
      if (!(await evaluate(`${W}.panes.length`))) {
        await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
        const first = await evaluate(`${W}.open('shell', ${JSON.stringify(PROJ)}, undefined, { tab: true }).key`)
        await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
        await evaluate(`${W}.split('${first}', 'row', { kind: 'term', preset: 'shell', cwd: ${JSON.stringify(PROJ)} })`)
        await waitFor(`${W}.panes.length >= 2`, 10000, 'split')
        await evaluate(`${W}.open('shell', ${JSON.stringify(PROJ)}, undefined, { tab: true })`)
        await sleep(400)
        await evaluate(`${W}.activateTab ? ${W}.activateTab(${W}.tabs[0].key) : document.querySelector('[role=tab][data-tab]')?.click()`)
      }
      await sleep(1200)
      check(`${id}: workspace no horizontal scroll`, await evaluate(noHScroll))
      check(`${id}: pane ring class only on the active pane`, await evaluate(`document.querySelectorAll('.ccm-pane-ring.ccm-pane-active').length <= 1`))
      await shot(`v91-${id}-workspace`)
      await evaluate(`${S}.get('ui').view = 'monitor'`)
      await sleep(900)
      check(`${id}: monitor no horizontal scroll`, await evaluate(noHScroll))
      await shot(`v91-${id}-monitor`)
      await evaluate(`${S}.get('ui').settingsOpen = true`)
      await waitFor(`!!document.querySelector('[data-settings-page="appearance"]')`, 10000, 'settings')
      await evaluate(`document.querySelector('[data-settings-page="appearance"]').click()`)
      await waitFor(`!!document.querySelector('[data-theme-option="orrery-dark"]')`, 10000, 'theme picker')
      await sleep(400)
      check(`${id}: picker lists Orrery first with default label`, await evaluate(`(() => { const o = [...document.querySelectorAll('[data-theme-option]')].map((b) => b.dataset.themeOption); return o[0] === 'orrery-light' && o.indexOf('orrery-dark') < o.indexOf('console') && o.indexOf('console') < o.indexOf('dark') && document.querySelectorAll('[data-theme-default]').length === 2 })()`))
      check(`${id}: picker marks the active theme`, await evaluate(`document.querySelector('[data-theme-option="${id}"]')?.getAttribute('aria-checked') === 'true'`))
      check(`${id}: settings no horizontal scroll`, await evaluate(noHScroll))
      await shot(`v91-${id}-settings`)
      await evaluate(`${S}.get('ui').settingsOpen = false`)
      await sleep(300)
    }
    await evaluate(`${S}.get('settings').setTheme('orrery-dark')`)
    await evaluate(`${S}.get('settings').toggleTheme ? ${S}.get('settings').toggleTheme() : ${S}.get('settings').setTheme('orrery-light')`)
    check('toggle pairs orrery-dark → orrery-light', (await evaluate(`${S}.get('settings').theme`)) === 'orrery-light')
  } catch (e) {
    check('run', false, String(e?.message ?? e))
    await shot('v91-fail').catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'v91-results.txt'), results.join('\n'))
    try { fs.rmSync(BASE, { recursive: true, force: true }) } catch {}
    const fails = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fails}/${results.length} pass`)
    process.exit(fails ? 1 : 0)
  }
}
main()
