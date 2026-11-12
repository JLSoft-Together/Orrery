import type { Plugin } from 'vite'

export function devLogPlugin(): Plugin {
  return {
    name: 'ccm-pwa-dev-log',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use('/__ccm_log', (req, res) => {
        if (req.method !== 'POST') {
          res.statusCode = 405
          res.end()
          return
        }
        const chunks: Buffer[] = []
        req.on('data', (chunk: Buffer) => chunks.push(chunk))
        req.on('end', () => {
          const body = Buffer.concat(chunks).toString('utf8').slice(0, 4000)
          const from = req.socket.remoteAddress ?? '?'
          console.log(`[pwa ${from}] ${body}`)
          res.statusCode = 204
          res.end()
        })
      })
    },
  }
}
