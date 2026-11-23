const fs = require('node:fs')
const path = require('node:path')
const { launch, OUT, REPO } = require('./cdp.cjs')
const CTRL = 2
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const SET = `${S}.get('settings')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }
const SHOTS = path.join(REPO, 'docs', 'qa', 'v111', 'WP-111c')

const banner = (port) => `node -e "console.log('  \\u279c  Local:   http://localhost:'+(${port - 3}+3)+'/');setTimeout(()=>{},120000)"`

;(async () => {
  const app = await launch(process.argv[2] ? { exe: process.argv[2] } : {})
  const { evaluate, waitFor, shot, sleep, cmd, key, type } = app
  const browsers = () => evaluate(`${W}.panes.filter((p) => p.kind === 'browser').map((p) => p.url)`)
  const chip = () => evaluate(`document.querySelector('[data-dev-chip]')?.textContent.trim() ?? null`)
  const run = async (line) => {
    await evaluate(`document.querySelector('[data-pane="' + ${W}.activeKey + '"] .xterm-helper-textarea')?.focus()`)
    await type(line)
    await key('Enter', 'Enter', 13)
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell')`)
    await cmd('Page.reload')
    await sleep(1500)
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
    await evaluate(`[...document.querySelectorAll('[data-launch-row]')].at(-1).click()`)
    await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
    await sleep(2500)
    const shellKey = await evaluate(`${W}.panes[0].key`)

    check('switch is on by default', (await evaluate(`${SET}.devPreviewAutoOpen`)) === true)
    check('no chip before any dev server', (await chip()) === null)

    await run(banner(5173))
    await waitFor(`${W}.panes.some((p) => p.kind === 'browser')`, 15000, 'browser pane')
    check('first banner opens a Browser pane on localhost:5173', (await browsers()).some((u) => u.includes('localhost:5173')), JSON.stringify(await browsers()))
    check('browser pane sits in the same tab', await evaluate(`(() => { const t = ${W}.tabs.find((t) => JSON.stringify(t.root).includes(${JSON.stringify(shellKey)})); return !!t && JSON.stringify(t.root).includes(${W}.panes.find((p) => p.kind === 'browser').key) })()`))
    check('terminal keeps the focus', (await evaluate(`${W}.activeKey`)) === shellKey)
    await waitFor(`!!document.querySelector('[data-dev-chip]')`, 5000, 'chip')
    check('chip shows localhost:5173', (await chip()) === 'localhost:5173', await chip())

    await sleep(800)
    await shot('v111-preview-chip')
    fs.mkdirSync(SHOTS, { recursive: true })
    const rect = await evaluate(`(() => { const p = document.querySelector('[data-pane="${shellKey}"]'); const r = p.getBoundingClientRect(); return { x: r.left, y: r.top, width: r.width, height: 56 } })()`)
    const clip = await cmd('Page.captureScreenshot', { format: 'png', clip: { ...rect, scale: 2 } })
    if (clip.result?.data) fs.writeFileSync(path.join(SHOTS, 'header-chip.png'), Buffer.from(clip.result.data, 'base64'))
    fs.copyFileSync(path.join(OUT, 'v111-preview-chip.png'), path.join(SHOTS, 'workspace.png'))

    await evaluate(`document.querySelector('[data-pane="' + ${W}.activeKey + '"] .xterm-helper-textarea')?.focus()`)
    await key('c', 'KeyC', 67, CTRL)
    await sleep(1500)
    await run(banner(5173))
    await sleep(1500)
    check('same URL again does not open another pane', (await browsers()).length === 1, JSON.stringify(await browsers()))

    const bk = await evaluate(`${W}.panes.find((p) => p.kind === 'browser').key`)
    await evaluate(`${W}.remove(${JSON.stringify(bk)})`)
    await sleep(500)
    await evaluate(`document.querySelector('[data-pane="${shellKey}"] .xterm-helper-textarea')?.focus()`)
    await key('c', 'KeyC', 67, CTRL)
    await sleep(1500)
    await run(banner(5173))
    await sleep(1500)
    check('after closing the preview the same URL stays a chip only', (await browsers()).length === 0 && (await chip()) === 'localhost:5173', JSON.stringify(await browsers()))

    await evaluate(`${SET}.devPreviewAutoOpen = false`)
    await evaluate(`document.querySelector('[data-pane="${shellKey}"] .xterm-helper-textarea')?.focus()`)
    await key('c', 'KeyC', 67, CTRL)
    await sleep(1500)
    await run(banner(5174))
    await waitFor(`document.querySelector('[data-dev-chip]')?.textContent.includes('5174')`, 8000, 'chip 5174')
    await sleep(1000)
    check('switch off: new URL gives the chip only', (await browsers()).length === 0, JSON.stringify(await browsers()))
    check('stored in ccm.devPreview.autoOpen', (await evaluate(`localStorage.getItem('ccm.devPreview.autoOpen')`)) === '0')

    await evaluate(`document.querySelector('[data-dev-chip]').click()`)
    await waitFor(`${W}.panes.some((p) => p.kind === 'browser')`, 8000, 'chip opens browser')
    check('clicking the chip opens the preview', (await browsers()).some((u) => u.includes('localhost:5174')), JSON.stringify(await browsers()))
    await evaluate(`${SET}.devPreviewAutoOpen = true`)

    await evaluate(`${W}.activate(${JSON.stringify(shellKey)})`)
    await evaluate(`document.querySelector('[data-pane="${shellKey}"] .xterm-helper-textarea')?.focus()`)
    await key('c', 'KeyC', 67, CTRL)
    await sleep(1500)
    await run('exit')
    await waitFor(`!document.querySelector('[data-dev-chip]')`, 10000, 'chip gone')
    check('chip disappears when the process exits', true)
  } catch (e) {
    check('run', false, e.message)
  }
  console.log(results.join('\n'))
  app.stop()
  process.exit(results.some((r) => r.startsWith('FAIL')) ? 1 : 0)
})()
