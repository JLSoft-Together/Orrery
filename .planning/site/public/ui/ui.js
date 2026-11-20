(async () => {
  const { esc } = window.site;
  const $ = id => document.getElementById(id);
  const h = (tag, cls, html) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; };
  const WF = {};
  const AREA = { desktop: ['where-d', 'Desktop'], phone: ['where-p', 'Điện thoại'] };
  const X = '<svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true"><path d="M3 3l6 6M9 3L3 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>';
  const FOCUS = '<svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true"><path d="M2 5V2h3M10 5V2H7M2 7v3h3M10 7v3H7" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>';

  const compare = (host, views, start = 1) => {
    const bar = h('div', 'wf-cmp');
    bar.setAttribute('role', 'tablist');
    const stage = h('div', 'wf-stage');
    const show = i => {
      [...bar.children].forEach((b, j) => { b.classList.toggle('on', i === j); b.setAttribute('aria-selected', String(i === j)); });
      stage.innerHTML = '';
      views[i](stage);
    };
    ['Hiện tại', 'Đề xuất'].forEach((n, i) => {
      const b = h('button', '', n);
      b.type = 'button';
      b.setAttribute('role', 'tab');
      b.addEventListener('click', () => show(i));
      bar.appendChild(b);
    });
    host.append(bar, stage);
    show(start);
  };
  const hint = (stage, text) => stage.appendChild(h('p', 'wf-hint', text));

  const STATES = [
    { k: 'run', w: 'Đang chạy', tool: 'Edit src/app.ts', t: 'chạy 38s' },
    { k: 'wait', w: 'Chờ bạn', tool: 'Bash: npm test?', t: 'chờ 4 phút' },
    { k: 'idle', w: 'Rảnh', tool: 'Xong lượt', t: '12 phút trước' },
    { k: 'err', w: 'Lỗi', tool: 'cargo test fail', t: '1 phút trước' }
  ];

  WF.railRow = host => {
    let si = 1;
    compare(host, [
      stage => {
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-body"><div class="wf-railbox">
          <div class="wf-grp"><span>ccm-e2e-v103-71-proj-28736</span><span>3</span></div>
          ${['11s', '3s', '2m'].map((t, i) => `<div class="wf-rrow${i === 0 ? ' focus' : ''}"><span class="wf-glyph"></span><span class="t">ccm-e2e-v103-71-proj-28736${i ? ' ' + (i + 1) : ''}</span><span class="tm">${t}</span><span class="s"><span class="tiny">${i === 2 ? 'Claude' : 'Shell'}</span><span class="x tiny">ccm-e2e-v103-71-proj-28736</span></span></div>`).join('')}
        </div></div>`;
        stage.appendChild(win);
        hint(stage, 'Tên dòng và dòng phụ đều là tên project, bị cắt. Trạng thái chỉ là vòng nét đứt, thời gian không nói là gì.');
      },
      stage => {
        const win = h('div', 'wf-win');
        const box = h('div', 'wf-railbox');
        const rows = [
          { n: 'Claude 1', s: () => STATES[si] },
          { n: 'Codex review', s: () => STATES[0] },
          { n: 'Shell build', s: () => STATES[2] }
        ];
        const draw = () => {
          box.innerHTML = `<div class="wf-grp"><span>ccm-e2e...28736</span><span>3</span></div>` + rows.map((r, i) => {
            const s = r.s();
            return `<div class="wf-rrow${i === 0 ? ' focus' : ''}" tabindex="0"><span class="wf-glyph ${s.k}"></span><span class="t">${r.n}</span><span class="tm hide-h">${s.t}</span><span class="s"><span class="st ${s.k}">${s.w}</span><span class="x">${s.tool}</span></span><span class="acts"><button type="button" class="wf-ic" aria-label="Focus pane">${FOCUS}</button><button type="button" class="wf-ic" aria-label="Đóng pane">${X}</button></span></div>`;
          }).join('');
        };
        draw();
        const body = h('div', 'wf-body');
        body.appendChild(box);
        win.appendChild(body);
        const ctl = h('div', 'wf-ctl');
        STATES.forEach((s, i) => {
          const b = h('button', 'wf-btn' + (i === si ? ' on' : ''), s.w);
          b.type = 'button';
          b.addEventListener('click', () => { si = i; [...ctl.children].forEach((x, j) => x.classList.toggle('on', j === i)); draw(); });
          ctl.appendChild(b);
        });
        stage.append(win, ctl);
        hint(stage, 'Bấm để đổi trạng thái dòng "Claude 1". Hover hoặc Tab vào dòng để thấy 2 action.');
      }
    ]);
  };

  const tone = v => v >= 90 ? 'crit' : v >= 70 ? 'warn' : '';
  const toneWord = v => v >= 90 ? 'Sắp chạm limit' : v >= 70 ? 'Gần hết' : 'Ổn';
  WF.quota = host => {
    let week = 91;
    compare(host, [
      stage => {
        const win = h('div', 'wf-win');
        const draw = () => {
          win.innerHTML = `<div class="wf-body" style="min-height:90px"></div><div class="wf-foot"><b>Claude Code</b><span>5h<span class="qb"><i style="width:42%"></i></span>42% reset 09:22 PM</span><span>Week<span class="qb"><i class="${week >= 90 ? 'crit' : ''}" style="width:${week}%"></i></span><span class="${week >= 90 ? 'crit-t' : ''}">${week}%</span> reset Sun 07:47 PM</span><b>Codex</b><span>5h<span class="qb"><i style="width:7%"></i></span>7% reset 11:07 PM</span></div>`;
        };
        draw();
        stage.appendChild(win);
        hint(stage, 'Mọi cửa sổ cùng cỡ chữ 11 đến 12px; giờ reset tuyệt đối dạng 12h.');
      },
      stage => {
        const win = h('div', 'wf-win');
        const wins = () => [
          { a: 'Claude', w: 'tuần', v: week, r: 'sau 2 ngày 3 giờ', at: 'CN 19:47' },
          { a: 'Claude', w: '5h', v: 42, r: 'sau 1 giờ 12 phút', at: '21:22' },
          { a: 'Codex', w: '5h', v: 7, r: 'sau 3 giờ 40 phút', at: '23:07' }
        ].sort((x, y) => y.v - x.v);
        let open = false;
        const draw = () => {
          const ws = wins();
          const big = ws.filter(x => x.v >= 70);
          const small = ws.filter(x => x.v < 70);
          win.innerHTML = `<div class="wf-body" style="min-height:90px"></div><div class="wf-q">
            ${big.map(x => `<button type="button" class="wf-qbig ${tone(x.v)}" data-open><span>${x.a} ${x.w} <b>${x.v}%</b>, còn ${100 - x.v}%, reset ${x.r}</span><span class="st ${x.v >= 90 ? 'err' : 'wait'}">${toneWord(x.v)}</span></button>`).join('')}
            ${small.map(x => `<button type="button" class="wf-qsm" data-open>${x.a} ${x.w} ${x.v}%</button>`).join('')}
            ${open ? `<div class="wf-qpop" role="dialog" aria-label="Quota"><b>Quota theo cửa sổ</b>${ws.map(x => `<div class="wf-qrow"><span>${x.a} ${x.w}</span><span class="wf-qtrack"><i class="${tone(x.v)}" style="width:${x.v}%"></i><em style="left:70%"></em><em style="left:90%"></em></span><b style="text-align:right">${x.v}%</b><small>${toneWord(x.v)}, reset ${x.r} (${x.at})</small></div>`).join('')}</div>` : ''}
          </div>`;
          win.querySelectorAll('[data-open]').forEach(b => b.addEventListener('click', () => { open = !open; draw(); }));
        };
        draw();
        const r = h('label', 'wf-range', `Claude tuần đã dùng <input type="range" min="0" max="100" value="${week}"> <b class="v">${week}%</b>`);
        r.querySelector('input').addEventListener('input', e => { week = +e.target.value; r.querySelector('.v').textContent = week + '%'; draw(); });
        stage.append(win, r);
        hint(stage, 'Kéo thanh: dưới 70% gom vào chip nhỏ, từ 70% mở rộng kèm thời gian còn lại. Bấm chip để mở chi tiết.');
      }
    ]);
  };

  const FOOT = [['Trạng thái', 'M2 7h2l2-4 2 8 2-4h2'], ['Inbox', 'M2 8l1.6-5h6.8L12 8v3H2z'], ['Usage', 'M3 11V7M6 11V4M9 11V6M12 11V2'], ['History', 'M7 3.5V7l2 1.3M2.5 7a4.5 4.5 0 1 0 1.3-3.2'], ['Settings', 'M2 4h10M2 10h10M5 2v4M9 8v4'], ['Thêm', 'M3 7h.01M7 7h.01M11 7h.01']];
  const ic = d => `<svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="${d}" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  const ETABS = [['Files', 'M2 3h4l1 1.5h5V11H2z'], ['Changes', 'M4 2v7M4 9a2 2 0 1 0 .01 0M10 5a2 2 0 1 0 .01 0M10 7c0 2-6 1-6 3'], ['Search', 'M6 2.5a3.5 3.5 0 1 0 .01 0M8.6 8.6L12 12'], ['TODO', 'M2 4h2M2 8h2M6 4h6M6 8h6'], ['Checks', 'M2.5 7.2l2.6 2.6L11.5 3.5']];

  WF.labels = host => {
    compare(host, [
      stage => {
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-resz"><div class="wf-left"><div class="wf-rrow focus"><span class="wf-glyph run"></span><span class="t">dauph</span><span class="tm">3s</span></div>
          <div class="wf-rfoot" style="justify-content:space-between">${FOOT.map(([n, d]) => `<button type="button" class="wf-rb" aria-label="${n}">${ic(d)}<span class="wf-tip slow">${n}</span></button>`).join('')}</div></div>
          <div class="wf-exp" style="width:46%"><div class="wf-etabs">${ETABS.map(([n, d], i) => `<button type="button" class="wf-etab${i ? '' : ' on'}" aria-label="${n}">${ic(d)}</button>`).join('')}</div><div class="wf-root">CCM-E2E-V103-71-PROJ-28736</div><div class="wf-file">level-1</div><div class="wf-file">short.ts</div></div></div>
          <div class="wf-toastx one">Not enough room to pin: the canvas needs at least 640px next to Explorer</div>`;
        stage.appendChild(win);
        hint(stage, 'Hover chân rail: tên hiện sau khoảng 1 giây. Tab Explorer không có chữ. Toast bị cắt 1 dòng.');
      },
      stage => {
        const win = h('div', 'wf-win');
        let w = 380;
        win.innerHTML = `<div class="wf-resz"><div class="wf-left"><div class="wf-rrow focus"><span class="wf-glyph run"></span><span class="t">Claude 1</span><span class="tm">chạy 3s</span></div>
          <div class="wf-rfoot">${FOOT.filter(f => ['Inbox', 'Usage', 'Settings'].includes(f[0])).map(([n, d]) => `<button type="button" class="wf-rb">${ic(d)}${n}</button>`).join('')}<button type="button" class="wf-rb" aria-label="Thêm: Trạng thái, History">${ic(FOOT[5][1])}<span class="wf-tip">Trạng thái, History</span></button></div></div>
          <div class="wf-handle" role="separator" aria-orientation="vertical" aria-label="Kéo đổi độ rộng Explorer" tabindex="0"></div>
          <div class="wf-exp"><div class="wf-etabs"></div><div class="wf-root">ccm-e2e...28736</div><div class="wf-file">level-1</div><div class="wf-file">short.ts</div><div class="wf-wd"></div></div></div>
          <div class="wf-toastx">Không đủ chỗ để ghim Explorer: vùng pane cần ít nhất 640px. Kéo rộng cửa sổ hoặc bỏ ghim.</div>`;
        const exp = win.querySelector('.wf-exp');
        const tabs = win.querySelector('.wf-etabs');
        const wd = win.querySelector('.wf-wd');
        const resz = win.querySelector('.wf-resz');
        let on = 0;
        const draw = () => {
          const max = Math.max(140, resz.clientWidth - 160);
          w = Math.min(Math.max(w, 140), max);
          exp.style.width = w + 'px';
          const lab = w >= 360;
          tabs.innerHTML = ETABS.map(([n, d], i) => `<button type="button" class="wf-etab${i === on ? ' on' : ''}" aria-label="${n}">${ic(d)}${lab ? `<span class="lb">${n}</span>` : `<span class="wf-tip">${n}</span>`}</button>`).join('');
          tabs.querySelectorAll('.wf-etab').forEach((b, i) => b.addEventListener('click', () => { on = i; draw(); }));
          wd.textContent = `${Math.round(w)}px, ${lab ? 'có chữ' : 'chỉ icon + tooltip'}`;
        };
        const hd = win.querySelector('.wf-handle');
        let x0 = 0, w0 = 0;
        hd.addEventListener('pointerdown', e => { x0 = e.clientX; w0 = w; hd.setPointerCapture(e.pointerId); hd.classList.add('drag'); });
        hd.addEventListener('pointermove', e => { if (!hd.classList.contains('drag')) return; w = w0 - (e.clientX - x0); draw(); });
        hd.addEventListener('pointerup', () => hd.classList.remove('drag'));
        hd.addEventListener('keydown', e => { if (e.key === 'ArrowLeft') { w += 20; draw(); } if (e.key === 'ArrowRight') { w -= 20; draw(); } });
        stage.appendChild(win);
        requestAnimationFrame(draw);
        addEventListener('resize', draw);
        hint(stage, 'Kéo thanh dọc (hoặc phím mũi tên) để đổi độ rộng Explorer: từ 360px tab có chữ. Chân rail có chữ cho 3 nút hay dùng.');
      }
    ]);
  };

  WF.phoneCards = host => {
    compare(host, [
      stage => {
        const p = h('div', 'wf-phone');
        p.innerHTML = `<div class="wf-ph-h">Workspace <span class="pill p-c6">Relay</span></div><div class="wf-pgrid two">
          <div class="wf-pg">api</div><div class="wf-pcard"><div class="nm"><span>claude patched</span></div><span class="st wait" style="align-self:flex-start">Chờ bạn</span><div class="wf-prev one">Chạy lệnh: npm test? [y/n]</div></div>
          <div class="wf-pg">web</div><div class="wf-pcard cx"><div class="nm"><span>web codex</span></div><span class="st run" style="align-self:flex-start">Đang chạy</span><div class="wf-prev one">&gt; Edit src/app.ts</div></div></div>`;
        stage.appendChild(p);
        hint(stage, 'Lưới 2 cột cố định: mỗi nhóm 1 pane thì nửa phải trống. Muốn trả lời phải mở pane full.');
      },
      stage => {
        const p = h('div', 'wf-phone');
        let answered = null, timer = null;
        const draw = () => {
          const waiting = !answered || answered === 'undo';
          p.innerHTML = `<div class="wf-ph-h">Workspace <span class="pill p-c6">Relay</span></div><div class="wf-pgrid">
            <div class="wf-pg">api</div>
            <div class="wf-pcard"><div class="nm"><span>claude patched</span><span class="tm" style="font-size:12px;color:var(--ink-2);font-weight:500">${waiting ? 'chờ 4 phút' : 'chạy 1s'}</span></div>
              ${waiting ? `<span class="st wait" style="align-self:flex-start">Chờ bạn</span><div class="wf-prev"><div>PASS src/lib/a.test.ts</div><div>PASS src/lib/b.test.ts</div><div>Bash wants to run: npm test</div></div><div class="wf-q2">Chạy lệnh npm test?</div><div class="wf-ans"><button type="button" class="pri" data-a="1">Có</button><button type="button" data-a="3">Không</button><button type="button" data-a="open">Mở pane</button></div>`
              : `<span class="st run" style="align-self:flex-start">Đang chạy</span><div class="wf-prev"><div>Bash(npm test)</div><div>  running 42 tests...</div><div>&nbsp;</div></div>`}
            </div>
            <div class="wf-pg">web</div>
            <div class="wf-pcard cx"><div class="nm"><span>web codex</span></div><span class="st run" style="align-self:flex-start">Đang chạy</span><div class="wf-prev"><div>&gt; Read src/app.ts</div><div>&gt; Edit src/app.ts</div><div>  +12 -3</div></div></div>
          </div>`;
          if (answered && answered !== 'undo') {
            const t = h('div', 'wf-ptoast', `<span>Đã gửi "${answered === '1' ? 'Có' : 'Không'}" tới claude patched</span><button type="button">Hoàn tác</button><i class="bar"></i>`);
            t.querySelector('button').addEventListener('click', () => { clearTimeout(timer); answered = 'undo'; draw(); });
            p.appendChild(t);
            timer = setTimeout(() => { const x = p.querySelector('.wf-ptoast'); if (x) x.remove(); }, 3000);
          }
          p.querySelectorAll('[data-a]').forEach(b => b.addEventListener('click', () => {
            if (b.dataset.a === 'open') { b.textContent = 'Đang mở...'; return; }
            answered = b.dataset.a; draw();
          }));
        };
        draw();
        stage.appendChild(p);
        hint(stage, 'Bấm Có hoặc Không: thẻ chuyển Đang chạy, toast có Hoàn tác 3 giây. Phím chỉ gửi sau khi chủ nhân bấm.');
      }
    ]);
  };

  const LOG = Array.from({ length: 28 }, (_, i) => `line ${String(874 + i).padStart(5, '0')} abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij`);
  const KEYS = ['Esc', 'Tab', 'Ctrl', 'Alt', '⇧Tab', '^C', '↑', '↓', '←', '→', 'Home', 'End'];
  WF.phoneTerm = host => {
    compare(host, [
      stage => {
        const p = h('div', 'wf-phone');
        p.innerHTML = `<div class="wf-tq">Chạy lệnh: npm test? [y/n]<div class="opts"><span>Có</span><span>Có, luôn luôn</span><span>Không</span><span>Dừng</span><span>Esc</span></div></div>
          <div class="wf-term2">${LOG.map(l => `<div>${l}</div>`).join('')}</div>
          <div class="wf-keys"><div class="wf-keyrow">${KEYS.map(k => `<span>${k}</span>`).join('')}</div></div>`;
        stage.appendChild(p);
        hint(stage, 'Chữ thu nhỏ để giữ số cột desktop, dòng vẫn bị cắt. Thẻ câu hỏi luôn mở. Dải phím không báo còn phím bên phải.');
      },
      stage => {
        const p = h('div', 'wf-phone');
        let read = true;
        p.innerHTML = `<div class="wf-ph-h" style="margin-bottom:6px"><span>Claude, api</span><span class="wf-cmp sm" role="group" aria-label="Chế độ terminal"><button type="button" data-m="d">Desktop</button><button type="button" data-m="r">Đọc</button></span></div>
          <button type="button" class="wf-tq" aria-expanded="true">Chạy lệnh: npm test? [y/n]<div class="opts"><span>Có</span><span>Có, luôn luôn</span><span>Không</span><span>Dừng</span></div></button>
          <div class="wf-term2">${LOG.map(l => `<div>${l}</div>`).join('')}</div>
          <div class="wf-keys fade"><div class="wf-keyrow">${KEYS.map(k => `<span>${k}</span>`).join('')}</div></div><div class="wf-dots"><i class="on"></i><i></i></div>`;
        const term = p.querySelector('.wf-term2');
        const tq = p.querySelector('.wf-tq');
        const row = p.querySelector('.wf-keyrow');
        const keys = p.querySelector('.wf-keys');
        const dots = p.querySelectorAll('.wf-dots i');
        const mode = () => { term.classList.toggle('read', read); p.querySelectorAll('[data-m]').forEach(b => b.classList.toggle('on', (b.dataset.m === 'r') === read)); };
        p.querySelectorAll('[data-m]').forEach(b => b.addEventListener('click', () => { read = b.dataset.m === 'r'; mode(); }));
        term.addEventListener('scroll', () => { if (term.scrollTop > 20 && !tq.classList.contains('mini')) { tq.classList.add('mini'); tq.setAttribute('aria-expanded', 'false'); } });
        tq.addEventListener('click', () => { const m = tq.classList.toggle('mini'); tq.setAttribute('aria-expanded', String(!m)); if (!m) term.scrollTop = 0; });
        row.addEventListener('scroll', () => {
          const end = row.scrollLeft + row.clientWidth >= row.scrollWidth - 4;
          keys.classList.toggle('fade', !end);
          dots[0].classList.toggle('on', !end); dots[1].classList.toggle('on', end);
        });
        mode();
        stage.appendChild(p);
        hint(stage, 'Đổi Desktop / Đọc. Cuộn log: thẻ câu hỏi thu thành 1 dòng, chạm để mở lại. Vuốt dải phím: mép mờ và chấm trang báo còn phím.');
      }
    ]);
  };

  let d = { items: [] };
  try { d = await (await fetch('/data/ui.json', { cache: 'no-cache' })).json(); } catch (e) {}
  if (d.lede) $('lede').textContent = d.lede;
  if (d.updated) { const [y, m, dd] = d.updated.split('-'); $('updated').textContent = `${dd}/${m}/${y}`; }
  const list = v => Array.isArray(v) ? `<ul>${v.map(x => `<li>${esc(x)}</li>`).join('')}</ul>` : esc(v);
  const items = d.items || [];
  const box = $('props');
  items.forEach(p => {
    const [ac, at] = AREA[p.area] || ['p-off', p.area];
    const el = h('article', 'prop');
    el.id = p.anchor;
    el.innerHTML = `<div class="prop-h"><div class="prop-tags"><span class="id">${esc(p.id)}</span><span class="pill ${ac}">${esc(at)}</span><span class="pill p-off">Công sức ${esc(p.effort)}</span></div><h3>${esc(p.title)}</h3></div>
      <div class="prop-body"><div class="prop-text"><dl>
        <dt>Vấn đề hiện tại</dt><dd>${list(p.problem)}</dd>
        <dt>Đề xuất</dt><dd>${list(p.proposal)}</dd>
        <dt>Lợi ích</dt><dd>${list(p.impact)}</dd>
        <dt>Rủi ro</dt><dd class="risk">${list(p.risk)}</dd>
        <dt>Bằng chứng</dt><dd class="ev">${(p.files || []).map(f => `<code>${esc(f)}</code>`).join(' ')}</dd>
      </dl></div>
      <div class="prop-wf"><div class="wf-label"><span>Wireframe</span><span>${esc(p.wfHint || '')}</span></div><div class="wf"></div></div></div>`;
    box.appendChild(el);
    const fn = WF[p.wireframe];
    if (fn) fn(el.querySelector('.wf'));
  });
  $('order').innerHTML = items.slice().sort((a, b) => a.rank - b.rank).map(p => {
    const [ac, at] = AREA[p.area] || ['p-off', p.area];
    const r = String(p.risk).split('.')[0];
    return `<tr><td class="nw">${p.rank}</td><td><a href="#${esc(p.anchor)}">${esc(p.id)}</a> ${esc(p.title)}</td><td><span class="pill ${ac}">${esc(at)}</span></td><td class="nw">${esc(p.effort)}</td><td>${esc(r)}</td></tr>`;
  }).join('');
  window.site.tables();
})();
