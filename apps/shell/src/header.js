(() => {
  const call = (channel, args) => window.__TAURI_INTERNALS__.invoke('ccm', { channel, args })
  const DRAG = 'data-tauri-drag-region'
  const css = 'header.ccm-titlebar[data-title-bar] > div { padding-right: 0.5rem !important }'

  const markDrag = (header) => {
    header.setAttribute(DRAG, '')
    for (const el of header.querySelectorAll('div, span, img, svg')) {
      if (!el.closest('button, a, input, select, textarea, [role=tab], [role=tablist] *, [role=status], [role=dialog], [role=menu], [tabindex]')) el.setAttribute(DRAG, '')
    }
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
