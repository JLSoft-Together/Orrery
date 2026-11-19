const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { launch, OUT, REPO } = require('./cdp.cjs')
const S = `document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s`
const W = `${S}.get('workspace')`
const results = []
const check = (name, ok, extra = '') => { results.push(`${ok ? 'PASS' : 'FAIL'} ${name}${extra ? ' - ' + extra : ''}`); console.log(results.at(-1)) }

const DIR = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-v103-log-')))
const git = (...args) => execFileSync('git', ['-c', 'user.name=e2e', '-c', 'user.email=e2e@local', '-c', 'commit.gpgsign=false', ...args], { cwd: DIR, encoding: 'utf8' }).trim()
const zlib = require('node:zlib')
const chunk = (type, data) => {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(zlib.crc32(body) >>> 0)
  return Buffer.concat([len, body, crc])
}
const png = (w, h, [r, g, b]) => {
  const head = Buffer.alloc(13)
  head.writeUInt32BE(w, 0)
  head.writeUInt32BE(h, 4)
  head.set([8, 2, 0, 0, 0], 8)
  const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(w * 3).map((_, i) => [r, g, b][i % 3])])
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', head), chunk('IDAT', zlib.deflateSync(Buffer.concat(Array(h).fill(row)))), chunk('IEND', Buffer.alloc(0))])
}
const RED = png(96, 64, [230, 70, 60])
const BLUE = png(64, 96, [50, 110, 230])
const svg = (fill, extra = '') => `<svg xmlns="http://www.w3.org/2000/svg" width="160" height="120" viewBox="0 0 160 120"><rect width="160" height="120" fill="#fff"/><circle cx="80" cy="60" r="44" fill="${fill}"/>${extra}</svg>`
const write = (f, data) => {
  fs.mkdirSync(path.dirname(path.join(DIR, f)), { recursive: true })
  fs.writeFileSync(path.join(DIR, f), data)
}

function fixture() {
  git('init', '-q', '-b', 'main')
  git('config', 'core.autocrlf', 'false')
  write('a.png', RED)
  write('logo.svg', svg('#f08a24'))
  write('notes.txt', 'one\n')
  git('add', '-A')
  git('commit', '-q', '-m', 'add images')
  write('a.png', BLUE)
  write('logo.svg', svg('#7a4cf0', '<rect x="20" y="20" width="40" height="40" fill="#18a999"/>'))
  write('notes.txt', 'one\ntwo\n')
  git('add', '-A')
  git('commit', '-q', '-m', 'edit images')
  fs.mkdirSync(path.join(DIR, 'img'), { recursive: true })
  git('mv', 'a.png', 'img/b.png')
  git('rm', '-q', 'logo.svg')
  write('c.txt', 'c\n')
  git('add', '-A')
  git('commit', '-q', '-m', 'move and drop')
}

async function run(app) {
  const { evaluate, waitFor, shot, sleep, cmd, key } = app
  const hashes = git('log', '--format=%H').split('\n')
  const [moved, edited, added] = hashes
  await waitFor(`!!window.ccmDesktop && !!${S}.get('ui')`, 30000, 'app ready')
  await evaluate(`localStorage.setItem('ccm.tourDone','1'); localStorage.setItem('ccm.locale','en'); localStorage.setItem('ccm.whatsNew.seen','${require('../package.json').version}'); localStorage.setItem('ccm.setupDone','1'); localStorage.setItem('ccm.tourWorkspaceDone','1'); localStorage.setItem('ccm.workspace.preset','shell'); localStorage.removeItem('ccm.workspace.logCols')`)
  await cmd('Page.reload')
  await sleep(1500)
  await waitFor(`!!${S}.get('ui')`, 30000, 'reloaded')
  await evaluate(`${S}.get('settings').themeFollow = false; ${S}.get('settings').setThemeMode('light')`)
  await evaluate(`${S}.get('ui').view = 'workspace'`)
  await waitFor(`!!document.querySelector('[data-launch-row]')`, 15000, 'picker')
  await evaluate(`${W}.open('shell', ${JSON.stringify(DIR)})`)
  await waitFor(`${W}.panes[0]?.state === 'running'`, 20000, 'shell running')
  await evaluate(`${W}.openLog(${JSON.stringify(DIR)})`)
  await waitFor(`document.querySelectorAll('[data-log-table] [data-commit]').length === 3`, 15000, 'log rows')

  const row = (hash) => `document.querySelector('[data-log-table] [data-commit="${hash}"]')`
  const title = () => evaluate(`document.querySelector('[data-log-diff-title]')?.textContent ?? ''`)
  const imgs = () => evaluate(`JSON.stringify([...document.querySelectorAll('[data-log-diff] [data-image-side]')].map((f) => ({ side: f.dataset.imageSide, w: f.querySelector('img')?.naturalWidth ?? 0, src: (f.querySelector('img')?.src ?? '').slice(0, 22) })))`).then(JSON.parse)
  const waitImgs = (n, mime = 'image/') => waitFor(`(() => { const f = [...document.querySelectorAll('[data-log-diff] [data-image-side]')]; return f.length === ${n} && f.every((x) => x.querySelector('img')?.naturalWidth > 0 && x.querySelector('img').src.startsWith('data:${mime}')) })()`, 10000, `images ${n}`).catch(() => {})

  await evaluate(`${row(edited)}.click()`)
  await waitFor(`!!document.querySelector('[data-log-diff-mode=preview]')`, 10000, 'preview').catch(() => {})
  check('selecting a commit shows the first file diff under the log', await evaluate(`!!document.querySelector('[data-log-diff-mode=preview]') && document.querySelector('[data-log-table]').offsetParent !== null && !!document.querySelector('[data-log-files]')`))

  await evaluate(`document.querySelector('[data-log-file="logo.svg"]').click()`)
  await waitImgs(2)
  const svgSides = await imgs()
  check('svg in a commit shows before and after as images', svgSides.length === 2 && svgSides.every((s) => s.w > 0 && s.src.startsWith('data:image/svg+xml')), JSON.stringify(svgSides))
  check('no binary file message for images', !(await evaluate(`document.querySelector('[data-log-diff]').textContent.includes('Binary')`)))
  await shot('v103-wp72-01-svg-preview-light')

  await evaluate(`document.querySelector('[data-log-file="a.png"]').click()`)
  await waitImgs(2, 'image/png')
  const pngSides = await imgs()
  check('png before and after both load from git', pngSides.length === 2 && pngSides.every((s) => s.w > 0 && s.src.startsWith('data:image/png')), JSON.stringify(pngSides))
  await evaluate(`document.querySelector('[data-image-mode-btn=slider]')?.click()`)
  await sleep(300)
  check('slider mode works on commit images', await evaluate(`document.querySelector('[data-image-compare]')?.dataset.imageMode === 'slider'`))
  await shot('v103-wp72-02-png-slider')
  await evaluate(`document.querySelector('[data-image-mode-btn=side]')?.click()`)

  await evaluate(`${row(edited)}.click(); document.querySelector('[data-log-table]').focus()`)
  await key('ArrowDown', 'ArrowDown', 40)
  await waitFor(`${row(added)}?.getAttribute('aria-selected') === 'true'`, 5000, 'older').catch(() => {})
  await waitImgs(1)
  const addedSides = await imgs()
  check('↓ moves to the older commit and keeps the same file', (await title()).includes('a.png') && (await title()).includes(added.slice(0, 8)), await title())
  check('an added image shows one side only', addedSides.length === 1 && addedSides[0].side === 'new' && addedSides[0].w > 0, JSON.stringify(addedSides))
  await key('k', 'KeyK', 75)
  await waitFor(`${row(edited)}?.getAttribute('aria-selected') === 'true'`, 5000, 'k').catch(() => {})
  check('k in the log moves to the newer commit', await evaluate(`${row(edited)}?.getAttribute('aria-selected') === 'true'`))

  await key('Enter', 'Enter', 13)
  await waitFor(`!!document.querySelector('[data-log-diff-mode=full]')`, 5000, 'full').catch(() => {})
  check('Enter opens the diff full size and keeps the file list', await evaluate(`!!document.querySelector('[data-log-diff-mode=full]') && document.querySelector('[data-log-table]').offsetParent === null && document.querySelector('[data-log-files]').offsetParent !== null`))
  await evaluate(`document.activeElement?.blur?.()`)
  await key('ArrowUp', 'ArrowUp', 38, 1)
  await waitFor(`document.querySelector('[data-log-diff-title]')?.textContent.includes(${JSON.stringify(moved.slice(0, 8))})`, 5000, 'alt up').catch(() => {})
  check('Alt+↑ steps to the newer commit inside the full diff', (await title()).includes(moved.slice(0, 8)) && (await title()).includes('img/b.png'), await title())
  await waitImgs(2)
  const renamed = await imgs()
  check('renamed image reads the old name from the parent', renamed.length === 2 && renamed.every((s) => s.w > 0), JSON.stringify(renamed))
  await shot('v103-wp72-03-full-renamed')

  await evaluate(`document.querySelector('[data-log-file="logo.svg"]').click()`)
  await waitImgs(1)
  const deleted = await imgs()
  check('a deleted image shows the old side only', deleted.length === 1 && deleted[0].side === 'old' && deleted[0].w > 0, JSON.stringify(deleted))
  const before = await title()
  await key('ArrowLeft', 'ArrowLeft', 37)
  await sleep(400)
  check('← steps to the previous file', (await title()) !== before, `${before} -> ${await title()}`)
  await evaluate(`document.activeElement?.blur?.()`)
  await key('Escape', 'Escape', 27)
  await waitFor(`!document.querySelector('[data-log-diff-mode=full]')`, 5000, 'esc').catch(() => {})
  check('Esc goes back to the log with the preview still open', await evaluate(`!document.querySelector('[data-log-diff-mode=full]') && !!document.querySelector('[data-log-diff-mode=preview]') && document.querySelector('[data-log-table]').offsetParent !== null`))

  await evaluate(`document.querySelector('[data-log-diff-hide]').click()`)
  await sleep(300)
  check('the preview can be hidden', await evaluate(`!document.querySelector('[data-log-diff]') && JSON.parse(localStorage.getItem('ccm.workspace.logCols')).preview === false`))
  await evaluate(`document.querySelector('[data-log-preview-toggle]').click()`)
  await waitFor(`!!document.querySelector('[data-log-diff-mode=preview]')`, 5000, 'preview back').catch(() => {})
  check('the header button brings the preview back', await evaluate(`!!document.querySelector('[data-log-diff-mode=preview]')`))
  check('no horizontal page scroll', await evaluate(`document.documentElement.scrollWidth <= document.documentElement.clientWidth`))

  await evaluate(`${S}.get('settings').setThemeMode('dark')`)
  await sleep(500)
  await evaluate(`document.querySelector('[data-log-file="img/b.png"]')?.click()`)
  await waitImgs(2)
  await shot('v103-wp72-04-preview-dark')
  await evaluate(`${row(edited)}.click()`)
  await sleep(300)
  await evaluate(`document.querySelector('[data-log-file="notes.txt"]').click()`)
  await waitFor(`!!document.querySelector('[data-log-diff] [data-diff-row], [data-log-diff] .cm-line, [data-log-diff] [data-line]') || document.querySelector('[data-log-diff]').textContent.includes('two')`, 8000, 'text diff').catch(() => {})
  check('text files still show a text diff', await evaluate(`document.querySelector('[data-log-diff]').textContent.includes('two')`))
  await shot('v103-wp72-05-text-dark')
}

;(async () => {
  fixture()
  const bin = ['debug', 'release'].map((p) => path.join(REPO, 'apps', 'core', 'target', p, process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core')).filter((f) => fs.existsSync(f)).sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)[0]
  const app = await launch({ ui: 'next', ...(process.argv[2] ? { exe: process.argv[2] } : {}), env: bin && !process.env.CCM_CORE_BIN ? { CCM_CORE_BIN: bin } : {} })
  try {
    await run(app)
  } catch (e) {
    check('run', false, e.message)
    await app.shot('v103-wp72-99-error').catch(() => {})
  } finally {
    app.stop()
    fs.writeFileSync(path.join(OUT, 'results-v103-log-diff.txt'), results.join('\n'))
    try { fs.rmSync(DIR, { recursive: true, force: true }) } catch {}
    process.exit(0)
  }
})()
