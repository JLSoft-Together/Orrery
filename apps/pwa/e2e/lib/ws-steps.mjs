import { go } from './ui-steps.mjs'
import { wait } from './ctx.mjs'

export function bigPayload() {
  const out = []
  let n = 0
  while (out.length < 65536) {
    const line = `line ${String(n).padStart(5, '0')} ${'abcdefghij'.repeat(6)}\r\n`
    for (const b of Buffer.from(line)) if (out.length < 65536) out.push(b)
    n++
  }
  return Buffer.from(out)
}

export function checksum(buffers) {
  let sum = 0
  for (const buf of buffers) for (const b of buf) sum = (Math.imul(sum, 31) + b) >>> 0
  return sum
}

const hexOf = (text) => Buffer.from(text, 'utf8').toString('hex')

export async function workspaceSteps(c, page, peer) {
  const { step, byId, expectText } = c
  const id = peer.info.machine
  const banner = Buffer.from('$ orrery e2e ready\r\n')
  const rx = async () => ({ bytes: Number(await byId(page, 'terminal').getAttribute('data-rx-bytes')), sum: Number(await byId(page, 'terminal').getAttribute('data-rx-sum')) })
  const dump = async () => JSON.stringify({ state: await byId(page, 'terminal').getAttribute('data-state').catch(() => null), bytes: await byId(page, 'terminal').getAttribute('data-rx-bytes').catch(() => null), detail: await byId(page, 'terminal').getAttribute('data-detail').catch(() => null), plog: peer.log.slice(-12), overlay: await byId(page, 'term-overlay').textContent().catch(() => null), events: peer.events.slice(-4), errors: c.errors.slice(-4), log: peer.log.slice(-4) })
  const live = () => page.waitForFunction(() => document.querySelector('[data-testid="terminal"]')?.getAttribute('data-state') === 'live', null, { timeout: 12000 }).catch(async (e) => { throw new Error(`not live: ${await dump()}`) })

  await step('Workspace A: one pane + chips (agent colour, status icon + text), mode switch to B is remembered across reload', async () => {
    await go(page, `/app/m/${id}/ws`)
    await byId(page, 'ws-focus').waitFor({ timeout: 8000 })
    if ((await byId(page, 'workspace-view').getAttribute('data-mode')) !== 'focus') throw new Error('A is not the default')
    if ((await page.locator('[data-testid="ws-chips"] > button').count()) !== 3) throw new Error('chip count')
    const card = byId(page, 'pane-p1')
    if ((await card.locator('[data-testid="status-chip"] svg').count()) < 1) throw new Error('status icon missing')
    if (!/Đang chạy/.test(await c.text(card))) throw new Error(`status text ${await c.text(card)}`)
    await byId(page, 'chip-p2').click()
    await page.waitForFunction(() => document.querySelector('[data-testid="ws-focus"]')?.getAttribute('data-pane') === 'p2')
    if ((await byId(page, 'pane-p2').evaluate((el) => el.classList.contains('p-codex'))) !== true) throw new Error('codex colour class missing')
    const edge = await byId(page, 'pane-p2').locator('.bar').evaluate((el) => getComputedStyle(el).backgroundColor)
    const claude = await byId(page, 'pane-p2').evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--ccm-agent-claude'))
    if (!edge || edge === 'rgba(0, 0, 0, 0)' || !claude) throw new Error(`edge colour ${edge}`)
    await byId(page, 'ws-mode').click()
    await byId(page, 'ws-grid').waitFor()
    if ((await page.locator('[data-testid="ws-grid"] > button').count()) !== 3) throw new Error('grid cards')
    await c.reload(page)
    await byId(page, 'ws-grid').waitFor({ timeout: 8000 })
    await byId(page, 'ws-filter-running').click()
    if ((await page.locator('[data-testid="ws-grid"] > button').count()) !== 2) throw new Error('running filter')
    await byId(page, 'ws-filter-all').click()
    await byId(page, 'ws-mode').click()
    await byId(page, 'ws-focus').waitFor()
  })

  await step('Terminal: attach with the phone size, receive 64 KiB pty_out byte for byte, no CSP violation', async () => {
    await byId(page, 'ws-open').click()
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    const attach = await peer.waitEvent((e) => e.ev === 'attach' && e.sessionId === 'sess-p1')
    if (!(attach.cols >= 20 && attach.rows >= 5 && attach.stream === 1)) throw new Error(`attach ${JSON.stringify(attach)}`)
    await live()
    await page.waitForFunction((n) => Number(document.querySelector('[data-testid="terminal"]')?.getAttribute('data-rx-bytes')) >= n, banner.length, { timeout: 8000 })
    peer.send('big')
    const total = banner.length + 65536
    await page.waitForFunction((n) => Number(document.querySelector('[data-testid="terminal"]')?.getAttribute('data-rx-bytes')) >= n, total, { timeout: 15000 })
    const got = await rx()
    if (got.bytes !== total) throw new Error(`received ${got.bytes} of ${total}`)
    const want = checksum([banner, bigPayload()])
    if (got.sum !== want) throw new Error(`checksum ${got.sum} != ${want}`)
    await wait(400)
    const shown = await page.locator('.xterm-rows').innerText()
    if (!/line \d{5}/.test(shown)) throw new Error('xterm shows no output')
    const csp = await page.evaluate(() => window.__csp.filter((x) => !/stylesheet/.test(x)))
    if (csp.length) throw new Error(`CSP: ${csp.join(' | ')}`)
    const colour = await page.locator('.xterm-rows span').first().evaluate((el) => getComputedStyle(el).color)
    if (colour === 'rgb(0, 0, 0)') throw new Error('xterm text unstyled')
  })

  await step('Typing: "echo xin chào" reaches the desktop as UTF-8 with claim; Ctrl is sticky once / locked / off; direct typing claims too', async () => {
    await byId(page, 'line-input').fill('echo xin chào')
    await byId(page, 'line-send').click()
    const line = await peer.waitEvent((e) => e.ev === 'pty_in' && e.hex === `${hexOf('echo xin chào')}0d`)
    if (line.claim !== true || line.stream !== 1) throw new Error(`line ${JSON.stringify(line)}`)
    if ((await byId(page, 'terminal').getAttribute('data-claimed')) !== '1') throw new Error('not claimed')
    const ctrl = byId(page, 'key-ctrl')
    const mods = []
    for (let i = 0; i < 3; i++) {
      await ctrl.click()
      mods.push(await ctrl.getAttribute('data-mod'))
    }
    if (mods.join(',') !== 'once,locked,off') throw new Error(`ctrl cycle ${mods}`)
    await ctrl.click()
    await byId(page, 'key-up').click()
    await peer.waitEvent((e) => e.ev === 'pty_in' && e.hex === hexOf('\x1b[1;5A') && e.claim === true)
    if ((await ctrl.getAttribute('data-mod')) !== 'off') throw new Error('once did not clear')
    await byId(page, 'key-esc').click()
    await peer.waitEvent((e) => e.ev === 'pty_in' && e.hex === '1b')
    await byId(page, 'key-ctrlC').click()
    await peer.waitEvent((e) => e.ev === 'pty_in' && e.hex === '03')
    await byId(page, 'direct-toggle').click()
    await wait(200)
    await page.keyboard.type('x')
    const direct = await peer.waitEvent((e) => e.ev === 'pty_in' && e.hex === '78', 3000).catch(async (e) => { throw new Error(`direct: active=${await page.evaluate(() => document.activeElement?.className + '|' + document.activeElement?.tagName + '|' + document.activeElement?.getAttribute('inputmode'))} ${e.message.slice(-200)}`) })
    if (direct.claim !== true) throw new Error('direct typing without claim')
    await byId(page, 'direct-toggle').click()
    await peer.waitEvent((e) => e.ev === 'resize' && e.stream === 1)
  })

  await step('Pane picker switches pane: detach the old stream, attach the new session', async () => {
    await byId(page, 'pane-pick').click()
    await byId(page, 'pane-sheet').waitFor()
    await byId(page, 'pick-p3').click()
    await page.waitForFunction(() => document.querySelector('[data-testid="terminal-view"]')?.getAttribute('data-pane') === 'p3')
    await peer.waitEvent((e) => e.ev === 'detach' && e.stream === 1)
    await peer.waitEvent((e) => e.ev === 'attach' && e.sessionId === 'sess-p3')
    await live()
    await byId(page, 'pane-pick').click()
    await byId(page, 'pick-p1').click()
    await page.waitForFunction(() => document.querySelector('[data-testid="terminal-view"]')?.getAttribute('data-pane') === 'p1')
    await peer.waitEvent((e) => e.ev === 'attach' && e.sessionId === 'sess-p1' && e.stream >= 3)
    await live()
  })

  await step('Permission bar on an asked pane sends ws.keys with the status time; remote confirm sheet answers confirm.answer', async () => {
    peer.send('asked')
    await byId(page, 'permission-bar').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="permission-bar"]', /npm test/)
    await byId(page, 'answer-approve').click()
    const keys = await peer.waitEvent((e) => e.ev === 'keys' && e.keys === 'approve')
    if (keys.paneKey !== 'p1' || !(keys.since > 0)) throw new Error(`keys ${JSON.stringify(keys)}`)
    await byId(page, 'answer-deny').click()
    await peer.waitEvent((e) => e.ev === 'keys' && e.keys === 'deny')
    peer.send('confirm')
    await byId(page, 'remote-confirm').waitFor({ timeout: 8000 })
    await expectText(page, '[data-testid="remote-confirm-title"]', /Bỏ thay đổi trong src\/app\.ts/)
    const left = await c.text(byId(page, 'remote-confirm-timer'))
    if (!/\b(59|60)\b/.test(left)) throw new Error(`timer ${left}`)
    await byId(page, 'remote-confirm-ok').click()
    const answer = await peer.waitEvent((e) => e.ev === 'confirm')
    if (answer.cid !== 'cid1' || answer.ok !== true) throw new Error(`answer ${JSON.stringify(answer)}`)
    await page.waitForFunction(() => !document.querySelector('[data-testid="remote-confirm"]'))
  })

  await step('Viewer: file with line numbers (read only), diff with add / delete rows, git log, run state', async () => {
    await byId(page, 'open-viewer').click()
    await byId(page, 'viewer-files').waitFor({ timeout: 8000 })
    await byId(page, 'entry-key_bar.dart').click()
    await byId(page, 'code-view').waitFor({ timeout: 8000 })
    if (Number(await byId(page, 'code-view').getAttribute('data-lines')) < 5) throw new Error('file lines')
    await expectText(page, '[data-testid="file-foot"]', /chỉ đọc/)
    await byId(page, 'files-up').click()
    await byId(page, 'files-list').waitFor()
    await byId(page, 'viewer-seg-diff').click()
    await byId(page, 'change-src/app.ts').click()
    await byId(page, 'diff-view').waitFor({ timeout: 8000 })
    const kinds = await page.locator('[data-testid="diff-view"] > div').evaluateAll((els) => els.map((e) => e.getAttribute('data-kind')))
    if (kinds.filter((k) => k === 'add').length !== 2 || kinds.filter((k) => k === 'del').length !== 1) throw new Error(`diff kinds ${kinds}`)
    await expectText(page, '[data-testid="diff-added"]', /2 thêm/)
    const addBg = await page.locator('[data-testid="diff-view"] .add').first().evaluate((el) => getComputedStyle(el).backgroundColor)
    if (addBg === 'rgba(0, 0, 0, 0)') throw new Error('add row has no colour')
    await byId(page, 'diff-back').click()
    await byId(page, 'viewer-seg-git').click()
    await byId(page, 'commit-a1b2c3d').waitFor({ timeout: 8000 })
    await byId(page, 'viewer-seg-run').click()
    await expectText(page, '[data-testid="run-state"]', /running/)
    await byId(page, 'viewer-seg-files').click()
    await byId(page, 'viewer-close').click()
    await page.waitForFunction(() => !document.querySelector('[data-testid="viewer"]'))
  })

  await step('Deep link /m/<machine>/p/<pane> opens the pane; a closed pane and an unpaired machine show their notices', async () => {
    await c.open(page, c.env.base, `/app/m/${id}/p/p2`)
    await byId(page, 'terminal-view').waitFor({ timeout: 10000 })
    await peer.waitEvent((e) => e.ev === 'attach' && e.sessionId === 'sess-p2')
    if ((await byId(page, 'terminal-view').getAttribute('data-pane')) !== 'p2') throw new Error('wrong pane')
    await live()
    await c.open(page, c.env.base, `/app/m/${id}/p/p99`)
    await byId(page, 'deeplink-gone').waitFor({ timeout: 25000 })
    await c.open(page, c.env.base, '/app/m/not-a-machine/p/p1')
    await byId(page, 'deeplink-unpaired').waitFor({ timeout: 8000 })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })

}

export async function viewOnlyStep(c, page, peer) {
  const { step, byId } = c
  const id = peer.info.machine
  await step('Send sheet types a prompt (ws.send); with remote control off the desktop answers denied and the UI says view only', async () => {
    await go(page, `/app/m/${id}/ws`)
    await byId(page, 'ws-send').click()
    await byId(page, 'send-text').fill('chạy npm test giúp mình')
    await byId(page, 'send-go').click()
    const sent = await peer.waitEvent((e) => e.ev === 'send')
    if (sent.text !== 'chạy npm test giúp mình') throw new Error(`send ${JSON.stringify(sent)}`)
    await page.waitForFunction(() => !document.querySelector('[data-testid="send-sheet"]'))
    peer.send('viewonly')
    await wait(200)
    await byId(page, 'ws-send').click()
    await byId(page, 'send-text').fill('x')
    await byId(page, 'send-go').click()
    await byId(page, 'send-error').waitFor({ timeout: 8000 })
    await byId(page, 'view-only').waitFor({ timeout: 5000 })
    await byId(page, 'send-sheet').click({ position: { x: 4, y: 4 } })
    await byId(page, 'ws-open').click()
    await byId(page, 'terminal-view').waitFor()
    await byId(page, 'view-only').waitFor()
    if (!(await byId(page, 'line-input').isDisabled())) throw new Error('input enabled while view only')
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
