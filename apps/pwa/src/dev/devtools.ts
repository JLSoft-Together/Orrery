type Level = 'log' | 'info' | 'warn' | 'error'

function send(text: string): void {
  void fetch(`${import.meta.env.BASE_URL}__ccm_log`, { method: 'POST', body: text, keepalive: true }).catch(() => {})
}

function show(value: unknown): string {
  if (value instanceof Error) return `${value.name}: ${value.message}`
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

export async function startDevtools(): Promise<void> {
  for (const level of ['log', 'info', 'warn', 'error'] as Level[]) {
    const original = console[level].bind(console)
    console[level] = (...args: unknown[]) => {
      original(...args)
      send(`${level} ${args.map(show).join(' ')}`)
    }
  }
  window.addEventListener('error', (event) => send(`error ${event.message}`))
  window.addEventListener('unhandledrejection', (event) => send(`rejection ${show(event.reason)}`))
  const eruda = (await import('eruda')).default
  eruda.init()
}
