const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-v89-'))
const cfg = path.join(root, 'claude')
const proj = path.join(root, 'proj')
const key = proj.replace(/\\/g, '/')
fs.mkdirSync(path.join(cfg, 'projects'), { recursive: true })
fs.mkdirSync(proj, { recursive: true })
const claudeJson = path.join(cfg, '.claude.json')
fs.writeFileSync(claudeJson, JSON.stringify({ numStartups: 3, mcpServers: { 'demo-a': { command: 'node', args: ['a.js'] }, orrery: { command: 'node', args: ['o.js'] } }, projects: { [key]: { allowedTools: [], disabledMcpServers: ['demo-b'] } }, zzz: { keep: [1, 2.5, 'x'] } }, null, 2))
fs.writeFileSync(path.join(cfg, 'settings.json'), JSON.stringify({ enabledPlugins: { 'demo-plugin@mk': true } }, null, 2))
const before = JSON.parse(fs.readFileSync(claudeJson, 'utf8'))
const readJson = (f) => { try { return JSON.parse(fs.readFileSync(f, 'utf8')) } catch { return null } }

async function main() {
  const app = await launch({ env: { CLAUDE_CONFIG_DIR: cfg, CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep } = app
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.theme','dark')`)
    await app.cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 20000, 'reloaded')
    check('bridge has extProject', await evaluate(`typeof window.ccmDesktop.extProject === 'function' && typeof window.ccmDesktop.extProjectToggle === 'function'`))

    await evaluate(`${S}.get('ui').sessionTools = { dir: ${JSON.stringify(proj)} }`)
    await waitFor(`!!document.querySelector('[data-session-tools] [data-tools-row="demo-a"]')`, 20000, 'sheet rows')
    const rows = await evaluate(`[...document.querySelectorAll('[data-tools-row]')].map(r => r.dataset.toolsRow + ':' + r.querySelector('[role=switch]').getAttribute('aria-checked') + ':' + (r.querySelector('[role=switch]').disabled ? 'off' : 'on')).join(' ')`)
    check('rows list mcp + plugin with state', /demo-a:true:on/.test(rows) && /demo-b:false:on/.test(rows) && /demo-plugin@mk:true:on/.test(rows), rows)
    check('orrery locked', /orrery:true:off/.test(rows), rows)
    check('no horizontal scroll', await evaluate(`document.documentElement.scrollWidth <= window.innerWidth`))
    await shot('v89-01-sheet')

    await evaluate(`document.querySelector('[data-tools-row="demo-a"] [role=switch]').click()`)
    await waitFor(`document.querySelector('[data-tools-row="demo-a"] [role=switch]').getAttribute('aria-checked') === 'false'`, 5000, 'mcp off')
    await sleep(2500)
    let j = readJson(claudeJson)
    check('mcp off written to projects[dir].disabledMcpServers', JSON.stringify(j?.projects?.[key]?.disabledMcpServers?.slice().sort()) === JSON.stringify(['demo-a', 'demo-b']), JSON.stringify(j?.projects?.[key]))
    check('other keys kept', JSON.stringify(j?.zzz) === JSON.stringify(before.zzz) && j?.numStartups === 3 && JSON.stringify(j?.mcpServers) === JSON.stringify(before.mcpServers) && Array.isArray(j?.projects?.[key]?.allowedTools))
    check('backup written', fs.existsSync(claudeJson + '.ccm-backup'))
    check('banner shown, no restart without pane', await evaluate(`!!document.querySelector('[data-tools-banner]') && !document.querySelector('[data-tools-restart]')`))

    await evaluate(`document.querySelector('[data-tools-row="demo-plugin@mk"] [role=switch]').click()`)
    await waitFor(`document.querySelector('[data-tools-row="demo-plugin@mk"] [role=switch]').getAttribute('aria-checked') === 'false'`, 5000, 'plugin off')
    await sleep(500)
    const local = path.join(proj, '.claude', 'settings.local.json')
    check('plugin off → settings.local.json false', readJson(local)?.enabledPlugins?.['demo-plugin@mk'] === false, JSON.stringify(readJson(local)))
    check('user settings untouched', readJson(path.join(cfg, 'settings.json'))?.enabledPlugins?.['demo-plugin@mk'] === true)
    await shot('v89-02-changed')

    await evaluate(`document.querySelector('[data-tools-row="demo-plugin@mk"] [role=switch]').click()`)
    await waitFor(`document.querySelector('[data-tools-row="demo-plugin@mk"] [role=switch]').getAttribute('aria-checked') === 'true'`, 5000, 'plugin on')
    await sleep(500)
    check('plugin on → key removed', readJson(local)?.enabledPlugins?.['demo-plugin@mk'] === undefined, JSON.stringify(readJson(local)))

    for (const n of ['demo-a', 'demo-b']) {
      await evaluate(`document.querySelector('[data-tools-row="${n}"] [role=switch]').click()`)
      await waitFor(`document.querySelector('[data-tools-row="${n}"] [role=switch]').getAttribute('aria-checked') === 'true'`, 5000, n + ' on')
      await sleep(2000)
    }
    j = readJson(claudeJson)
    check('all on → disabledMcpServers key removed', j?.projects?.[key] && !('disabledMcpServers' in j.projects[key]), JSON.stringify(j?.projects?.[key]))

    const bad = await evaluate(`window.ccmDesktop.extProjectToggle({ dir: ${JSON.stringify(proj)}, kind: 'mcp', name: 'orrery', enabled: false })`)
    check('orrery toggle refused', bad?.ok === false && bad?.code === 'locked', JSON.stringify(bad))
    const bad2 = await evaluate(`window.ccmDesktop.extProjectToggle({ dir: ${JSON.stringify(proj)}, kind: 'mcp', name: '-x', enabled: false })`)
    check('dash name refused', bad2?.ok === false && bad2?.code === 'badArg', JSON.stringify(bad2))

    await evaluate(`document.querySelector('[data-session-tools]').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))`)
    await sleep(400)
    check('Esc closes sheet', await evaluate(`!document.querySelector('[data-session-tools]')`))

    await evaluate(`${S}.get('ui').setView('settings')`)
    await sleep(800)
    check('settings has contextWarn row', await evaluate(`!!document.querySelector('[data-setting-row="contextWarn"]') || ${S}.get('settings').contextWarnTokens === 500000`), String(await evaluate(`${S}.get('settings').contextWarnTokens`)))
  } catch (e) {
    check('run', false, String(e?.message ?? e))
    await shot('v89-fail').catch(() => {})
  } finally {
    await app.stop()
    fs.writeFileSync(path.join(OUT, 'v89-results.txt'), results.join('\n'))
    fs.rmSync(root, { recursive: true, force: true })
    const fails = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fails}/${results.length} pass`)
    process.exit(fails ? 1 : 0)
  }
}
main()
