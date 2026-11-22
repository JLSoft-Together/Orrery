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
const SID = '5b1c2d3e-0000-4000-8000-00000000a089'
fs.mkdirSync(path.join(cfg, 'projects', 'F--v89'), { recursive: true })
fs.mkdirSync(proj, { recursive: true })
const claudeJson = path.join(cfg, '.claude.json')
const extra = Object.fromEntries(['c', 'd', 'e', 'f', 'g', 'h'].map((n) => [`demo-${n}`, { command: 'node', args: [`${n}.js`] }]))
fs.writeFileSync(claudeJson, JSON.stringify({ numStartups: 3, mcpServers: { 'demo-a': { command: 'node', args: ['a.js'] }, orrery: { command: 'node', args: ['o.js'] }, ...extra }, projects: { [key]: { allowedTools: [], disabledMcpServers: ['demo-b'] } }, zzz: { keep: [1, 2.5, 'x'] } }, null, 2))
fs.writeFileSync(path.join(cfg, 'settings.json'), JSON.stringify({ enabledPlugins: { 'demo-plugin@mk': true } }, null, 2))
const use = (id, name, input = {}) => JSON.stringify({ type: 'assistant', message: { content: [{ type: 'tool_use', id, name, input }] } })
fs.writeFileSync(path.join(cfg, 'projects', 'F--v89', `${SID}.jsonl`), [use('u1', 'mcp__demo-a__x'), use('u2', 'mcp__demo-a__y'), use('u2', 'mcp__demo-a__y'), use('u3', 'mcp__demo-a__x'), use('u4', 'Skill', { skill: 'demo-plugin:s' }), use('u5', 'Read', { file_path: 'secret' }), ''].join('\n'))
const before = JSON.parse(fs.readFileSync(claudeJson, 'utf8'))
const readJson = (f) => { try { return JSON.parse(fs.readFileSync(f, 'utf8')) } catch { return null } }
const sw = (n) => `document.querySelector('[data-tools-row="${n}"] [role=switch]')`
const checked = (n) => `${sw(n)}.getAttribute('aria-checked')`

async function main() {
  const app = await launch({ env: { CLAUDE_CONFIG_DIR: cfg, CCM_VERIFY_PROCESSES: '0' } })
  const { evaluate, waitFor, shot, sleep } = app
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','vi'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.theme','dark')`)
    await app.cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 20000, 'reloaded')
    check('bridge has extProject', await evaluate(`typeof window.ccmDesktop.extProject === 'function' && typeof window.ccmDesktop.extProjectToggle === 'function' && typeof window.ccmDesktop.extSessionUsage === 'function'`))

    const t0 = Date.now()
    await evaluate(`${S}.get('ui').sessionTools = { dir: ${JSON.stringify(proj)}, sessionId: '${SID}' }`)
    await waitFor(`!!document.querySelector('[data-session-tools] [data-tools-row="demo-a"]')`, 20000, 'sheet rows')
    const firstOpen = Date.now() - t0
    const rows = await evaluate(`[...document.querySelectorAll('[data-tools-row]')].map(r => r.dataset.toolsRow + ':' + r.querySelector('[role=switch]').getAttribute('aria-checked') + ':' + (r.querySelector('[role=switch]').disabled ? 'off' : 'on')).join(' ')`)
    check('rows list mcp + plugin with state', /demo-a:true:on/.test(rows) && /demo-b:false:on/.test(rows) && /demo-plugin@mk:true:on/.test(rows), rows)
    check('orrery locked', /orrery:true:off/.test(rows), rows)
    check('no horizontal scroll', await evaluate(`document.documentElement.scrollWidth <= window.innerWidth`))
    check('filter shown past 8 rows', await evaluate(`!!document.querySelector('[data-tools-filter]')`))
    check('rows laid out in more than one column', await evaluate(`(() => { const r = [...document.querySelectorAll('[data-tools-group="mcp"] [data-tools-row]')].map(e => e.getBoundingClientRect()); return r.length > 1 && Math.abs(r[0].top - r[1].top) < 2 && r[1].left > r[0].right })()`))
    check('apply disabled with no change', await evaluate(`document.querySelector('[data-tools-apply]').disabled === true && document.querySelector('[data-tools-reset]').disabled === true`))

    await waitFor(`!!document.querySelector('[data-tools-row="demo-a"] [data-tools-calls]')`, 5000, 'usage').catch(() => {})
    const calls = await evaluate(`document.querySelector('[data-tools-row="demo-a"] [data-tools-calls]')?.textContent.trim() ?? ''`)
    check('demo-a shows 3 calls in this session', /^3\b/.test(calls), calls)
    check('plugin shows its skill call', /^1\b/.test(await evaluate(`document.querySelector('[data-tools-row="demo-plugin@mk"] [data-tools-calls]')?.textContent.trim() ?? ''`)))
    check('unused hint on enabled MCP rows with 0 calls, not on orrery / off rows', await evaluate(`!!document.querySelector('[data-tools-row="demo-c"] [data-tools-unused]') && !document.querySelector('[data-tools-row="orrery"] [data-tools-unused]') && !document.querySelector('[data-tools-row="demo-b"] [data-tools-unused]') && !document.querySelector('[data-tools-row="demo-a"] [data-tools-unused]')`))
    await shot('v89-01-sheet')

    const tFlip = Date.now()
    await evaluate(`${sw('demo-a')}.click()`)
    await waitFor(`${checked('demo-a')} === 'false'`, 5000, 'mcp off in draft')
    const flipMs = Date.now() - tFlip
    await sleep(400)
    check('toggle only changes the draft (no write)', JSON.stringify(readJson(claudeJson)) === JSON.stringify(before), `${flipMs} ms`)
    check('pending count 1, apply enabled', await evaluate(`!!document.querySelector('[data-tools-pending]') && !document.querySelector('[data-tools-apply]').disabled && /1/.test(document.querySelector('[data-tools-apply]').textContent) && !!document.querySelector('[data-tools-row="demo-a"][data-tools-dirty]')`))
    await evaluate(`document.querySelector('[data-tools-reset]').click()`)
    await waitFor(`${checked('demo-a')} === 'true'`, 3000, 'reset')
    check('reset clears the draft', await evaluate(`document.querySelector('[data-tools-apply]').disabled && !document.querySelector('[data-tools-dirty]')`))

    await evaluate(`document.querySelector('[data-tools-off-unused]').click()`)
    await sleep(200)
    const offDraft = await evaluate(`[...document.querySelectorAll('[data-tools-dirty]')].map(e => e.dataset.toolsRow).sort().join(',')`)
    check('"turn off unused" drafts only unused rows', offDraft === 'demo-c,demo-d,demo-e,demo-f,demo-g,demo-h', offDraft)
    await evaluate(`document.querySelector('[data-tools-reset]').click()`)
    await sleep(200)

    await evaluate(`${sw('demo-a')}.click(); ${sw('demo-plugin@mk')}.click()`)
    await waitFor(`${checked('demo-plugin@mk')} === 'false'`, 3000, 'plugin draft')
    await shot('v89-02-draft')
    const tApply = Date.now()
    await evaluate(`document.querySelector('[data-tools-apply]').click()`)
    await waitFor(`!!document.querySelector('[data-tools-banner]')`, 10000, 'applied')
    const applyMs = Date.now() - tApply
    let j = readJson(claudeJson)
    check('batch: mcp off written to projects[dir].disabledMcpServers', JSON.stringify(j?.projects?.[key]?.disabledMcpServers?.slice().sort()) === JSON.stringify(['demo-a', 'demo-b']), `${applyMs} ms ${JSON.stringify(j?.projects?.[key])}`)
    check('other keys kept', JSON.stringify(j?.zzz) === JSON.stringify(before.zzz) && j?.numStartups === 3 && JSON.stringify(j?.mcpServers) === JSON.stringify(before.mcpServers) && Array.isArray(j?.projects?.[key]?.allowedTools))
    check('backup written', fs.existsSync(claudeJson + '.ccm-backup'))
    const local = path.join(proj, '.claude', 'settings.local.json')
    check('batch: plugin off → settings.local.json false', readJson(local)?.enabledPlugins?.['demo-plugin@mk'] === false, JSON.stringify(readJson(local)))
    check('user settings untouched', readJson(path.join(cfg, 'settings.json'))?.enabledPlugins?.['demo-plugin@mk'] === true)
    check('banner shown, no restart without pane', await evaluate(`!!document.querySelector('[data-tools-banner]') && !document.querySelector('[data-tools-restart]') && document.querySelector('[data-tools-apply]').disabled`))
    await shot('v89-03-applied')

    await evaluate(`document.querySelector('[data-tools-all="mcp"]').click(); ${sw('demo-plugin@mk')}.click()`)
    await waitFor(`${checked('demo-a')} === 'true' && ${checked('demo-b')} === 'true'`, 3000, 'all on')
    await evaluate(`document.querySelector('[data-tools-apply]').click()`)
    await waitFor(`!document.querySelector('[data-tools-pending]') && document.querySelector('[data-tools-apply]').disabled && !!document.querySelector('[data-tools-banner]')`, 10000, 'applied 2')
    j = readJson(claudeJson)
    check('all on → disabledMcpServers key removed', j?.projects?.[key] && !('disabledMcpServers' in j.projects[key]), JSON.stringify(j?.projects?.[key]))
    check('plugin on → key removed', readJson(local)?.enabledPlugins?.['demo-plugin@mk'] === undefined, JSON.stringify(readJson(local)))

    await evaluate(`${sw('demo-h')}.click()`)
    await waitFor(`${checked('demo-h')} === 'false'`, 3000, 'draft before close')
    await evaluate(`document.querySelector('[data-session-tools]').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))`)
    await sleep(400)
    check('Esc closes sheet', await evaluate(`!document.querySelector('[data-session-tools]')`))
    check('discard toast with undo', await evaluate(`!!document.querySelector('[data-tools-undo]')`))
    await shot('v89-04-discarded')
    await evaluate(`document.querySelector('[data-tools-undo]').click()`)
    await waitFor(`!!document.querySelector('[data-session-tools] [data-tools-row="demo-h"]')`, 5000, 'reopened')
    check('undo reopens with the draft', await evaluate(`${checked('demo-h')} === 'false' && !!document.querySelector('[data-tools-row="demo-h"][data-tools-dirty]')`))
    const tReopen = Date.now()
    await evaluate(`${S}.get('ui').sessionTools = null`)
    await sleep(300)
    await evaluate(`${S}.get('ui').sessionTools = { dir: ${JSON.stringify(proj)} }`)
    await waitFor(`!!document.querySelector('[data-session-tools] [data-tools-row="demo-a"]')`, 5000, 'reopen')
    const reopenMs = Date.now() - tReopen - 300
    check('closing without undo leaves files alone', !(readJson(claudeJson)?.projects?.[key]?.disabledMcpServers ?? []).includes('demo-h'))
    console.log(`timing: first open ${firstOpen} ms, reopen from cache ${reopenMs} ms, flip ${flipMs} ms, apply 2 ${applyMs} ms`)
    await evaluate(`${S}.get('ui').sessionTools = null`)

    const d = JSON.stringify(proj)
    const bad = await evaluate(`window.ccmDesktop.extProjectToggle({ dir: ${d}, kind: 'mcp', name: 'orrery', enabled: false })`)
    check('orrery toggle refused', bad?.ok === false && bad?.code === 'locked', JSON.stringify(bad))
    const bad2 = await evaluate(`window.ccmDesktop.extProjectToggle({ dir: ${d}, kind: 'mcp', name: '-x', enabled: false })`)
    check('dash name refused', bad2?.ok === false && bad2?.code === 'badArg', JSON.stringify(bad2))
    const bad3 = await evaluate(`window.ccmDesktop.extProjectToggle({ dir: ${d}, changes: [{ kind: 'mcp', name: 'demo-a', enabled: false }, { kind: 'mcp', name: 'orrery', enabled: false }] })`)
    check('batch with orrery refused, nothing written', bad3?.ok === false && bad3?.code === 'locked' && !(readJson(claudeJson)?.projects?.[key]?.disabledMcpServers ?? []).includes('demo-a'), JSON.stringify(bad3))
    const bad4 = await evaluate(`window.ccmDesktop.extProjectToggle({ dir: ${d}, changes: [] })`)
    check('empty batch refused', bad4?.ok === false && bad4?.code === 'badArg', JSON.stringify(bad4))
    const u = await evaluate(`window.ccmDesktop.extSessionUsage('${SID}')`)
    check('usage has names and counts only', u?.ok === true && u.total === 5 && u.servers?.['demo-a'] === 3 && u.namespaces?.['demo-plugin'] === 1 && !JSON.stringify(u).includes('secret'), JSON.stringify(u))
    const u2 = await evaluate(`window.ccmDesktop.extSessionUsage('../x')`)
    check('usage refuses a bad id', u2?.ok === false && u2?.code === 'badArg', JSON.stringify(u2))

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
