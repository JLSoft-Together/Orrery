(() => {
  const T = () => window.__TAURI_INTERNALS__
  const call = (args) => T().invoke('ccm', { channel: 'shell:guest', args })
  const noop = () => undefined
  const guests = new Map()
  const early = new Map()
  const live = new Set()
  const OVERLAY = '[role=dialog],[role=alertdialog],[role=menu],[role=tooltip],[data-context-menu]'
  const SKIP = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE', 'LINK', 'META'])
  let listening = false
  let raf = 0
  let until = 0
  let timer = 0

  const fire = (el, type, props) => {
    const ev = new Event(type)
    if (props) Object.assign(ev, props)
    el.dispatchEvent(ev)
  }

  const hits = (a, b) => a.width > 0 && a.height > 0 && a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top

  const overlays = () => {
    const out = [...document.querySelectorAll(OVERLAY)]
    for (const child of document.body ? document.body.children : []) {
      if (child.id === 'app' || SKIP.has(child.tagName) || child.classList.contains('ccm-shell-caption')) continue
      out.push(child)
    }
    return out
  }

  const covered = (el, r, layers) => layers.some((o) => !o.contains(el) && hits(o.getBoundingClientRect(), r))

  const measure = (el, layers) => {
    const r = el.getBoundingClientRect()
    const seen = el.isConnected && r.width > 1 && r.height > 1 && (!el.checkVisibility || el.checkVisibility({ visibilityProperty: true }))
    const scale = el.offsetWidth ? r.width / el.offsetWidth : 1
    return {
      x: r.left,
      y: r.top,
      w: r.width,
      h: r.height,
      show: seen && !covered(el, r, layers),
      scale: Number.isFinite(scale) && scale > 0 ? Math.round(scale * 1000) / 1000 : 1,
    }
  }

  const tick = () => {
    raf = 0
    const layers = live.size ? overlays() : []
    for (const s of live) s.sync(layers)
    if (performance.now() < until && live.size) raf = requestAnimationFrame(tick)
  }

  const kick = (ms = 450) => {
    until = Math.max(until, performance.now() + ms)
    if (!raf && live.size) raf = requestAnimationFrame(tick)
  }

  const listen = () => {
    if (listening) return
    listening = true
    const handler = T().transformCallback((e) => {
      const p = e.payload || {}
      const s = guests.get(p.id)
      if (s) return s.event(p)
      if (!early.has(p.id)) early.set(p.id, [])
      if (early.size < 64) early.get(p.id).push(p)
    })
    T().invoke('plugin:event|listen', { event: 'shell:guest', target: { kind: 'Any' }, handler }).catch(noop)
    const opts = { capture: true, passive: true }
    for (const type of ['resize', 'scroll', 'pointerdown', 'pointerup', 'click', 'keydown', 'keyup', 'wheel']) addEventListener(type, () => kick(), opts)
    for (const type of ['transitionrun', 'transitionend', 'animationstart', 'animationend']) document.addEventListener(type, () => kick(900), opts)
  }

  const watch = (s) => {
    live.add(s)
    listen()
    kick()
    if (!timer) timer = setInterval(() => (live.size ? tick() : (clearInterval(timer), (timer = 0))), 250)
  }

  let styled = false
  const upgrade = (el) => {
    if (!styled && document.head) {
      styled = true
      const style = document.createElement('style')
      style.textContent = 'webview { display: flex }'
      document.head.append(style)
    }
    const s = { id: 0, opening: null, url: '', src: '', title: '', back: false, fwd: false, zoom: 1, devtools: false, last: '', gone: 0 }
    const op = (name, ...rest) => (s.opening ? s.opening.then((id) => call([name, id, ...rest])) : Promise.reject(new Error('webview not attached')))
    const open = (url, tries = 0) => {
      if (s.opening || !url) return
      if (!el.isConnected) {
        if (tries < 100) setTimeout(() => open(url, tries + 1), 50)
        return
      }
      listen()
      const place = measure(el, overlays())
      s.last = JSON.stringify(place)
      s.opening = call(['open', 0, url, place, s.zoom]).then((id) => {
        s.id = id
        guests.set(id, s)
        for (const p of early.get(id) ?? []) s.event(p)
        early.delete(id)
        return id
      })
      s.opening.catch(() => {
        s.opening = null
      })
      watch(s)
    }
    s.sync = (layers) => {
      if (!el.isConnected) {
        s.gone ||= performance.now()
        if (s.id && performance.now() - s.gone > 3000) {
          call(['close', s.id]).catch(noop)
          guests.delete(s.id)
          live.delete(s)
          s.id = 0
          s.opening = null
          s.last = ''
          return
        }
      } else if (s.gone) {
        s.gone = 0
        if (!s.opening) open(s.url || s.src)
      }
      if (!s.id) return
      const place = measure(el, layers)
      const key = JSON.stringify(place)
      if (key === s.last) return
      s.last = key
      call(['bounds', s.id, place]).catch(noop)
    }
    s.event = (p) => {
      switch (p.type) {
        case 'start':
          s.url = p.url || s.url
          fire(el, 'did-start-loading')
          fire(el, 'did-navigate', { url: s.url, isMainFrame: true })
          break
        case 'finish':
          s.url = p.url || s.url
          s.back = !!p.canBack
          s.fwd = !!p.canForward
          fire(el, 'dom-ready')
          fire(el, 'did-finish-load')
          fire(el, 'did-stop-loading')
          break
        case 'nav':
          s.url = p.url || s.url
          s.back = !!p.canBack
          s.fwd = !!p.canForward
          fire(el, 'did-navigate-in-page', { url: s.url, isMainFrame: true })
          break
        case 'title':
          s.title = p.title || ''
          fire(el, 'page-title-updated', { title: s.title })
          break
        case 'console':
          fire(el, 'console-message', { level: p.level, message: p.message, line: p.line, sourceId: p.source })
          break
        case 'favicon':
          fire(el, 'page-favicon-updated', { favicons: p.favicons || [] })
          break
        case 'devtools':
          s.devtools = !!p.open
          break
      }
    }
    Object.defineProperty(el, 'src', {
      configurable: true,
      get: () => s.src,
      set: (v) => {
        const url = v == null ? '' : String(v)
        if (!url || url === s.src) return
        s.src = url
        if (s.opening) op('load', url).catch(noop)
        else queueMicrotask(() => open(url))
      },
    })
    Object.assign(el, {
      loadURL: (url) => {
        s.src = String(url)
        if (!s.opening) {
          open(s.src)
          return Promise.resolve()
        }
        return op('load', s.src).then(noop)
      },
      getURL: () => s.url || s.src,
      getTitle: () => s.title,
      canGoBack: () => s.back,
      canGoForward: () => s.fwd,
      goBack: () => void op('back').catch(noop),
      goForward: () => void op('forward').catch(noop),
      reload: () => void op('reload').catch(noop),
      reloadIgnoringCache: () => void op('reload').catch(noop),
      stop: () => void op('stop').catch(noop),
      isDevToolsOpened: () => s.devtools,
      openDevTools: () => op('devtools', 'open').then((v) => (s.devtools = !!v), noop),
      closeDevTools: () => op('devtools', 'close').then((v) => (s.devtools = !!v), noop),
      setZoomFactor: (f) => {
        s.zoom = Number(f) || 1
        op('zoom', s.zoom).catch(noop)
      },
      executeJavaScript: (code) => op('exec', String(code)),
      getWebContentsId: () => s.id,
      capturePage: () => op('capture').then((url) => ({ toDataURL: () => url || '', isEmpty: () => !url }), () => ({ toDataURL: () => '', isEmpty: () => true })),
      focus: () => void op('focus').catch(noop),
    })
  }

  const create = Document.prototype.createElement
  Document.prototype.createElement = function (tag, options) {
    const el = create.call(this, tag, options)
    if (typeof tag === 'string' && tag.toLowerCase() === 'webview') upgrade(el)
    return el
  }
})()
