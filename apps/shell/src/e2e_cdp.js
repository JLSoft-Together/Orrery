window.__ccmE2E = window.__ccmE2E || (() => {
  const st = { down: null, over: null, buttons: 0, capture: null, space: null }
  const PRESS = 'button:not([disabled]), a[href], summary, input[type="submit"], input[type="button"]'
  const TOGGLE = 'button:not([disabled]), summary, input[type="checkbox"], input[type="radio"], input[type="submit"], input[type="button"]'
  const selectable = (el) => !el.closest('.xterm') && !editable(el) && getComputedStyle(el).webkitUserSelect !== 'none' && getComputedStyle(el).userSelect !== 'none'
  const caret = (x, y) => {
    const r = document.caretRangeFromPoint?.(x, y)
    return r ? { n: r.startContainer, o: r.startOffset } : null
  }
  const proto = Element.prototype
  const nativeSet = proto.setPointerCapture
  const nativeRelease = proto.releasePointerCapture
  proto.setPointerCapture = function (id) {
    st.capture = this
    try { nativeSet.call(this, id) } catch {}
    this.dispatchEvent(new PointerEvent('gotpointercapture', { bubbles: true, pointerId: id, pointerType: 'mouse' }))
  }
  proto.releasePointerCapture = function (id) {
    const had = st.capture === this
    if (had) st.capture = null
    try { nativeRelease.call(this, id) } catch {}
    if (had) this.dispatchEvent(new PointerEvent('lostpointercapture', { bubbles: true, pointerId: id, pointerType: 'mouse' }))
  }
  proto.hasPointerCapture = function () { return st.capture === this }
  const target = (el) => (st.capture && st.capture.isConnected ? st.capture : el)
  const KEYS = { Enter: 13, Escape: 27, Backspace: 8, Tab: 9, ' ': 32, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40, Delete: 46, Home: 36, End: 35, PageUp: 33, PageDown: 34, Insert: 45, F1: 112, F2: 113, F5: 116, F12: 123 }
  const mods = (m) => ({ altKey: !!(m & 1), ctrlKey: !!(m & 2), metaKey: !!(m & 4), shiftKey: !!(m & 8) })
  const bit = (b) => (b === 2 ? 2 : b === 1 ? 4 : 1)
  const btn = (b) => (b === 'right' ? 2 : b === 'middle' ? 1 : 0)
  const at = (x, y) => document.elementFromPoint(x, y) || document.body
  const fire = (el, type, init, Ctor = MouseEvent, bubbles = true) => el.dispatchEvent(new Ctor(type, { bubbles, cancelable: true, composed: true, view: window, ...init }))
  const pointer = (init) => ({ pointerId: 1, pointerType: 'mouse', isPrimary: true, width: 1, height: 1, pressure: init.buttons ? 0.5 : 0, ...init })
  const editable = (el) => !!el && (el.isContentEditable || (el.tagName === 'TEXTAREA' && !el.readOnly) || (el.tagName === 'INPUT' && !el.readOnly && !/^(button|submit|reset|checkbox|radio|range|color|file|image|hidden)$/i.test(el.type)))
  const focusTarget = (el) => el.closest('input:not([disabled]), textarea:not([disabled]), select:not([disabled]), button:not([disabled]), a[href], [tabindex]:not([tabindex="-1"]), [tabindex="-1"], [contenteditable=""], [contenteditable="true"], summary')
  const common = (a, b) => {
    if (!a) return b
    for (let n = b; n; n = n.parentElement) if (n === a || n.contains(a)) return n
    return b
  }
  const ancestors = (el) => {
    const out = []
    for (let n = el; n; n = n.parentElement) out.push(n)
    return out
  }
  const hover = (el, base) => {
    const prev = st.over
    if (prev === el) return
    const from = prev ? ancestors(prev) : []
    const to = ancestors(el)
    if (prev && prev.isConnected) {
      fire(prev, 'pointerout', pointer({ ...base, relatedTarget: el }), PointerEvent)
      fire(prev, 'mouseout', { ...base, relatedTarget: el })
      for (const n of from) if (!to.includes(n)) {
        fire(n, 'pointerleave', pointer({ ...base, relatedTarget: el }), PointerEvent, false)
        fire(n, 'mouseleave', { ...base, relatedTarget: el }, MouseEvent, false)
      }
    }
    fire(el, 'pointerover', pointer({ ...base, relatedTarget: prev }), PointerEvent)
    fire(el, 'mouseover', { ...base, relatedTarget: prev })
    for (const n of to.reverse()) if (!from.includes(n)) {
      fire(n, 'pointerenter', pointer({ ...base, relatedTarget: prev }), PointerEvent, false)
      fire(n, 'mouseenter', { ...base, relatedTarget: prev }, MouseEvent, false)
    }
    st.over = el
  }
  const scrollable = (el, dx, dy) => {
    for (let s = el; s && s !== document.body; s = s.parentElement) {
      const cs = getComputedStyle(s)
      if ((dy && /(auto|scroll|overlay)/.test(cs.overflowY) && s.scrollHeight > s.clientHeight) || (dx && /(auto|scroll|overlay)/.test(cs.overflowX) && s.scrollWidth > s.clientWidth)) return s
    }
    return document.scrollingElement
  }
  function mouse(p) {
    const el = at(p.x, p.y)
    const b = btn(p.button)
    const base = { clientX: p.x, clientY: p.y, screenX: p.x, screenY: p.y, button: b, ...mods(p.modifiers || 0) }
    if (p.type === 'mouseMoved') {
      hover(el, { ...base, buttons: st.buttons })
      fire(target(el), 'pointermove', pointer({ ...base, button: -1, buttons: st.buttons }), PointerEvent)
      const moved = fire(el, 'mousemove', { ...base, button: 0, buttons: st.buttons })
      if (moved && st.buttons & 1 && st.anchor) {
        const f = caret(p.x, p.y)
        if (f) getSelection()?.setBaseAndExtent(st.anchor.n, st.anchor.o, f.n, f.o)
      }
      return
    }
    if (p.type === 'mousePressed') {
      hover(el, { ...base, buttons: st.buttons })
      st.buttons |= bit(b)
      st.down = el
      const go = fire(el, 'pointerdown', pointer({ ...base, buttons: st.buttons }), PointerEvent)
      const def = go && fire(el, 'mousedown', { ...base, buttons: st.buttons, detail: p.clickCount || 1 })
      if (def && b === 0) {
        st.anchor = selectable(el) ? caret(p.x, p.y) : null
        if (!el.closest('.xterm')) getSelection()?.removeAllRanges()
        const f = focusTarget(el)
        if (f) f.focus()
        else if (document.activeElement && document.activeElement !== document.body) document.activeElement.blur()
      }
      if (b === 2) fire(el, 'contextmenu', { ...base, buttons: st.buttons })
      return
    }
    if (p.type === 'mouseReleased') {
      st.buttons &= ~bit(b)
      const cap = target(el)
      fire(cap, 'pointerup', pointer({ ...base, buttons: st.buttons }), PointerEvent)
      if (st.capture) {
        const lost = st.capture
        st.capture = null
        lost.dispatchEvent(new PointerEvent('lostpointercapture', { bubbles: true, pointerId: 1, pointerType: 'mouse' }))
      }
      fire(el, 'mouseup', { ...base, buttons: st.buttons, detail: p.clickCount || 1 })
      if (b !== 2 && st.down) {
        const target = common(st.down, el)
        if (b === 0) {
          fire(target, 'click', { ...base, detail: p.clickCount || 1 })
          if ((p.clickCount || 1) === 2) fire(target, 'dblclick', { ...base, detail: 2 })
        } else fire(target, 'auxclick', { ...base, detail: 1 })
      }
      st.down = null
      st.anchor = null
      return
    }
    if (p.type === 'mouseWheel') {
      const dx = p.deltaX || 0
      const dy = p.deltaY || 0
      if (fire(el, 'wheel', { ...base, deltaX: dx, deltaY: dy, deltaMode: 0 }, WheelEvent)) scrollable(el, dx, dy)?.scrollBy(dx, dy)
    }
  }
  const moveFocus = (back) => {
    const list = [...document.querySelectorAll('a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])')].filter((n) => n.offsetParent !== null || n.getClientRects().length)
    if (!list.length) return
    const i = list.indexOf(document.activeElement)
    const next = list[(i + (back ? -1 : 1) + list.length) % list.length]
    next.focus()
  }
  function insert(p) {
    const t = document.activeElement
    if (!t || !p.text) return
    if (!document.execCommand('insertText', false, p.text) && editable(t)) {
      const s = t.selectionStart ?? t.value.length
      const e = t.selectionEnd ?? s
      t.value = t.value.slice(0, s) + p.text + t.value.slice(e)
      t.selectionStart = t.selectionEnd = s + p.text.length
      t.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: p.text }))
    }
  }
  function key(p) {
    const t = document.activeElement || document.body
    if (p.type === 'char') return insert({ text: p.text })
    const k = p.key || ''
    const code = p.windowsVirtualKeyCode || p.nativeVirtualKeyCode || KEYS[k] || (k.length === 1 ? k.toUpperCase().charCodeAt(0) : 0)
    const m = mods(p.modifiers || 0)
    const init = { key: k, code: p.code || '', keyCode: code, which: code, charCode: 0, repeat: !!p.autoRepeat, location: p.location || 0, ...m }
    const type = p.type === 'keyUp' ? 'keyup' : 'keydown'
    const go = fire(t, type, init, KeyboardEvent)
    const plain = !m.ctrlKey && !m.metaKey && !m.altKey
    if (type === 'keyup') {
      if (k === ' ' && go && st.space === t && t.matches?.(TOGGLE)) t.click()
      if (k === ' ') st.space = null
      return
    }
    if (!go) return
    const menu = m.metaKey && !m.ctrlKey && !m.altKey && !m.shiftKey ? { KeyC: 'copy', KeyX: 'cut', KeyA: 'selectAll' }[p.code] : null
    const cmds = p.commands?.length ? p.commands : menu ? [menu] : []
    for (const c of ['copy', 'cut']) if (cmds.includes(c)) t.dispatchEvent(new ClipboardEvent(c, { bubbles: true, cancelable: true, composed: true, clipboardData: new DataTransfer() }))
    if (cmds.includes('selectAll')) editable(t) ? document.execCommand('selectAll') : getSelection()?.selectAllChildren(document.body)
    if (cmds.length) return
    if (k === ' ' && plain) st.space = t
    if (k === 'Enter' && plain && t.matches?.(PRESS) && !editable(t)) return t.click()
    if (k === ' ' && plain && t.matches?.(TOGGLE)) return
    if (k === 'Tab' && !m.ctrlKey && !m.metaKey && !m.altKey) return moveFocus(m.shiftKey)
    const text = p.text ?? (k.length === 1 && !m.ctrlKey && !m.metaKey ? k : '')
    if (!text || m.ctrlKey || m.metaKey) return
    const press = fire(t, 'keypress', { ...init, charCode: text.charCodeAt(0), keyCode: text.charCodeAt(0) }, KeyboardEvent)
    if (!press) return
    if (text === '\r') {
      if (t.tagName === 'INPUT' && t.form) t.form.requestSubmit?.()
      else if (editable(t)) insert({ text: '\n' })
      return
    }
    if (editable(t)) insert({ text })
  }
  function drag(p) {
    const el = at(p.x, p.y)
    if (!st.dt || p.type === 'dragEnter') {
      st.dt = new DataTransfer()
      for (const it of p.data?.items || []) st.dt.setData(it.mimeType, it.data)
      for (const name of p.names || []) st.dt.items.add(new File([], name))
    }
    const type = { dragEnter: 'dragenter', dragOver: 'dragover', drop: 'drop', dragCancel: 'dragleave' }[p.type]
    if (type) el.dispatchEvent(new DragEvent(type, { bubbles: true, cancelable: true, composed: true, clientX: p.x, clientY: p.y, ...mods(p.modifiers || 0), dataTransfer: st.dt }))
    if (p.type === 'drop' || p.type === 'dragCancel') st.dt = null
  }
  return { mouse, key, insert, drag }
})()
