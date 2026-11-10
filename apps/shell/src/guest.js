(() => {
  if (window.top !== window || window.__ccmGuestReply) return
  const T = window.__TAURI_INTERNALS__
  if (!T || typeof T.invoke !== 'function') return
  const invoke = T.invoke.bind(T)
  const send = (kind, data) => {
    try {
      invoke('guest', { kind, data }).catch(() => undefined)
    } catch {}
  }
  const plain = (v) => {
    if (v === undefined) return null
    try {
      return JSON.parse(JSON.stringify(v))
    } catch {
      return null
    }
  }
  const reason = (e) => String((e && e.message) || e)
  Object.defineProperty(window, '__ccmGuestReply', {
    value: (nonce, ok, v) => send('reply', ok ? { nonce, ok: true, value: plain(v) } : { nonce, ok: false, error: reason(v) }),
    writable: false,
    configurable: false,
  })

  const nav = () => send('nav', {})
  for (const name of ['pushState', 'replaceState']) {
    const orig = history[name]
    history[name] = function (...args) {
      const out = orig.apply(this, args)
      queueMicrotask(nav)
      return out
    }
  }
  addEventListener('popstate', nav)
  addEventListener('hashchange', nav)

  let budget = 50
  let since = Date.now()
  const take = () => {
    const now = Date.now()
    if (now - since > 1000) {
      budget = 50
      since = now
    }
    return budget-- > 0
  }
  const text = (list) =>
    list
      .map((x) => {
        if (typeof x === 'string') return x
        if (x instanceof Error) return x.stack || x.message
        try {
          return JSON.stringify(x)
        } catch {
          return String(x)
        }
      })
      .join(' ')
      .slice(0, 4000)
  for (const [name, level] of [['error', 'error'], ['warn', 'warning']]) {
    const orig = console[name]
    console[name] = function (...args) {
      if (take()) send('console', { level, message: text(args) })
      return orig.apply(this, args)
    }
  }
  addEventListener('error', (e) => {
    if (e.message && take()) send('console', { level: 'error', message: String(e.message), line: e.lineno, source: e.filename })
  })
  addEventListener('unhandledrejection', (e) => {
    if (take()) send('console', { level: 'error', message: 'Uncaught (in promise) ' + text([e.reason]) })
  })

  const icons = () => {
    const list = [...document.querySelectorAll('link[rel~="icon"]')].map((l) => l.href).filter(Boolean)
    if (!list.length) list.push(new URL('/favicon.ico', location.href).href)
    send('favicon', { favicons: list.slice(0, 8) })
  }
  if (document.readyState === 'loading') addEventListener('DOMContentLoaded', icons, { once: true })
  else icons()

  const mac = /Mac/.test(navigator.platform || navigator.userAgent)
  const DIGIT = /^Digit[1-9]$/
  const chord = (e) => {
    if (e.isComposing) return null
    const { ctrlKey: ctrl, shiftKey: shift, altKey: alt, metaKey: meta, code } = e
    const hit = () => ({ key: e.key, code, ctrlKey: ctrl, shiftKey: shift, altKey: alt, metaKey: meta })
    if (code === 'Backquote' && ctrl && !shift && !alt && !meta) return hit()
    if (mac) {
      if (ctrl && alt && !meta && !shift && code === 'KeyN') return hit()
      if (!meta || ctrl || alt) return null
      if (['KeyT', 'KeyW', 'KeyK', 'KeyP'].includes(code)) return shift ? null : hit()
      if (['KeyA', 'KeyE', 'KeyO', 'KeyF'].includes(code)) return shift ? hit() : null
      if (code === 'KeyD') return hit()
      if (code === 'BracketLeft' || code === 'BracketRight') return shift ? hit() : null
      if (DIGIT.test(code)) return shift ? null : hit()
      return null
    }
    if (meta) return null
    if (ctrl && shift && !alt && ['KeyT', 'KeyW', 'KeyA', 'KeyP', 'KeyE', 'KeyO'].includes(code)) return hit()
    if (alt && shift && !ctrl && ['KeyD', 'KeyF', 'Equal', 'NumpadAdd', 'Minus', 'NumpadSubtract'].includes(code)) return hit()
    if (ctrl && !alt && code === 'Tab') return hit()
    if (ctrl && !alt && !shift && code === 'KeyK') return hit()
    const altGraph = e.getModifierState && e.getModifierState('AltGraph')
    if (ctrl && alt && !shift && DIGIT.test(code) && /^[1-9]$/.test(e.key) && !altGraph) return hit()
    if (ctrl && alt && !shift && code === 'KeyN' && e.key.toLowerCase() === 'n' && !altGraph) return hit()
    return null
  }
  addEventListener(
    'keydown',
    (e) => {
      const c = chord(e)
      if (c) {
        e.preventDefault()
        e.stopImmediatePropagation()
        send('key', c)
        return
      }
      const key = (e.key || '').toLowerCase()
      const mod = e.ctrlKey || e.metaKey
      if (key === 'f5' || (mod && key === 'r')) {
        e.preventDefault()
        location.reload()
      } else if (key === 'f12') {
        e.preventDefault()
        send('devtools', {})
      }
    },
    true,
  )
})()
