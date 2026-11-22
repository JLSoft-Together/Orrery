import { mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { PHONE, wait } from './ctx.mjs'
import { go } from './ui-steps.mjs'

export const REMOTE_STEPS = 3
export const WIDTHS = [375, 390, 430]

export async function layoutIssues(page) {
  return page.evaluate(() => {
    const width = window.innerWidth
    const scroller = (el) => {
      for (let p = el.parentElement; p && p !== document.body; p = p.parentElement) {
        const s = getComputedStyle(p)
        if (/(auto|scroll)/.test(s.overflowX)) return true
        if (p.classList.contains('xterm')) return true
      }
      return false
    }
    const hidden = (el) => {
      for (let p = el; p && p !== document.body; p = p.parentElement) {
        const s = getComputedStyle(p)
        if (s.visibility === 'hidden' || s.opacity === '0' || s.display === 'none' || p.getAttribute('aria-hidden') === 'true' || p.inert) return true
      }
      return false
    }
    const name = (el) => `${el.tagName.toLowerCase()}${el.dataset.testid ? `[${el.dataset.testid}]` : ''}.${[...el.classList].join('.')}`
    const out = []
    for (const el of document.querySelectorAll('body *')) {
      const r = el.getBoundingClientRect()
      if (r.width < 1 || r.height < 1) continue
      if (r.right <= width + 1 && r.left >= -1) continue
      if (scroller(el) || hidden(el)) continue
      out.push(`${name(el)} ${Math.round(r.left)}..${Math.round(r.right)}`)
      if (out.length >= 6) break
    }
    return { doc: document.documentElement.scrollWidth - width, out }
  })
}

async function after(peer, from, match, ms = 8000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    const hit = peer.events.slice(from).find(match)
    if (hit) return hit
    await wait(40)
  }
  throw new Error(`peer event not seen: ${JSON.stringify(peer.events.slice(-6))}`)
}

async function boxLines(page) {
  return page.evaluate(() => (document.querySelector('[data-testid="terminal"]')?.ccmLines?.() ?? []).filter((l) => /[╭│╰]/.test(l)))
}

export async function remoteSteps(c, page, peer, failDir) {
  const { step, byId } = c
  const id = peer.info.machine
  mkdirSync(failDir, { recursive: true })
  const guard = async (label, fn) => {
    try {
      await fn()
    } catch (error) {
      await page.screenshot({ path: join(failDir, `${c.browserName}-${label}.png`) }).catch(() => {})
      throw error
    }
  }

  await step('Layout: no horizontal overflow and no element past the screen edge at 375 / 390 / 430 px (chat home, chat, monitor, workspace, terminal, settings, machines)', () =>
    guard('layout', async () => {
      peer.send('chat')
      await go(page, `/app/m/${id}`)
      await c.reload(page)
      await page.waitForSelector('[data-testid="chat-home"][data-chat="1"]', { timeout: 10000 })
      const routes = [
        ['machines', '/app/', 'machines-view'],
        ['chat home', `/app/m/${id}`, 'chat-home'],
        ['chat', `/app/m/${id}/c/p1`, 'chat-view'],
        ['monitor', `/app/m/${id}/mon`, 'monitor-view'],
        ['workspace', `/app/m/${id}/ws`, 'workspace-view'],
        ['terminal', `/app/m/${id}/p/p3`, 'terminal-view'],
        ['settings', '/app/settings', 'settings-view'],
      ]
      const bad = []
      const runs = [...WIDTHS.map((w) => [w, 0]), [375, 17], [375, 21]]
      for (const [w, font] of runs) {
        await page.setViewportSize({ width: w, height: 844 })
        await page.evaluate((px) => (document.documentElement.style.fontSize = px ? `${px}px` : ''), font)
        for (const [label, path, testid] of routes) {
          await go(page, path)
          await byId(page, testid).first().waitFor({ timeout: 8000 })
          await wait(450)
          const r = await layoutIssues(page)
          if (r.doc > 0 || r.out.length) bad.push(`${w}px${font ? ` text ${font}px` : ''} ${label}: doc +${r.doc} ${r.out.join(' | ')}`)
        }
      }
      await page.evaluate(() => (document.documentElement.style.fontSize = ''))
      await page.setViewportSize(PHONE)
      if (bad.length) throw new Error(bad.join(' ;; '))
    }),
  )

  await step('Chat is the default: Workspace card and Monitor row of a Claude pane open the chat, the terminal stays one tap away', () =>
    guard('chat-default', async () => {
      await go(page, `/app/m/${id}/ws`)
      await byId(page, 'workspace-view').waitFor()
      if ((await byId(page, 'ws-mode').count()) && !(await page.locator('[data-testid="pane-open-p1"]').count())) await byId(page, 'ws-mode').click()
      await byId(page, 'pane-open-p1').click()
      await page.waitForSelector('[data-testid="chat-view"]', { timeout: 8000 })
      await go(page, `/app/m/${id}/mon`)
      await byId(page, 'monitor-view').waitFor()
      await page.locator('[data-testid="monitor-list"] button[data-testid^="session-"]').first().waitFor()
      const rows = page.locator('[data-testid="monitor-list"] button[data-testid^="session-"]')
      let opened = false
      const seen = []
      const total = await rows.count()
      for (let i = 0; i < total; i++) {
        await go(page, `/app/m/${id}/mon`)
        await byId(page, 'monitor-view').waitFor()
        await wait(300)
        const row = rows.nth(i)
        const rid = await row.getAttribute('data-testid')
        await row.click()
        await page.waitForFunction(() => document.querySelector('[data-testid="chat-view"]') || document.querySelector('[data-testid="session-view"]'), null, { timeout: 8000 }).catch(() => {})
        seen.push(`${rid} -> ${new URL(page.url()).pathname}`)
        if (await byId(page, 'chat-view').count()) {
          opened = true
          break
        }
      }
      if (!opened) throw new Error(`no Monitor row opened the chat: ${seen.join(', ')}`)
      await byId(page, 'chat-terminal').click()
      await byId(page, 'terminal-view').waitFor({ timeout: 8000 })
    }),
  )

  await step('Terminal: always the PC size, scaled to the screen width without side scroll, the 180 column box is whole, the phone never asks for a resize', () =>
    guard('terminal', async () => {
      await page.setViewportSize(PHONE)
      const from = peer.events.length
      await go(page, `/app/m/${id}/p/p3`)
      await page.waitForSelector('[data-testid="terminal"][data-state="live"]', { timeout: 10000 })
      peer.send('wide')
      await page.waitForFunction(() => (document.querySelector('[data-testid="terminal"]')?.ccmLines?.() ?? []).some((l) => [...l].length === 180), null, { timeout: 8000 })
      await wait(400)
      for (const w of WIDTHS) {
        await page.setViewportSize({ width: w, height: 844 })
        await wait(400)
        const m = await page.evaluate(() => {
          const s = document.querySelector('[data-testid="terminal"]')
          const h = s?.querySelector('.host')
          const x = h?.querySelector('.xterm-screen')
          return { sw: h?.scrollWidth ?? 0, cw: h?.clientWidth ?? 0, right: x?.getBoundingClientRect().right ?? 0, hr: h?.getBoundingClientRect().right ?? 0, left: x?.getBoundingClientRect().left ?? 0, hl: h?.getBoundingClientRect().left ?? 0, scale: Number(s?.getAttribute('data-scale') ?? 1), doc: document.documentElement.scrollWidth - innerWidth }
        })
        if (m.sw > m.cw + 1 || m.right > m.hr + 1 || m.doc > 0) throw new Error(`${w}px terminal wider than the screen ${JSON.stringify(m)}`)
        if (!(m.scale < 1) || m.right < m.hr - 40) throw new Error(`${w}px PC size not fitted to the width ${JSON.stringify(m)}`)
        const lines = await boxLines(page)
        if (lines.length !== 3 || lines.some((l) => [...l].length !== 180)) throw new Error(`${w}px box broken: ${lines.map((l) => [...l].length)}`)
      }
      await page.setViewportSize(PHONE)
      await byId(page, 'line-input').fill('echo hi')
      await byId(page, 'line-send').click()
      const typed = await after(peer, from, (e) => e.ev === 'pty_in')
      if (typed.claim === true) throw new Error('typing sent a claim')
      await wait(300)
      if (peer.events.slice(from).some((e) => e.ev === 'resize')) throw new Error('the phone asked the desktop to resize')
      if ((await page.locator('[data-testid="term-mode"], [data-testid="term-fit"]').count()) !== 0) throw new Error('Desktop / Read or Fit button still shown')
    }),
  )
}
