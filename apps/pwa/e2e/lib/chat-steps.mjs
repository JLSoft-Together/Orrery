import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { IPAD, PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

export const CHAT_STEPS = 9

async function after(peer, from, match, ms = 8000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    const hit = peer.events.slice(from).find(match)
    if (hit) return hit
    await wait(40)
  }
  throw new Error(`peer event not seen: ${JSON.stringify(peer.events.slice(-6))}`)
}

export async function chatSteps(c, page, peer) {
  const { step, byId, env, browserName, shoot, noOverflow, text, reload } = c
  const id = peer.info.machine
  const dir = join(env.qaDirH, '..', '..', 'v107', 'C4')
  mkdirSync(dir, { recursive: true })
  const msgs = () => page.locator('[data-testid="chat-msgs"]')

  await step('Chat home: inbox rows from the workspace, last agent sentence from chat.last, waiting first, no tab bar; light and dark', async () => {
    await page.setViewportSize(PHONE)
    await page.emulateMedia({ colorScheme: 'light' })
    peer.send('chat')
    const from = peer.events.length
    await go(page, `/app/m/${id}`)
    await reload(page)
    await after(peer, from, (e) => e.ev === 'hello' && Array.isArray(e.caps) && e.caps.includes('chat'))
    await page.waitForSelector('[data-testid="chat-home"][data-chat="1"]', { timeout: 10000 })
    await page.waitForFunction(() => /redirect_uri/.test(document.querySelector('[data-testid="chat-row-p1"]')?.textContent ?? ''), null, { timeout: 8000 })
    if (await byId(page, 'tab-bar').count()) throw new Error('tab bar on the chat home')
    for (const key of ['p1', 'p2', 'p3']) await byId(page, `chat-row-${key}`).waitFor()
    await noOverflow(page, 'chat home')
    await wait(300)
    await shoot(page, join(dir, `chat-home-phone-light-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'dark' })
    await wait(600)
    await shoot(page, join(dir, `chat-home-phone-dark-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'light' })
  })

  await step('Chat: a Claude row opens the chat, watch reply renders text, markdown, code, tool line, files card; raw HTML stays text', async () => {
    const from = peer.events.length
    const t0 = Date.now()
    await byId(page, 'chat-row-p1').click()
    await page.waitForSelector('[data-testid="chat-view"][data-status="ready"]', { timeout: 8000 })
    await byId(page, 'chat-ai').first().waitFor()
    const ms = Date.now() - t0
    console.log(`[${browserName}] tap to chat visible ${ms} ms`)
    await after(peer, from, (e) => e.ev === 'chat.watch' && e.paneKey === 'p1')
    if ((await byId(page, 'chat-me').count()) < 1) throw new Error('my message missing')
    await byId(page, 'chat-code').waitFor()
    const ai = await text(byId(page, 'chat-ai').last())
    if (!/<b>raw<\/b>/.test(ai)) throw new Error(`raw html not shown as text: ${ai}`)
    if (await page.locator('[data-testid="chat-ai"] b').count()) throw new Error('html injected')
    if ((await page.locator('[data-testid="chat-ai"] strong').count()) !== 1) throw new Error('bold missing')
    const tools = await text(byId(page, 'chat-tools'))
    if (!/đọc 4 file/.test(tools) || !/chạy 1 lệnh/.test(tools)) throw new Error(`tool sentence ${tools}`)
    await byId(page, 'chat-tools-toggle').click()
    await byId(page, 'chat-tools-list').waitFor()
    if (!/rg redirect_uri/.test(await text(byId(page, 'chat-tools-list')))) throw new Error('tool list')
    const files = await text(byId(page, 'chat-files'))
    if (!/Đã sửa 3 file/.test(files) || !/\+16/.test(files)) throw new Error(`files card ${files}`)
    const order = await page.locator('[data-testid="chat-msgs"] > [data-testid]').evaluateAll((els) => els.map((e) => e.getAttribute('data-testid')))
    if (order.indexOf('chat-files') < order.indexOf('chat-ai')) throw new Error(`files card not at the end of the turn: ${order}`)
    if (!/Claude|claude/.test(await text(byId(page, 'chat-title'))) && !(await text(byId(page, 'chat-title')))) throw new Error('title')
    await noOverflow(page, 'chat view')
    await shoot(page, join(dir, `chat-view-phone-light-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'dark' })
    await wait(600)
    await shoot(page, join(dir, `chat-view-phone-dark-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'light' })
  })

  await step('Chat: scrolling to the top loads the older page with gen and before, scroll stays anchored', async () => {
    const from = peer.events.length
    await msgs().evaluate((el) => {
      el.scrollTop = 0
      el.dispatchEvent(new Event('scroll'))
    })
    const ev = await after(peer, from, (e) => e.ev === 'chat.page')
    if (ev.before !== 200 || ev.gen !== 'sess-p1:1') throw new Error(`page args ${JSON.stringify(ev)}`)
    await page.waitForFunction(() => /Câu hỏi cũ nhất/.test(document.querySelector('[data-testid="chat-msgs"]')?.textContent ?? ''), null, { timeout: 8000 })
    const top = await msgs().evaluate((el) => el.scrollTop)
    if (top < 40) throw new Error(`scroll jumped to the top (${top})`)
  })

  await step('Chat: live items upsert in place, send shows my message at once and goes through ws.send, empty input turns into Stop', async () => {
    peer.send('chatitems')
    await page.waitForFunction(() => /Đã sửa xong, test pass/.test(document.querySelector('[data-testid="chat-msgs"]')?.textContent ?? ''), null, { timeout: 8000 })
    if ((await page.getByText('Lỗi do', { exact: false }).count()) !== 0) throw new Error('old revision still shown')
    const from = peer.events.length
    await byId(page, 'chat-input').fill('Chạy test giúp')
    const t0 = Date.now()
    await byId(page, 'chat-send').click()
    await byId(page, 'chat-out').waitFor({ timeout: 2000 })
    console.log(`[${browserName}] send to bubble ${Date.now() - t0} ms`)
    const sent = await after(peer, from, (e) => e.ev === 'send')
    if (sent.text !== 'Chạy test giúp' || sent.paneKey !== 'p1') throw new Error(`send ${JSON.stringify(sent)}`)
    if ((await byId(page, 'chat-send').getAttribute('data-mode')) !== 'stop') throw new Error('send button is not Stop while the agent works')
    await byId(page, 'chat-send').click()
    const keys = await after(peer, from, (e) => e.ev === 'keys')
    if (keys.keys !== 'stop') throw new Error(`stop keys ${JSON.stringify(keys)}`)
  })

  await step('Chat: a permission menu shows its labels as buttons; Undo within 3 s sends nothing, a pick sends ws.menu with sig and index', async () => {
    peer.send('chatmenu')
    await byId(page, 'chat-ask').waitFor({ timeout: 8000 })
    if (!/Do you want to proceed/.test(await text(byId(page, 'chat-ask')))) throw new Error('question')
    if ((await page.locator('[data-testid^="chat-ask-item-"]').count()) !== 3) throw new Error('menu items')
    await shoot(page, join(dir, `chat-ask-phone-light-${browserName}.png`))
    const from = peer.events.length
    await byId(page, 'chat-ask-item-1').click()
    await byId(page, 'chat-ask-sent').waitFor({ timeout: 500 })
    await byId(page, 'chat-ask-undo').click()
    await byId(page, 'chat-ask').waitFor()
    await wait(3400)
    if (peer.events.slice(from).some((e) => e.ev === 'menu')) throw new Error('undo still sent')
    await byId(page, 'chat-ask-item-2').click()
    const menu = await after(peer, from, (e) => e.ev === 'menu', 6000)
    if (menu.index !== 2 || menu.sig !== 'ab12cd34' || menu.paneKey !== 'p1') throw new Error(`menu ${JSON.stringify(menu)}`)
    peer.send('chatmenuoff')
    await byId(page, 'chat-ask').waitFor({ state: 'detached', timeout: 8000 })
  })

  await step('Chat: a file row opens the diff, a file outside the project is disabled; reset re-watches; terminal button opens the terminal and leaving unwatches', async () => {
    await byId(page, 'chat-file-src/app.ts').click()
    await byId(page, 'chat-diff').waitFor()
    await byId(page, 'diff-view').waitFor({ timeout: 8000 })
    await page.locator('[data-testid="chat-diff"]').click({ position: { x: 10, y: 10 } })
    await byId(page, 'chat-diff').waitFor({ state: 'detached' })
    if (!(await byId(page, 'chat-file-~/notes.md').isDisabled())) throw new Error('outside file is clickable')
    let from = peer.events.length
    peer.send('chatreset')
    await after(peer, from, (e) => e.ev === 'chat.watch' && e.paneKey === 'p1')
    from = peer.events.length
    await byId(page, 'chat-terminal').click()
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    if ((await byId(page, 'terminal-view').getAttribute('data-pane')) !== 'p1') throw new Error('wrong pane')
    await after(peer, from, (e) => e.ev === 'chat.unwatch' && e.paneKey === 'p1')
  })

  await step('Chat home: a shell row opens the terminal, New shell is one tap into the terminal, iPad shows the chat in the right column', async () => {
    await go(page, `/app/m/${id}`)
    await byId(page, 'chat-home').waitFor()
    await byId(page, 'chat-row-p3').click()
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    if ((await byId(page, 'terminal-view').getAttribute('data-pane')) !== 'p3') throw new Error('shell did not open the terminal')
    await go(page, `/app/m/${id}`)
    await byId(page, 'chat-home').waitFor()
    const from = peer.events.length
    await byId(page, 'new-shell').click()
    const open = await after(peer, from, (e) => e.ev === 'open')
    if (open.preset !== 'shell' || !open.cwd) throw new Error(`open ${JSON.stringify(open)}`)
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    await page.setViewportSize(IPAD)
    await go(page, `/app/m/${id}/c/p1`)
    await byId(page, 'chat-home').waitFor({ timeout: 8000 })
    await byId(page, 'detail-pane').locator('[data-testid="chat-view"]').waitFor({ timeout: 8000 })
    if ((await byId(page, 'chat-row-p1').getAttribute('aria-current')) !== 'true') throw new Error('row not marked')
    if (await byId(page, 'detail-pane').locator('[data-testid="back"]').count()) throw new Error('back button in the iPad detail')
    await noOverflow(page, 'ipad chat')
    await wait(400)
    await shoot(page, join(dir, `chat-ipad-light-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'dark' })
    await wait(600)
    await shoot(page, join(dir, `chat-ipad-dark-${browserName}.png`))
    await page.emulateMedia({ colorScheme: 'light' })
    await page.setViewportSize(PHONE)
  })

  await step('Chat: an old desktop without chat keeps the old behaviour, a Claude row opens the terminal and no chat command is sent', async () => {
    peer.send('chatoff')
    await go(page, `/app/m/${id}`)
    await reload(page)
    await page.waitForSelector('[data-testid="chat-home"][data-chat="0"]', { timeout: 10000 })
    const from = peer.events.length
    await byId(page, 'chat-row-p1').click()
    await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    await wait(300)
    if (peer.events.slice(from).some((e) => /^chat\./.test(e.ev ?? ''))) throw new Error('chat command sent to an old desktop')
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })

  await step('Chat: a push or inbox open of a Claude pane lands in the chat, a shell pane and an old desktop stay on the terminal', async () => {
    const open = (url) => page.evaluate((u) => navigator.serviceWorker.dispatchEvent(new MessageEvent('message', { data: { type: 'orrery-open', url: u } })), url)
    const at = (want) => page.waitForFunction((w) => location.pathname === w, want, { timeout: 5000 })
    peer.send('chatoff')
    await go(page, `/app/m/${id}`)
    await reload(page)
    await page.waitForSelector('[data-testid="chat-home"][data-chat="0"]', { timeout: 10000 })
    await open(`/app/m/${id}/p/p1`)
    await at(`/app/m/${id}/p/p1`)
    peer.send('chat')
    await go(page, `/app/m/${id}`)
    await reload(page)
    await page.waitForSelector('[data-testid="chat-home"][data-chat="1"]', { timeout: 10000 })
    await open(`/app/m/${id}/p/p3`)
    await at(`/app/m/${id}/p/p3`)
    await go(page, `/app/m/${id}`)
    await page.waitForSelector('[data-testid="chat-home"][data-chat="1"]', { timeout: 10000 })
    await open(`/app/m/${id}/p/p1`)
    await at(`/app/m/${id}/c/p1`)
    await byId(page, 'chat-view').waitFor({ timeout: 8000 })
    await go(page, '/app/')
    await byId(page, 'machines-view').waitFor()
  })
}
