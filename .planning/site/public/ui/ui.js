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

  const FILE = 'M3 1.5h5l3 3V12.5H3z M8 1.5v3h3';
  const DIFF = 'M4 2v5M1.5 4.5h5M8 10.5h4.5M4 9.5v3M10 2v5';
  const CHG = 'M4 2v7M4 9a2 2 0 1 0 .01 0M10 5a2 2 0 1 0 .01 0M10 7c0 2-6 1-6 3';
  const CHEV = '<svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true"><path d="M4 2.5L7.5 6 4 9.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  WF.railTools = host => {
    let dirty = true, open = false;
    compare(host, [
      stage => {
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-body"><div class="wf-railbox"><div class="wf-grp"><span>ccm-e2e-explorer</span><span>5</span></div>
          ${[['Claude 1', 'wait', 'Chờ bạn', 'chờ 2 phút'], ['Shell', 'idle', 'Idle', 'idle 4s'], ['main.ts', '', 'Idle', 'idle 3s'], ['README.md', '', 'Idle', 'idle 3s'], ['Thay đổi (3)', '', 'Rảnh', 'rảnh 1s']].map(([n, k, w, t]) => `<div class="wf-rrow"><span class="wf-glyph ${k}"></span><span class="t">${n}</span><span class="tm">${t}</span><span class="s"><span class="st ${k || 'off'}">${w}</span></span></div>`).join('')}
        </div></div>`;
        stage.appendChild(win);
        hint(stage, 'File, diff và Changes cùng chip Idle / Rảnh như agent. Không thấy main.ts chưa lưu. Project đếm 5.');
      },
      stage => {
        const win = h('div', 'wf-win');
        const box = h('div', 'wf-railbox');
        const draw = () => {
          const tools = [
            { n: 'main.ts', d: FILE, m: dirty ? '<span class="wf-dirty"></span>Chưa lưu' : 'Đã lưu', cls: dirty ? 'warn' : '' },
            { n: 'README.md', d: DIFF, m: '<b class="add">+1</b> <b class="del">-1</b>', cls: '' },
            { n: 'Thay đổi', d: CHG, m: '3 file mới', cls: '' }
          ];
          box.innerHTML = `<div class="wf-grp"><span>ccm-e2e-explorer</span><span>2</span></div>
            <div class="wf-rrow"><span class="wf-glyph wait"></span><span class="t">Claude 1</span><span class="tm">chờ 2 phút</span><span class="s"><span class="st wait">Chờ bạn</span><span class="x">Bash: npm test?</span></span></div>
            <div class="wf-rrow"><span class="wf-glyph idle"></span><span class="t">Shell</span><span class="tm">4 giây</span><span class="s"><span class="st idle">Rảnh</span></span></div>
            <button type="button" class="wf-tools" aria-expanded="${open}"><span class="cv${open ? ' on' : ''}">${CHEV}</span>Công cụ <span class="n">3</span>${dirty ? '<span class="wf-tdirty"><span class="wf-dirty"></span>1 chưa lưu</span>' : ''}</button>
            ${open ? `<div class="wf-tlist">${tools.map(x => `<div class="wf-trow">${ic(x.d)}<span class="t">${x.n}</span><span class="m ${x.cls}">${x.m}</span></div>`).join('')}</div>` : ''}`;
          box.querySelector('.wf-tools').addEventListener('click', () => { open = !open; draw(); });
        };
        draw();
        const body = h('div', 'wf-body');
        body.appendChild(box);
        win.appendChild(body);
        const ctl = h('div', 'wf-ctl');
        const b = h('button', 'wf-btn on', 'main.ts chưa lưu');
        b.type = 'button';
        b.addEventListener('click', () => { dirty = !dirty; b.classList.toggle('on', dirty); draw(); });
        ctl.appendChild(b);
        stage.append(win, ctl);
        hint(stage, 'Bấm dòng Công cụ để mở. Bấm nút dưới để đổi main.ts lưu / chưa lưu: dòng Công cụ thu gọn vẫn báo.');
      }
    ]);
  };

  const INBOX = [
    { g: 'Chờ trả lời menu', n: 'db shell', s: 'Đang chờ chọn trong menu', k: 'wait' },
    { g: 'Xin quyền', n: 'claude patched', s: 'Xin quyền chạy npm test', k: 'wait' },
    { g: 'Đụng file', n: 'web codex', s: 'Cùng sửa src/app.ts với api claude', k: 'wait' },
    { g: 'Xong, chưa xem', n: 'npm run dev', s: 'Xong, chưa xem', k: 'idle' }
  ];
  const BEHIND = ['docs', 'Claude  Chờ bạn  npm test', 'web            322n', 'Codex  Đang chạy  Edit src/app.ts', 'api            322n', 'Claude  Rảnh', 'infra', 'Terminal  Rảnh'];
  WF.inboxSheet = host => {
    compare(host, [
      stage => {
        const p = h('div', 'wf-phone wf-ibx');
        p.innerHTML = `<div class="wf-behind">${BEHIND.map(x => `<div>${x}</div>`).join('')}</div>
          <div class="wf-sheet glass"><i class="grab"></i><b class="ttl">Inbox</b>${INBOX.map(x => `<div class="wf-ig">${x.g} 1</div><div class="wf-irow"><span class="nm">${x.n}</span><span class="sb">${x.s}</span><span class="ag">&lt;1p</span></div>`).join('')}</div>`;
        stage.appendChild(p);
        hint(stage, 'Nền sheet trong: chữ list phía sau hiện xuyên qua tên hàng. Hàng chỉ mở pane, không có Đã xem / Hoãn. "<1p" nhìn như mũi tên.');
      },
      stage => {
        const p = h('div', 'wf-phone wf-ibx');
        let rows = INBOX.map((x, i) => ({ ...x, i, gone: null })), timer = null, menu = -1;
        const ages = ['vừa xong', '4 phút', '12 phút', '1 giờ'];
        const draw = () => {
          const live = rows.filter(r => !r.gone);
          p.innerHTML = `<div class="wf-behind">${BEHIND.map(x => `<div>${x}</div>`).join('')}</div><div class="wf-scrim"></div>
            <div class="wf-sheet"><i class="grab"></i><b class="ttl">Inbox <span class="n">${live.length}</span></b>
            ${live.length ? live.map(x => `<div class="wf-ig">${x.g}</div><div class="wf-swipe" data-i="${x.i}"><span class="ul l">Hoãn 15 phút</span><span class="ul r">Đã xem</span><div class="wf-irow mv" tabindex="0"><span class="nm">${x.n}</span><span class="sb">${x.s}</span><span class="ag">${ages[x.i]}</span><button type="button" class="more" aria-label="Thêm hành động" aria-expanded="${menu === x.i}">${ic('M3 7h.01M7 7h.01M11 7h.01')}</button></div>${menu === x.i ? `<div class="wf-iact"><button type="button" data-do="seen">Đã xem</button><button type="button" data-do="snooze">Hoãn 15 phút</button></div>` : ''}</div>`).join('') : '<p class="wf-iempty">Inbox trống.</p>'}</div>`;
          const done = rows.filter(r => r.gone).slice(-1)[0];
          if (done && timer) {
            const t = h('div', 'wf-ptoast', `<span>${done.gone === 'seen' ? 'Đã xem' : 'Hoãn 15 phút'}: ${done.n}</span><button type="button">Hoàn tác</button><i class="bar"></i>`);
            t.querySelector('button').addEventListener('click', () => { clearTimeout(timer); timer = null; done.gone = null; draw(); });
            p.appendChild(t);
          }
          p.querySelectorAll('.wf-swipe').forEach(sw => {
            const i = +sw.dataset.i, row = sw.querySelector('.mv');
            let x0 = null, dx = 0;
            row.addEventListener('pointerdown', e => { if (e.target.closest('button')) return; x0 = e.clientX; dx = 0; row.setPointerCapture(e.pointerId); row.style.transition = 'none'; });
            row.addEventListener('pointermove', e => { if (x0 == null) return; dx = Math.max(-120, Math.min(120, e.clientX - x0)); row.style.transform = `translateX(${dx}px)`; sw.classList.toggle('to-l', dx > 0); sw.classList.toggle('to-r', dx < 0); });
            const end = () => { if (x0 == null) return; x0 = null; row.style.transition = ''; if (Math.abs(dx) > 70) act(i, dx < 0 ? 'seen' : 'snooze'); else row.style.transform = ''; };
            row.addEventListener('pointerup', end);
            row.addEventListener('pointercancel', end);
            row.addEventListener('keydown', e => { if (e.key === 'e' || e.key === 'E') act(i, 'seen'); if (e.key === 's' || e.key === 'S') act(i, 'snooze'); });
            sw.querySelector('.more').addEventListener('click', () => { menu = menu === i ? -1 : i; draw(); });
            sw.querySelectorAll('[data-do]').forEach(b => b.addEventListener('click', () => act(i, b.dataset.do)));
          });
        };
        const act = (i, how) => {
          clearTimeout(timer);
          rows.forEach(r => { if (r.gone && r.i !== i) r.done = true; });
          rows = rows.filter(r => !r.done);
          const r = rows.find(x => x.i === i);
          r.gone = how; menu = -1;
          timer = setTimeout(() => { timer = null; const t = p.querySelector('.wf-ptoast'); if (t) t.remove(); }, 3000);
          draw();
        };
        draw();
        const ctl = h('div', 'wf-ctl');
        const reset = h('button', 'wf-btn', 'Đặt lại');
        reset.type = 'button';
        reset.addEventListener('click', () => { clearTimeout(timer); timer = null; menu = -1; rows = INBOX.map((x, i) => ({ ...x, i, gone: null })); draw(); });
        ctl.appendChild(reset);
        stage.append(p, ctl);
        hint(stage, 'Vuốt hàng sang trái: Đã xem; sang phải: Hoãn 15 phút. Nút ... cho người không vuốt, phím E / S khi focus hàng. Toast có Hoàn tác 3 giây.');
      }
    ]);
  };

  const FIT = {
    vi: { seg: ['Workspace', 'Giám sát'], grp: ['Project', 'Trạng thái'], search: 'Tìm...', foot: ['Thống kê', 'Lịch sử', 'Cài đặt'], wait: 'Chờ bạn', tool: 'Bash: npm test?', time: 'chờ 2 phút', group: 'Nhóm' },
    en: { seg: ['Workspace', 'Monitor'], grp: ['Project', 'Status'], search: 'Search...', foot: ['Usage', 'History', 'Settings'], wait: 'Waiting', tool: 'Bash: npm test?', time: 'waiting 2m', group: 'Group' }
  };
  const FOOTD = [FOOT[2][1], FOOT[3][1], FOOT[4][1]];
  const SEGD = ['M2 3h4v4H2zM8 3h4v4H8zM2 9h4v3H2zM8 9h4v3H8z', 'M1.5 7h2.5l2-4 2 8 2-4h2.5'];
  WF.railFit = host => {
    compare(host, [
      stage => {
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-body"><div class="wf-fit" style="width:min(264px,100%)">
          <div class="wf-fseg"><span class="on">Worksp...</span><span>Giám sát</span></div>
          <div class="wf-fsearch">${ic(ETABS[2][1])}<span>Tìm session, projec...</span><kbd>Ctrl+K</kbd></div>
          <div class="wf-fgrp"><span>Nhóm</span><div class="wf-fseg sm"><span class="on">Project</span><span>Trạng th...</span></div></div>
          <div class="wf-grp"><span>ccm-e2e-turn... ma...</span><span>3</span></div>
          <div class="wf-rrow"><span class="wf-glyph wait"></span><span class="t">api: migrate schema</span><span class="s"><span class="st wait">Chờ bạn</span><span class="x">C...</span><span class="tm">chờ 2 phút</span></span></div>
          <div class="wf-rfoot"><span class="wf-rb">${ic(FOOT[0][1])}</span>${FOOTD.map((dd, i) => `<span class="wf-rb">${ic(dd)}${['Thố...', 'Li...', 'Cà...'][i]}</span>`).join('')}<span class="wf-rb">${ic(FOOT[5][1])}</span></div>
        </div></div>`;
        stage.appendChild(win);
        hint(stage, 'Ảnh desktop-inbox-light: 5 chỗ chữ bị cắt giữa từ, tool trên dòng 2 còn 1 ký tự.');
      },
      stage => {
        const win = h('div', 'wf-win');
        const body = h('div', 'wf-body');
        const rail = h('div', 'wf-fit');
        body.appendChild(rail);
        win.appendChild(body);
        let w = 264, lang = 'vi';
        const draw = () => {
          const L = FIT[lang];
          const paint = (foot, segLab) => {
            const footLab = foot !== 'icons';
            rail.style.width = `min(${w}px,100%)`;
            rail.innerHTML = `<div class="wf-fseg" role="radiogroup" aria-label="Chế độ">${L.seg.map((s, i) => `<span class="${i ? '' : 'on'}" ${segLab ? '' : `aria-label="${s}"`}>${ic(SEGD[i])}${segLab ? `<em class="lb">${s}</em>` : ''}</span>`).join('')}</div>
              <div class="wf-fsearch">${ic(ETABS[2][1])}<span>${L.search}</span><kbd>Ctrl+K</kbd></div>
              <div class="wf-fgrp"><span>${L.group}</span><div class="wf-fseg sm">${L.grp.map((s, i) => `<span class="${i ? '' : 'on'}"><em class="lb">${s}</em></span>`).join('')}</div></div>
              <div class="wf-grp"><span class="cut">ccm-e2e...261OyA</span><span>3</span></div>
              <div class="wf-rrow"><span class="wf-glyph wait"></span><span class="t">api: migrate schema</span><span class="tm">${L.time}</span><span class="s"><span class="st wait">${L.wait}</span><span class="x">${L.tool}</span></span></div>
              <div class="wf-rfoot ${foot}"><button type="button" class="wf-rb" aria-label="${lang === 'vi' ? 'Trạng thái' : 'Status'}">${ic(FOOT[0][1])}<span class="wf-tip">${lang === 'vi' ? 'Trạng thái' : 'Status'}</span></button>${FOOTD.map((dd, i) => `<button type="button" class="wf-rb" ${footLab ? '' : `aria-label="${L.foot[i]}"`}>${ic(dd)}${footLab ? `<em class="lb">${L.foot[i]}</em>` : `<span class="wf-tip">${L.foot[i]}</span>`}</button>`).join('')}<button type="button" class="wf-rb" aria-label="Thêm">${ic(FOOT[5][1])}<span class="wf-tip">Thêm</span></button></div>`;
          };
          const over = sel => [...rail.querySelectorAll(sel + ' .lb')].some(e => e.scrollWidth > e.clientWidth + 1);
          paint('row', true);
          const segOk = !over('.wf-fseg:first-child');
          let foot = 'row';
          if (over('.wf-rfoot')) { paint('stack', segOk); foot = over('.wf-rfoot') ? 'icons' : 'stack'; }
          paint(foot, segOk);
          out.textContent = `Rail ${Math.min(w, Math.round(body.clientWidth) || w)}px: chân rail ${{ row: 'chữ cạnh icon', stack: 'chữ dưới icon', icons: 'chỉ icon + tooltip' }[foot]}, segment ${segOk ? 'có chữ' : 'chỉ icon'}`;
        };
        const r = h('label', 'wf-range', `Độ rộng rail <input type="range" min="200" max="340" step="4" value="${w}">`);
        const out = h('p', 'wf-wd');
        r.querySelector('input').addEventListener('input', e => { w = +e.target.value; draw(); });
        const ctl = h('div', 'wf-ctl');
        ['vi', 'en'].forEach(k => {
          const b = h('button', 'wf-btn' + (k === lang ? ' on' : ''), k === 'vi' ? 'Tiếng Việt' : 'English');
          b.type = 'button';
          b.addEventListener('click', () => { lang = k; [...ctl.children].forEach(x => x.classList.toggle('on', x === b)); draw(); });
          ctl.appendChild(b);
        });
        stage.append(win, r, ctl, out);
        requestAnimationFrame(draw);
        addEventListener('resize', draw);
        hint(stage, 'Kéo độ rộng rail và đổi ngôn ngữ: nhãn chân rail cạnh icon khi đủ chỗ, xuống dưới icon khi hẹp, không bao giờ cắt giữa từ. Thời gian lên dòng 1 để tool đủ chỗ.');
      }
    ]);
  };

  const MON = [
    { p: 'docs', b: 'docs', a: 'Claude', k: 'wait', w: 'Chờ bạn', tool: 'npm test', t: '4 phút', old: '' },
    { p: 'web', b: 'main', a: 'Codex', k: 'run', w: 'Đang chạy', tool: 'Edit src/app.ts', t: '38 giây', old: '322n' },
    { p: 'api', b: 'feat/api', a: 'Claude', k: 'idle', w: 'Rảnh', tool: '', t: 'từ 03/01', old: '322n' },
    { p: 'infra', b: 'infra', a: 'Terminal', k: 'idle', w: 'Rảnh', tool: '', t: '2 giờ', old: '' }
  ];
  const AG = { Claude: 'ag-c', Codex: 'ag-x', Terminal: 'ag-t' };
  WF.phoneMonitor = host => {
    compare(host, [
      stage => {
        const p = h('div', 'wf-phone');
        p.innerHTML = `<div class="wf-ph-h">Orrery Desk <span class="pill p-c6">Relay</span></div><div class="wf-mfil"><span class="on">Tất cả 4</span><span>Đang chạy 1</span><span>Chờ bạn 1</span></div>
          ${MON.map(m => `<div class="wf-mcard"><div class="wf-mhead"><span class="lt">${m.p[0].toUpperCase()}</span><b>${m.p}</b><span class="br">${m.b}</span><span>1</span></div><div class="wf-mrow"><span class="wf-glyph ${m.k}"></span><span class="nm">${m.p}</span><span class="tm">${m.old}</span><span class="ms"><span class="${AG[m.a]}">${m.a}</span> <span class="tx-${m.k}">${m.w}</span> <code>${m.tool}</code></span></div></div>`).join('')}`;
        stage.appendChild(p);
        hint(stage, 'Mỗi project 1 thẻ có header + 1 dòng cùng tên. Thời gian "322n" không rõ đơn vị. Màn iPhone chỉ đủ 4 session.');
      },
      stage => {
        const p = h('div', 'wf-phone');
        let extra = false, opened = false;
        const draw = () => {
          const rows = MON.slice();
          const order = { wait: 0, err: 0, run: 1, idle: 2 };
          rows.sort((a, b) => order[a.k] - order[b.k]);
          const row = (m, inCard) => `<div class="wf-mrow${inCard ? '' : ' solo'}"><span class="wf-glyph ${m.k}"></span><span class="nm">${inCard ? m.n || m.a : m.p}</span><span class="tm">${m.t}</span><span class="ms"><span class="st ${m.k}">${m.w}</span>${m.tool ? `<code>${m.tool}</code>` : ''}<span class="${AG[m.a]}">${m.a}</span>${inCard ? '' : `<span class="br">${m.b}</span>`}</span>${m.k === 'wait' ? `<button type="button" class="wf-mans">${opened ? 'Đang mở thẻ trả lời...' : 'Trả lời'}</button>` : ''}</div>`;
          p.innerHTML = `<div class="wf-ph-h">Orrery Desk <span class="pill p-c6">Relay</span></div><div class="wf-mfil fade"><span class="on">Tất cả ${extra ? 5 : 4}</span><span>Chờ bạn 1</span><span>Đang chạy ${extra ? 2 : 1}</span><span>Rảnh 2</span></div>
            <div class="wf-mlist">${rows.map(m => {
              if (m.p === 'web' && extra) return `<div class="wf-mcard"><div class="wf-mhead"><span class="lt">W</span><b>web</b><span class="br">main</span><span>2</span></div>${row({ ...m, n: 'Codex' }, true)}${row({ ...m, n: 'Claude 2', a: 'Claude', tool: 'Read README.md', t: 'vừa xong' }, true)}</div>`;
              return row(m, false);
            }).join('')}</div>`;
          const ans = p.querySelector('.wf-mans');
          if (ans) ans.addEventListener('click', () => { opened = true; draw(); });
        };
        draw();
        const ctl = h('div', 'wf-ctl');
        const b = h('button', 'wf-btn', 'Thêm 1 session vào web');
        b.type = 'button';
        b.addEventListener('click', () => { extra = !extra; opened = false; b.classList.toggle('on', extra); b.textContent = extra ? 'Bỏ session thứ 2 của web' : 'Thêm 1 session vào web'; draw(); });
        ctl.appendChild(b);
        stage.append(p, ctl);
        hint(stage, 'Project 1 session là 1 dòng; thêm session thứ 2 thì web thành thẻ có header. Chờ bạn nổi lên đầu, nút Trả lời chỉ mở thẻ trả lời, không gửi phím.');
      }
    ]);
  };

  const DLINES = ['  1  line 1', '  2  line 2', '- 3  line 3', '+ 3  line three', '  4  line 4', '  5  line 5', '  ... Show 10 unchanged lines', '- 20 line 20', '  21 line 21'];
  WF.drawer = host => {
    const view = (stage, smart) => {
      const win = h('div', 'wf-win');
      let W = 1360, open = true;
      const RAIL = 264, EXP = 285, MIN = 640;
      win.innerHTML = `<div class="wf-dwin"><div class="wf-drail"><i></i><i></i><i class="on"></i></div><div class="wf-dcanvas"><div class="wf-dpane"><div class="wf-dbar"><span>src/a.ts</span><span class="sp"></span><span>Change 1 of 2</span><span class="seg"><b>Inline</b><span>Split</span></span></div><div class="wf-dcode">${DLINES.map(l => `<div class="${l[0] === '-' ? 'del' : l[0] === '+' ? 'add' : ''}">${l}</div>`).join('')}</div></div></div><div class="wf-dscrim"></div><div class="wf-dexp"><b>Explorer</b><div class="wf-file">src</div><div class="wf-file">a.ts</div><div class="wf-file">crlf.txt</div><div class="wf-file">new.md</div></div></div>`;
      const box = win.querySelector('.wf-dwin');
      const out = h('p', 'wf-wd');
      const draw = () => {
        const canvas = W - RAIL - (open ? EXP : 0);
        const push = smart && open && W - RAIL - EXP >= MIN;
        box.classList.toggle('open', open);
        box.classList.toggle('push', push);
        box.classList.toggle('over', open && !push);
        box.classList.toggle('smart', smart);
        box.style.setProperty('--rw', (RAIL / W * 100) + '%');
        box.style.setProperty('--ew', (EXP / W * 100) + '%');
        out.textContent = !open ? `Cửa sổ ${W}px, Explorer đóng` : push ? `Cửa sổ ${W}px: vùng pane còn ${canvas}px, đủ ${MIN}px nên đẩy pane` : smart ? `Cửa sổ ${W}px: vùng pane chỉ còn ${W - RAIL - EXP}px, phủ + scrim, Esc để đóng` : `Cửa sổ ${W}px: drawer phủ lên pane, scrim trong suốt`;
      };
      const r = h('label', 'wf-range', `Độ rộng cửa sổ <input type="range" min="960" max="1920" step="20" value="${W}">`);
      r.querySelector('input').addEventListener('input', e => { W = +e.target.value; draw(); });
      const ctl = h('div', 'wf-ctl');
      const b = h('button', 'wf-btn on', 'Explorer');
      b.type = 'button';
      b.setAttribute('aria-pressed', 'true');
      b.addEventListener('click', () => { open = !open; b.classList.toggle('on', open); b.setAttribute('aria-pressed', String(open)); draw(); });
      ctl.appendChild(b);
      win.querySelector('.wf-dscrim').addEventListener('click', () => { if (smart) { open = false; b.classList.remove('on'); draw(); } });
      stage.append(win, r, ctl, out);
      draw();
    };
    compare(host, [
      stage => { view(stage, false); hint(stage, 'Ảnh diff-02-viewer: Explorer đè lên phải diff, nút Inline / Split bị che dù cửa sổ 1360px đủ chỗ.'); },
      stage => { view(stage, true); hint(stage, 'Kéo độ rộng cửa sổ: từ khoảng 1190px trở lên Explorer đẩy pane sang, hẹp hơn thì phủ với scrim tối, bấm scrim để đóng.'); }
    ]);
  };

  let d = { items: [] };
  try { d = await (await fetch('/data/ui.json', { cache: 'no-cache' })).json(); } catch (e) {}
  if (d.lede) $('lede').textContent = d.lede;
  if (d.updated) { const [y, m, dd] = d.updated.split('-'); $('updated').textContent = `${dd}/${m}/${y}`; }
  const list = v => Array.isArray(v) ? `<ul>${v.map(x => `<li>${esc(x)}</li>`).join('')}</ul>` : esc(v);
  const all = d.items || [];
  const items = all.filter(p => p.status !== 'done');
  const done = all.filter(p => p.status === 'done');
  const doneBox = $('done');
  if (doneBox) {
    $('done-n').textContent = done.length;
    $('da-lam').hidden = !done.length;
    $('done-det').addEventListener('toggle', () => dispatchEvent(new Event('resize')));
  }
  all.forEach(p => {
    const [ac, at] = AREA[p.area] || ['p-off', p.area];
    const isDone = p.status === 'done';
    const box = isDone && doneBox ? doneBox : $('props');
    const el = h('article', 'prop' + (isDone ? ' is-done' : ''));
    el.id = p.anchor;
    el.innerHTML = `<div class="prop-h"><div class="prop-tags"><span class="id">${esc(p.id)}</span>${isDone ? `<span class="pill p-done"><svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6.2l2.4 2.4L9.5 3.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>Đã làm</span>` : ''}<span class="pill ${ac}">${esc(at)}</span><span class="pill p-off">Công sức ${esc(p.effort)}</span>${p.round ? `<span class="pill p-off">Đợt ${esc(p.round)}</span>` : ''}</div><h3>${esc(p.title)}</h3></div>
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
