(async () => {
  const { esc } = window.site;
  const $ = id => document.getElementById(id);
  const GROUP = { ui: ['p-c6', 'UI/UX'], clean: ['p-c5', 'Clean'], feature: ['p-c4', 'Tính năng'] };
  const PRIO = { P1: 'p-c1', P2: 'p-warn', P3: 'p-off' };
  const EFFORT = { S: 1, M: 2, L: 3 };
  const list = v => Array.isArray(v) ? `<ul>${v.map(x => `<li>${esc(x)}</li>`).join('')}</ul>` : esc(v);

  let d = { items: [] };
  try {
    const r = await fetch('/data/proposals.json', { cache: 'no-cache' });
    d = await r.json();
  } catch (e) {}

  if (d.source) $('src').textContent = d.source;
  if (d.updated) { const [y, m, dd] = d.updated.split('-'); $('updated').textContent = `${dd}/${m}/${y}`; }
  const items = d.items || [];
  const box = $('props');

  if (!items.length) {
    $('notice').hidden = false;
    const ghosts = Array.from({ length: 6 }, (_, i) => `<article class="prop ghost">
      <div class="prop-h"><div class="prop-tags"><span class="id">#${i + 1}</span><span class="pill p-off">Nhóm</span><span class="pill p-off">Ưu tiên</span><span class="pill p-off">Công sức</span></div>
      <h3>Đề xuất ${i + 1}, đang cập nhật</h3></div>
      <div class="prop-body">
        <div class="prop-text" aria-hidden="true"><div class="line"></div><div class="line"></div><div class="line s"></div></div>
        <div class="prop-wf"><div class="wf-empty">Wireframe tương tác sẽ hiện ở đây</div></div>
      </div>
    </article>`).join('');
    box.innerHTML = ghosts;
    return;
  }

  if (d.lede) $('lede').textContent = d.lede;
  const byRef = Object.fromEntries(items.map(p => [p.id, p.anchor]));
  const refLink = r => r.split(',').map(x => x.trim()).map(x => byRef[x] ? `<a href="#${byRef[x]}">${esc(x)}</a>` : esc(x)).join(', ');
  if (d.order) {
    $('order').innerHTML = d.order.map(o => { const [gc, gt] = GROUP[o.group] || ['p-off', o.group]; return `<tr><td class="nw">${o.rank}</td><td>${refLink(o.ref)} ${esc(o.what)}</td><td><span class="pill ${gc}">${esc(gt)}</span></td><td class="nw">${esc(o.effort)}</td><td>${esc(o.why)}</td></tr>`; }).join('');
    window.site.tables();
  }
  if (d.asks) {
    $('asks').innerHTML = d.asks.map(a => `<div class="ask"><span class="ref">Liên quan ${refLink(a.ref)}</span><strong>${esc(a.q)}</strong><div class="opts">${a.options.map(o => `<span>${esc(o)}</span>`).join('')}</div></div>`).join('');
  }

  const counts = { all: items.length, ui: 0, clean: 0, feature: 0 };
  items.forEach(p => { counts[p.group] = (counts[p.group] || 0) + 1; });
  document.querySelectorAll('[data-n]').forEach(n => { n.textContent = counts[n.dataset.n] || 0; });

  items.forEach((p, i) => {
    const [gc, gt] = GROUP[p.group] || ['p-off', p.group];
    const el = document.createElement('article');
    el.className = 'prop';
    el.id = p.anchor || p.id;
    el.dataset.g = p.group;
    el.dataset.i = i;
    el.innerHTML = `<div class="prop-h">
        <div class="prop-tags"><span class="id">${esc(p.id)}</span><span class="pill ${gc}">${esc(gt)}</span><span class="pill ${PRIO[p.priority] || 'p-off'}">${esc(p.priority)}</span><span class="pill p-off">Công sức ${esc(p.effortText || p.effort)}</span><span class="pill p-off">Thứ tự ${esc(p.rank)}</span></div>
        <h3>${esc(p.title)}</h3>
      </div>
      <div class="prop-body">
        <div class="prop-text"><dl>
          ${p.problem ? `<dt>Vấn đề</dt><dd>${list(p.problem)}</dd>` : ''}
          ${p.proposal ? `<dt>Đề xuất</dt><dd>${list(p.proposal)}</dd>` : ''}
          ${p.impact ? `<dt>Lợi ích</dt><dd>${list(p.impact)}</dd>` : ''}
          ${p.risk ? `<dt>Rủi ro</dt><dd class="risk">${list(p.risk)}</dd>` : ''}
          ${p.files ? `<dt>Đụng tới</dt><dd>${p.files.map(f => `<code>${esc(f)}</code>`).join(' ')}</dd>` : ''}
        </dl></div>
        <div class="prop-wf"><div class="wf-label"><span>Wireframe</span><span>${esc(p.wfHint || 'Bấm thử')}</span></div><div class="wf" data-wf="${esc(p.wireframe || '')}"></div></div>
      </div>`;
    box.appendChild(el);
    const host = el.querySelector('.wf');
    const fn = window.WF[p.wireframe];
    if (fn) fn(host); else host.innerHTML = '<div class="wf-empty">Wireframe đang cập nhật</div>';
  });

  const apply = () => {
    const g = $('gf').querySelector('.on').dataset.g;
    const s = $('sort').value;
    const els = [...box.children];
    els.forEach(e => { e.hidden = g !== 'all' && e.dataset.g !== g; });
    const key = e => {
      const p = items[+e.dataset.i];
      if (s === 'prio') return String(p.rank || 99).padStart(3, '0');
      if (s === 'effort') return (EFFORT[p.effort] || 9) + (p.priority || 'P9');
      return String(+e.dataset.i).padStart(3, '0');
    };
    els.sort((a, b) => key(a).localeCompare(key(b))).forEach(e => box.appendChild(e));
  };
  $('gf').addEventListener('click', e => {
    const b = e.target.closest('button');
    if (!b) return;
    $('gf').querySelectorAll('button').forEach(x => x.classList.toggle('on', x === b));
    apply();
  });
  $('sort').addEventListener('change', apply);
  apply();
})();
