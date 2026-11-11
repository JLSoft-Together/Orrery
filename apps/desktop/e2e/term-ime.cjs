const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' — ' + extra : ''}`); console.log(results.at(-1)) }
const WIN = process.platform === 'win32'

const BASE = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-ime-')))
const BIN = path.join(BASE, 'bin')
const PROJ = path.join(BASE, 'proj')
const CFG = path.join(BASE, 'claude')
const LINES = path.join(BASE, 'lines.txt')
const RAW = path.join(BASE, 'raw.txt')
const REC = path.join(BASE, 'rec.cjs')
for (const d of [BIN, PROJ, CFG]) fs.mkdirSync(d, { recursive: true })
fs.writeFileSync(REC, `const fs = require('fs')
let line = []
process.stdin.setRawMode(true)
process.stdin.setEncoding('utf8')
process.stdout.write('> ')
process.stdin.on('data', (d) => {
  fs.appendFileSync(${JSON.stringify(RAW)}, JSON.stringify(d) + '\\n')
  for (const ch of Array.from(d)) {
    if (ch === '\\x03') process.exit(0)
    if (ch === '\\x7f' || ch === '\\b') { if (line.length) { line.pop(); process.stdout.write('\\b \\b') } continue }
    if (ch === '\\r' || ch === '\\n') { fs.appendFileSync(${JSON.stringify(LINES)}, line.join('') + '\\n'); line = []; process.stdout.write('\\r\\n> '); continue }
    if (ch < ' ') continue
    line.push(ch)
    process.stdout.write(ch)
  }
})
`)
if (WIN) fs.writeFileSync(path.join(BIN, 'claude.cmd'), `@echo off\r\nnode "${REC}"\r\n`)
else fs.writeFileSync(path.join(BIN, 'claude'), `#!/bin/sh\nexec node "${REC}"\n`, { mode: 0o755 })
fs.writeFileSync(path.join(CFG, '.claude.json'), JSON.stringify({ projects: {} }, null, 2))

const readLines = () => { try { return fs.readFileSync(LINES, 'utf8').split('\n').slice(0, -1) } catch { return [] } }

async function main() {
  const app = await launch({ env: { CLAUDE_CONFIG_DIR: CFG, CCM_VERIFY_PROCESSES: '0', PATH: `${BIN}${path.delimiter}${process.env.PATH}` } })
  const { evaluate, waitFor, shot, sleep, cmd } = app
  const press = async (k, text) => {
    const vk = k === 'Backspace' ? 8 : k === 'Enter' ? 13 : k === ' ' ? 32 : /^[a-z]$/.test(k) ? k.toUpperCase().charCodeAt(0) : 231
    const code = k === 'Backspace' || k === 'Enter' ? k : k === ' ' ? 'Space' : /^[a-z]$/.test(k) ? 'Key' + k.toUpperCase() : ''
    const t = text ?? (k === 'Enter' ? '\r' : k === 'Backspace' ? undefined : k)
    await cmd('Input.dispatchKeyEvent', { type: t ? 'keyDown' : 'rawKeyDown', key: k, code, windowsVirtualKeyCode: vk, text: t, unmodifiedText: t })
    await cmd('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code, windowsVirtualKeyCode: vk })
    await sleep(30)
  }
  const keys = async (s) => { for (const ch of Array.from(s)) await press(ch) }
  const compose = async (steps) => {
    for (const s of steps) {
      await cmd('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Process', code: '', windowsVirtualKeyCode: 229 })
      await cmd('Input.imeSetComposition', { text: s, selectionStart: s.length, selectionEnd: s.length })
      await cmd('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Process', code: '', windowsVirtualKeyCode: 229 })
      await sleep(30)
    }
    await cmd('Input.insertText', { text: steps.at(-1) })
    await sleep(60)
  }
  const unikey = async (seq) => {
    for (const s of seq) {
      if (s === '\b') await press('Backspace')
      else await press(s, s)
    }
  }
  const submit = async (label) => {
    const before = readLines().length
    await press('Enter')
    for (let i = 0; i < 40 && readLines().length <= before; i++) await sleep(100)
    return readLines()[before] ?? `(${label}: no line)`
  }
  try {
    await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
    await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.tips', JSON.stringify({ off: true, learned: [], shown: {} }))`)
    await cmd('Page.reload')
    await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
    await evaluate(`${S}.get('ui').view = 'workspace'`)
    const pane = await evaluate(`${W}.open('claude', ${JSON.stringify(PROJ)}, undefined, { tab: true }).key`)
    await waitFor(`${W}.panes.find((p) => p.key === '${pane}')?.state === 'running'`, 20000, 'claude pane running')
    await waitFor(`!!document.querySelector(':is([data-pane="${pane}"], [data-pane-of="${pane}"]) .xterm-helper-textarea')`, 15000, 'xterm')
    await sleep(2500)
    await evaluate(`document.querySelector(':is([data-pane="${pane}"], [data-pane-of="${pane}"]) .xterm-helper-textarea').focus()`)
    await sleep(200)

    await keys('hello world')
    let got = await submit('ascii')
    check('ASCII keys arrive once', got === 'hello world', JSON.stringify(got))

    await compose(['t', 'ti', 'tie', 'tiê', 'tiến', 'tiếng'])
    await press(' ')
    await compose(['v', 'vi', 'vie', 'viê', 'việ', 'việt'])
    got = await submit('compose')
    check('IME composition (Microsoft Telex) commits each word once', got === 'tiếng việt', JSON.stringify(got))

    await unikey(['v', 'i', 'e', '\b', 'ê', 't', '\b', '\b', 'ệ', 't'])
    await press(' ')
    await unikey(['n', 'y', '\b', 'ý'])
    got = await submit('unikey')
    check('Unikey-style Backspace + Unicode key', got === 'việt ný', JSON.stringify(got))

    await compose(['o', 'ok', 'oke', 'oké'])
    await press('Backspace')
    await press('Backspace')
    await keys('i')
    await press(' ')
    await compose(['đ', 'đâ', 'đây'])
    got = await submit('delete')
    check('Backspace after composed word, then type on', got === 'oi đây', JSON.stringify(got))

    await compose(['u', 'uu'])
    await press('Backspace')
    await press('Backspace')
    await press('Backspace')
    await compose(['ư', 'ưu'])
    got = await submit('uu')
    check('no stray u after deleting composed text', got === 'ưu', JSON.stringify(got))

    await keys('abc')
    await press('Backspace')
    await compose(['ê'])
    got = await submit('mixed')
    check('ASCII, Backspace, then composition', got === 'abê', JSON.stringify(got))

    const ta = await evaluate(`document.querySelector(':is([data-pane="${pane}"], [data-pane-of="${pane}"]) .xterm-helper-textarea').value`)
    check('textarea cleared after Enter', ta === '', JSON.stringify(ta))
    await shot('term-ime-01')
    const raw = (() => { try { return fs.readFileSync(RAW, 'utf8').trim().split('\n').length } catch { return 0 } })()
    console.log(`raw chunks: ${raw}`)
  } catch (e) {
    check('run', false, String(e.stack || e).slice(0, 400))
    await shot('term-ime-fail').catch(() => {})
  } finally {
    app.stop()
    const fail = results.filter((r) => r.startsWith('FAIL')).length
    console.log(`\n${results.length - fail}/${results.length} pass`)
    try { console.log('raw:\n' + fs.readFileSync(RAW, 'utf8')) } catch {}
    process.exit(fail ? 1 : 0)
  }
}
main()
