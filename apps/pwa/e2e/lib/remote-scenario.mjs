import { makeCtx, PHONE } from './ctx.mjs'
import { CHAT_STEPS, chatSteps } from './chat-steps.mjs'
import { startPeer } from './procs.mjs'
import { REMOTE_STEPS, remoteSteps } from './remote-steps.mjs'

export const REMOTE_SCENARIO_STEPS = 1 + REMOTE_STEPS + CHAT_STEPS

export async function runRemoteScenario({ browserName, launch, env, record }) {
  await fetch(`${env.base}/__version?web=${env.webHash}`)
  const browser = await launch(browserName)
  const c = { ...makeCtx({ browser, browserName, env, record, videoPath: env.videoPath }), shoot: async () => {} }
  try {
    const peer = await startPeer(env.peerExe, env.relayHttp)
    c.peers.push(peer)
    const { page } = await c.newPage(PHONE)
    await c.step('pairs by pasting the code through the relay', async () => {
      await page.goto(`${env.base}/app/`)
      await c.ready(page)
      await c.pairPaste(page, peer)
      await c.byId(page, `machine-${peer.info.machine}`).waitFor({ timeout: 10000 })
    })
    await remoteSteps(c, page, peer, env.failDir)
    await page.goto(`${env.base}/app/`)
    await c.ready(page)
    await chatSteps(c, page, peer)
  } finally {
    for (const p of c.peers) p.stop()
    await browser.close()
  }
}
