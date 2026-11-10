(() => {
  const win = __ccmShell.platform === 'win32'
  const call = (channel, args) => window.__TAURI_INTERNALS__.invoke('ccm', { channel, args })
  const DRAG = 'data-tauri-drag-region'
  const ICONS = {
    min: '<path d="M1 5.5h9" />',
    max: '<rect x="1.5" y="1.5" width="8" height="8" />',
    restore: '<path d="M3.5 3.5V1.5h7v7h-2" /><rect x="1.5" y="3.5" width="7" height="7" />',
    close: '<path d="M1.5 1.5l8 8M9.5 1.5l-8 8" />',
  }
  const svg = (name) => `<svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor" stroke-width="1" aria-hidden="true">${ICONS[name]}</svg>`
  const css = [
    `header.ccm-titlebar[data-title-bar] > div { padding-right: ${win ? 'calc(138px + 0.5rem)' : '0.5rem'} !important }`,
    '.ccm-shell-caption { position: fixed; top: 0; right: 0; height: 40px; display: flex; z-index: 2147483000; color: var(--ccm-ink, #1d1d1f) }',
    '.ccm-shell-caption button { width: 46px; height: 40px; border: 0; padding: 0; background: transparent; color: inherit; display: flex; align-items: center; justify-content: center; cursor: default; outline-offset: -2px }',
    '.ccm-shell-caption button:hover { background: rgb(128 128 128 / 0.2) }',
    '.ccm-shell-caption button[data-act=close]:hover { background: #c42b1c; color: #fff }',
  ].join('\n')

  const markDrag = (header) => {
    header.setAttribute(DRAG, '')
    for (const el of header.querySelectorAll('div, span, img, svg')) {
      if (!el.closest('button, a, input, select, textarea, [role=tab], [role=tablist] *, [role=status], [role=dialog], [role=menu], [tabindex]')) el.setAttribute(DRAG, '')
    }
  }

  let maxButton = null
  const syncMax = () =>
    call('shell:window', ['state']).then((s) => {
      if (!maxButton || !s) return
      maxButton.innerHTML = svg(s.maximized ? 'restore' : 'max')
      maxButton.setAttribute('aria-label', s.maximized ? 'Restore' : 'Maximize')
    }, () => undefined)

  const caption = () => {
    const bar = document.createElement('div')
    bar.className = 'ccm-shell-caption'
    bar.setAttribute('role', 'group')
    for (const [act, label] of [['min', 'Minimize'], ['max', 'Maximize'], ['close', 'Close']]) {
      const b = document.createElement('button')
      b.type = 'button'
      b.dataset.act = act
      b.tabIndex = -1
      b.setAttribute('aria-label', label)
      b.innerHTML = svg(act)
      b.addEventListener('click', () => call('shell:window', [act]).then(syncMax, () => undefined))
      bar.append(b)
      if (act === 'max') maxButton = b
    }
    document.body.append(bar)
    window.addEventListener('resize', syncMax)
    syncMax()
  }

  const keys = (e) => {
    const mod = e.ctrlKey || e.metaKey
    if (!mod || e.altKey) return
    const action = e.key === '=' || e.key === '+' ? 'in' : e.key === '-' ? 'out' : e.key === '0' ? 'reset' : null
    if (!action) return
    e.preventDefault()
    e.stopPropagation()
    call('ccm:zoom', [action]).catch(() => undefined)
  }

  const start = () => {
    const style = document.createElement('style')
    style.textContent = css
    document.head.append(style)
    if (win) caption()
    window.addEventListener('keydown', keys, true)
    let pending = false
    let header = null
    const inHeader = new MutationObserver(() => {
      if (pending || !header) return
      pending = true
      requestAnimationFrame(() => {
        pending = false
        if (header) markDrag(header)
      })
    })
    const find = new MutationObserver(() => attach())
    const attach = () => {
      const found = document.querySelector('header.ccm-titlebar')
      if (!found) return
      find.disconnect()
      header = found
      markDrag(found)
      inHeader.observe(found, { childList: true, subtree: true })
    }
    find.observe(document.body, { childList: true, subtree: true })
    attach()
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true })
  else start()
})()
