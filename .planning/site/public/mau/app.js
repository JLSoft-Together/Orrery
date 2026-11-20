(() => {
  const root = document.documentElement
  const $ = (s) => document.querySelector(s)
  const store = {
    get(k, d) {
      try {
        const v = localStorage.getItem(k)
        return v == null ? d : JSON.parse(v)
      } catch (e) {
        return d
      }
    },
    set(k, v) {
      try {
        localStorage.setItem(k, JSON.stringify(v))
      } catch (e) {}
    },
  }

  const picked = new Set(store.get('mau.picked', []))
  const keepMap = store.get('mau.keep', {})
  let randOn = store.get('mau.rand', true)
  const allOld = [...MAU_CURRENT.next, ...MAU_CURRENT.classic]
  const isKept = (t) => (t.id in keepMap ? keepMap[t.id] : t.keep)

  const icon = (id, cls) => `<svg${cls ? ` class="${cls}"` : ''}><use href="#i-${id}"/></svg>`
  const scheme = () => (root.dataset.theme === 'dark' ? 'dark' : 'light')

  function preview() {
    return `
      <div class="app">
        <div class="rail">
          <div class="rail-top"><span class="logo">O</span>Phiên</div>
          <div class="row on"><span class="g-claude">${icon('claude')}</span><span class="rname">api-server</span><span class="chip c-working">${icon('working')}Đang chạy</span></div>
          <div class="row"><span class="g-codex">${icon('codex')}</span><span class="rname">mobile</span><span class="chip c-waiting">${icon('waiting')}Chờ trả lời</span></div>
          <div class="row"><span class="g-shell">${icon('shell')}</span><span class="rname">build</span><span class="chip c-idle">${icon('idle')}Rảnh</span></div>
          <div class="row"><span class="g-claude">${icon('claude')}</span><span class="rname">docs</span><span class="chip c-error">${icon('error')}Lỗi</span></div>
          <div class="rail-foot">
            <div class="meter">5h<b style="--w:42%"></b>42%</div>
            <div class="meter">Tuần<b style="--w:68%"></b>68%</div>
          </div>
        </div>
        <div class="main">
          <div class="tabs">
            <span class="tab on"><span class="g-claude">${icon('claude')}</span>api-server<span class="x">${icon('x')}</span></span>
            <span class="tab"><span class="g-codex">${icon('codex')}</span>mobile</span>
            <span class="tab t3"><span class="g-shell">${icon('shell')}</span>build</span>
            <span class="tab-add">${icon('plus')}</span>
          </div>
          <div class="pane">
            <div class="phead"><span class="g-claude">${icon('claude')}</span><span class="t">api-server</span><span class="chip c-working">${icon('working')}Đang chạy</span><span class="sp"></span><span class="ic opt">${icon('split')}</span><span class="ic">${icon('x')}</span></div>
            <div class="well">
              <div><span class="w-acc">❯</span> npm test <span class="w-dim">--watch</span></div>
              <div class="w-g">✓ 128 passed <span class="w-dim">in 2.4s</span></div>
              <div class="w-y">! 2 warnings</div>
              <div class="w-r">✗ auth.test.ts:41</div>
              <div><span class="w-b">src/cache.ts</span> <span class="w-dim">12 ms</span></div>
            </div>
            <div class="diff">
              <div class="diff-h"><span>cache.ts</span><span class="a">+1</span><span class="d">-1</span></div>
              <div class="l add">+ return cache.get(id)</div>
              <div class="l del">- return db.find(id)</div>
            </div>
            <div class="actions"><span class="b pri">Gửi</span><span class="b sec">Huỷ</span></div>
          </div>
          <div class="toast"><span class="ok">${icon('done')}</span>Đã lưu<span class="u">Hoàn tác</span></div>
        </div>
      </div>`
  }

  function applyTokens(el, t) {
    for (const [k, v] of Object.entries(t)) el.style.setProperty('--' + k, v)
  }

  function minText(t) {
    const rows = mauChecks(t)
    const text = rows.filter((r) => r.min === 4.5)
    const min = Math.min(...text.map((r) => r.ratio))
    const pass = rows.every((r) => r.ratio >= r.min)
    return { min, pass }
  }

  const DOTS = ['canvas', 'surface', 'ink', 'accent', 'working', 'waiting', 'done', 'error', 'agent-claude', 'agent-codex', 'agent-shell', 'well']

  function renderNew() {
    const s = scheme()
    const grid = $('#newGrid')
    grid.innerHTML = ''
    for (const p of MAU_PALETTES) {
      const t = p[s]
      const r = minText(t)
      const card = document.createElement('button')
      card.type = 'button'
      card.className = 'pcard'
      card.dataset.id = p.id
      card.setAttribute('aria-pressed', String(picked.has(p.id)))
      card.innerHTML = `${preview()}
        <div class="pmeta">
          <span class="pick">${icon('check')}</span>
          <span class="grow"><span class="pname">${p.name}</span><span class="ptag">${p.tag}</span></span>
          <span class="aa${r.pass ? '' : ' fail'}" title="Contrast min ${r.min.toFixed(2)}:1">${r.pass ? icon('check') : ''}AA ${r.min.toFixed(1)}</span>
        </div>
        <div class="dots">${DOTS.map((k) => `<i style="background:${t[k]}" title="${k} ${t[k]}"></i>`).join('')}</div>`
      applyTokens(card.querySelector('.app'), t)
      card.addEventListener('click', () => {
        if (picked.has(p.id)) picked.delete(p.id)
        else picked.add(p.id)
        card.setAttribute('aria-pressed', String(picked.has(p.id)))
        store.set('mau.picked', [...picked])
        refresh()
      })
      grid.appendChild(card)
    }
  }

  function oldCard(t) {
    const el = document.createElement('button')
    el.type = 'button'
    const kept = isKept(t)
    el.className = 'ocard' + (kept ? '' : ' off')
    el.setAttribute('aria-pressed', String(kept))
    const sw = t.custom
      ? '<div class="osw custom"></div>'
      : `<div class="osw">${t.modes.map((c) => `<span style="background:${c}"></span>`).join('')}<i class="acc" style="background:${t.accent}"></i><i class="wl" style="background:${t.well};box-shadow:inset 0 0 0 1px rgb(127 127 127 / .3)"></i>${t.stars ? `<span class="star">${icon('star')}</span>` : ''}</div>`
    el.innerHTML = `${sw}<div class="orow"><span class="oname">${t.name}</span><span class="tag ${kept ? 'keep' : 'drop'}">${kept ? 'Giữ' : 'Bỏ'}</span></div>`
    el.addEventListener('click', () => {
      keepMap[t.id] = !isKept(t)
      store.set('mau.keep', keepMap)
      el.replaceWith(oldCard(t))
      refresh()
    })
    return el
  }

  function renderOld() {
    const n = $('#nextGrid')
    const c = $('#classicGrid')
    n.innerHTML = ''
    c.innerHTML = ''
    for (const t of MAU_CURRENT.next) n.appendChild(oldCard(t))
    for (const t of MAU_CURRENT.classic) c.appendChild(oldCard(t))
  }

  function pool() {
    const s = scheme()
    const a = MAU_PALETTES.filter((p) => picked.has(p.id)).map((p) => ({ id: p.id, name: p.name, bg: p[s].canvas, accent: p[s].accent, fresh: true }))
    const b = allOld.filter((t) => isKept(t) && !t.custom && (s === 'dark' || t.modes.length > 1 || t.modes[0] > '#8')).map((t) => ({ id: t.id, name: t.name, bg: s === 'dark' ? t.modes[t.modes.length - 1] : t.modes[0], accent: t.accent }))
    return [...a, ...b]
  }

  function renderRand() {
    const items = pool()
    $('#rand').classList.toggle('off', !randOn)
    $('#randOn').setAttribute('aria-checked', String(randOn))
    $('#randPool').textContent = `${items.length} preset`
    $('#randStack').innerHTML = items.map((x) => `<i data-id="${x.id}" title="${x.name}" style="background:${x.bg};--a:${x.accent}"></i>`).join('')
  }

  function refresh() {
    renderRand()
    const kept = allOld.filter(isKept)
    $('#sum').innerHTML = `<span>Thêm <b>${picked.size}</b></span><span>Giữ <b>${kept.length}</b></span><span>Bỏ <b>${allOld.length - kept.length}</b></span><span>Ngẫu nhiên <b>${randOn ? 'bật' : 'tắt'}</b></span>`
  }

  function setScheme(s) {
    root.dataset.theme = s
    try {
      localStorage.setItem('mau.scheme', s)
    } catch (e) {}
    document.querySelectorAll('[data-scheme]').forEach((b) => b.setAttribute('aria-checked', String(b.dataset.scheme === s)))
    renderNew()
    renderRand()
  }

  document.querySelectorAll('[data-scheme]').forEach((b) => b.addEventListener('click', () => setScheme(b.dataset.scheme)))

  $('#randOn').addEventListener('click', () => {
    randOn = !randOn
    store.set('mau.rand', randOn)
    refresh()
  })

  $('#randTry').addEventListener('click', () => {
    const items = pool()
    if (!items.length) return
    const x = items[Math.floor(Math.random() * items.length)]
    document.querySelectorAll('#randStack i').forEach((i) => i.classList.toggle('hit', i.dataset.id === x.id))
    const out = $('#randPick')
    out.hidden = false
    out.innerHTML = `Lần mở này: <b>${x.name}</b>`
    if (x.fresh) {
      const card = document.querySelector(`.pcard[data-id="${x.id}"]`)
      if (card) {
        card.classList.remove('flash')
        void card.offsetWidth
        card.classList.add('flash')
      }
    }
  })

  function summary() {
    const add = MAU_PALETTES.filter((p) => picked.has(p.id)).map((p) => `${p.name} (${p.id})`)
    const fmt = (list) => list.map((t) => `${t.name} (${t.id})`).join(', ') || 'không'
    const keepNext = MAU_CURRENT.next.filter(isKept)
    const dropNext = MAU_CURRENT.next.filter((t) => !isKept(t))
    const keepClassic = MAU_CURRENT.classic.filter(isKept)
    const dropClassic = MAU_CURRENT.classic.filter((t) => !isKept(t))
    const p = pool().map((x) => x.name)
    return [
      'Orrery v108 màu, lựa chọn:',
      `Thêm bảng màu mới: ${add.join(', ') || 'không'}`,
      `UI next giữ: ${fmt(keepNext)}`,
      `UI next bỏ: ${fmt(dropNext)}`,
      `Classic giữ: ${fmt(keepClassic)}`,
      `Classic bỏ: ${fmt(dropClassic)}`,
      `Ngẫu nhiên: ${randOn ? `bật, lúc mở app chọn 1 trong ${p.length}: ${p.join(', ')}` : 'tắt'}`,
      'Token: .planning/RESEARCH-v108-palettes.md',
    ].join('\n')
  }

  async function copyText(text) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch (e) {
      const ta = document.createElement('textarea')
      ta.value = text
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      let ok = false
      try {
        ok = document.execCommand('copy')
      } catch (e2) {}
      ta.remove()
      return ok
    }
  }

  $('#copy').addEventListener('click', async () => {
    const ok = await copyText(summary())
    const label = $('#copy span')
    label.textContent = ok ? 'Đã copy' : 'Không copy được'
    setTimeout(() => (label.textContent = 'Copy lựa chọn'), 1600)
  })

  window.mauSummary = summary
  setScheme(scheme())
  renderOld()
  refresh()
})()
