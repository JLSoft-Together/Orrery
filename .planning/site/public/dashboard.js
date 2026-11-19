(async () => {
  const { esc, tables } = window.site;
  const $ = id => document.getElementById(id);
  const ICON = {
    ok: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6.2l2.3 2.3 4.7-5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    doing: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="4.2" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M6 1.8a4.2 4.2 0 0 1 0 8.4z" fill="currentColor"/></svg>',
    wait: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="4.4" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M6 3.4V6l1.8 1.2" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>',
    warn: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M6 1.5l4.6 8.5H1.4z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M6 5v2.2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="6" cy="8.6" r=".8" fill="currentColor"/></svg>',
    ask: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><circle cx="6" cy="6" r="4.6" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M4.6 4.7a1.5 1.5 0 1 1 2 1.4c-.4.2-.6.5-.6.9" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><circle cx="6" cy="8.7" r=".75" fill="currentColor"/></svg>'
  };
  const STATUS = {
    ok: ['p-c2', 'ok'], done: ['p-c2', 'ok'], doing: ['p-c6', 'doing'], wait: ['p-warn', 'wait'],
    warn: ['p-c1', 'warn'], bug: ['p-c1', 'warn'], todo: ['p-off', 'wait'], ask: ['p-c3', 'ask']
  };
  const pill = (kind, text) => { const [cls, ic] = STATUS[kind] || ['p-off', 'wait']; return `<span class="pill ${cls}">${ICON[ic]}${esc(text)}</span>`; };
  const WP_TEXT = { done: 'Xong', doing: 'Đang làm', wait: 'Chờ thiết bị' };
  const PEND_TEXT = { doing: 'Đang làm', bug: 'Lỗi', wait: 'Chờ thiết bị', todo: 'Treo nhỏ', ask: 'Chờ chốt' };
  const TYPE = { feat: ['p-c6', 'Tính năng'], fix: ['p-c1', 'Sửa lỗi'], docs: ['p-off', 'Tài liệu'], build: ['p-c5', 'Build'], test: ['p-c3', 'Test'], release: ['p-c4', 'Phát hành'] };
  const LANE = { desktop: ['p-c6', 'Desktop'], pwa: ['p-c4', 'Web app'], remote: ['p-c3', 'Remote'], disk: ['p-c5', 'Disk'], release: ['p-c2', 'Phát hành'] };
  const dm = s => { const [, m, d] = s.split('-'); return `${d}/${m}`; };

  let d;
  try {
    const r = await fetch('/data/dashboard.json', { cache: 'no-cache' });
    d = await r.json();
  } catch (e) {
    $('summary').textContent = 'Không tải được /data/dashboard.json.';
    return;
  }

  $('updated').textContent = dm(d.updated) + '/' + d.updated.slice(0, 4);
  $('ver').textContent = d.version;
  $('summary').textContent = d.summary;
  $('facts').innerHTML = [
    ['Phát hành', dm(d.released)],
    ['Nhánh', d.branch],
    ['Claude Code', d.targets.claude],
    ['Codex CLI', d.targets.codex],
    ['Số plan', d.planCount + ', mới nhất ' + d.lastPlan]
  ].map(([k, v]) => `<li><span>${esc(k)}</span> <b>${esc(v)}</b></li>`).join('');

  $('areas').innerHTML = d.areas.map(a => `<article class="area">
    <div class="area-h"><strong><span class="dot" style="background:var(--${a.color})"></span>${esc(a.name)}</strong>${pill(a.status, a.statusText)}</div>
    <div class="stack">${esc(a.stack)}</div>
    <p>${esc(a.now)}</p>
    <div class="from"><code>${esc(a.src)}</code></div>
  </article>`).join('');

  $('plans').innerHTML = d.plans.map(p => {
    const cells = Array.from({ length: p.total }, (_, i) => `<i class="${i < p.done ? 'd' : 'w'}"></i>`).join('');
    const pct = Math.round(p.done / p.total * 100);
    return `<div class="plan">
      <div class="pt">${esc(p.title)}<small>${esc(p.file)}</small></div>
      <div class="cells" role="img" aria-label="${p.done} trên ${p.total} đã xong">${cells}</div>
      <div class="ratio">${p.done}/${p.total}<small>${pct}%</small></div>
      <div class="pn">${esc(p.note)}</div>
    </div>`;
  }).join('');

  $('wps').innerHTML = d.wps.map(w => `<tr data-s="${esc(w.status)}">
    <td class="nw"><code>${esc(w.id)}</code></td>
    <td>${esc(w.title)}</td>
    <td class="hide-sm nw">${esc(w.plan)}</td>
    <td>${pill(w.status, WP_TEXT[w.status] || w.status)}</td>
    <td class="hide-sm nw">${w.commit ? `<code>${esc(w.commit)}</code>` : ''}</td>
  </tr>`).join('');
  $('wpf').addEventListener('click', e => {
    const b = e.target.closest('button');
    if (!b) return;
    $('wpf').querySelectorAll('button').forEach(x => x.classList.toggle('on', x === b));
    const f = b.dataset.f;
    $('wps').querySelectorAll('tr').forEach(tr => tr.classList.toggle('hide', f !== 'all' && tr.dataset.s !== f));
    tables();
  });

  $('pending').innerHTML = d.pending.map(p => `<div class="pend">
    <div class="pend-h"><strong>${esc(p.title)}</strong>${pill(p.kind, PEND_TEXT[p.kind] || p.kind)}</div>
    <p>${esc(p.detail)}</p>
  </div>`).join('');

  const roadColor = ['var(--c6)', 'var(--c4)', 'var(--off)'];
  $('road').innerHTML = d.roadmap.map((r, i) => `<li>
    <div class="st"><i style="background:${roadColor[i] || 'var(--off)'}"></i>${esc(r.stage)}</div>
    <ul>${r.items.map(x => `<li>${esc(x)}</li>`).join('')}</ul>
  </li>`).join('');

  $('mile').innerHTML = [...d.milestones].reverse().map(m => {
    const [cls, txt] = LANE[m.lane] || ['p-off', m.lane];
    return `<li><span class="d">${dm(m.date)}</span><span class="pill ${cls}">${esc(txt)}</span><span>${esc(m.label)}</span></li>`;
  }).join('');

  const byDay = new Map();
  d.commits.forEach(c => { if (!byDay.has(c.date)) byDay.set(c.date, []); byDay.get(c.date).push(c); });
  const typeColor = t => `var(--${({ feat: 'c6', fix: 'c1', docs: 'off', build: 'c5', test: 'c3', release: 'c4' })[t] || 'off'})`;
  $('days').innerHTML = [...byDay.entries()].map(([day, cs]) => `<div class="day">
    <div class="dd">${dm(day)}</div>
    <div class="bar" aria-hidden="true">${cs.map(c => `<i style="background:${typeColor(c.type)}"></i>`).join('')}</div>
    <div class="cnt"><b>${cs.length}</b> commit</div>
  </div>`).join('');
  $('act-sub').textContent = `${d.commits.length} commit gần nhất trên nhánh ${d.branch}, mỗi vạch là một commit theo loại.`;
  $('commits').innerHTML = d.commits.map(c => {
    const [cls, txt] = TYPE[c.type] || ['p-off', c.type];
    return `<tr><td class="nw">${dm(c.date)}</td><td><span class="pill ${cls}">${esc(txt)}</span></td><td>${c.scope ? `<code>${esc(c.scope)}</code> ` : ''}${esc(c.msg)}</td><td class="hide-sm nw"><code>${esc(c.h)}</code></td></tr>`;
  }).join('');

  tables();
})();
