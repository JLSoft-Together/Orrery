(() => {
  const WF = window.WF = window.WF || {};
  const h = (tag, cls, html) => { const e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; };
  const ICON = {
    bell: '<svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true"><path d="M3.5 10V6.5a3.5 3.5 0 0 1 7 0V10l1 1h-9z M5.8 12.3a1.3 1.3 0 0 0 2.4 0" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg>',
    inbox: '<svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true"><path d="M2 8.5l1.6-5.5h6.8L12 8.5v3H2z M2 8.5h3l.8 1.5h2.4L9 8.5h3" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>',
    warn: '<svg width="13" height="13" viewBox="0 0 12 12" aria-hidden="true"><path d="M6 1.5l4.6 8.5H1.4z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/><path d="M6 5v2.2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><circle cx="6" cy="8.6" r=".8" fill="currentColor"/></svg>',
    search: '<svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true"><circle cx="6" cy="6" r="4" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M9 9l3 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>',
    check: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6.2l2.3 2.3 4.7-5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    chev: '<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M4.5 2.5L8 6l-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>'
  };

  const compare = (host, views, start = 1) => {
    const bar = h('div', 'wf-cmp');
    bar.setAttribute('role', 'tablist');
    const stage = h('div', 'wf-stage');
    const names = ['Hiện tại', 'Đề xuất'];
    const show = i => {
      [...bar.children].forEach((b, j) => { b.classList.toggle('on', i === j); b.setAttribute('aria-selected', String(i === j)); });
      stage.innerHTML = '';
      views[i](stage);
    };
    names.forEach((n, i) => {
      const b = h('button', '', n);
      b.type = 'button';
      b.setAttribute('role', 'tab');
      b.addEventListener('click', () => show(i));
      bar.appendChild(b);
    });
    host.append(bar, stage);
    show(start);
  };

  const hint = (stage, text) => { const p = h('p', 'wf-hint', text); stage.appendChild(p); return p; };

  WF.inbox = host => {
    const items = [
      { g: 'Chờ trả lời menu', s: 'api-server', t: 'Chọn 1 / 2 / 3', w: '4 phút' },
      { g: 'Xin quyền', s: 'web-ui', t: 'Bash: npm run build', w: '2 phút' },
      { g: 'Câu hỏi', s: 'docs', t: 'Đổi tên file README?', w: '1 phút' },
      { g: 'Lỗi', s: 'core', t: 'cargo test fail', w: '6 phút' },
      { g: 'Xong chưa xem', s: 'mobile', t: 'Đã xong lượt', w: '9 phút' }
    ];
    compare(host, [
      stage => {
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-bar"><span class="wf-tab">api-server</span><span class="wf-tab">web-ui</span><span class="wf-sp"></span>
          <button type="button" class="wf-chip warn" data-p="a">${ICON.bell}Cần bạn 3</button>
          <button type="button" class="wf-chip" data-p="q">Chờ trả lời 1</button></div>
          <div class="wf-body wf-rel"><div class="wf-rail"><div class="wf-row">api-server <span class="wf-badge">${ICON.bell}1</span></div><div class="wf-row">web-ui <span class="wf-badge">${ICON.bell}1</span></div><div class="wf-row">core <span class="wf-dot warn"></span></div></div>
          <div class="wf-pop" hidden></div></div>`;
        stage.appendChild(win);
        const pop = win.querySelector('.wf-pop');
        win.querySelectorAll('[data-p]').forEach(b => b.addEventListener('click', () => {
          const a = b.dataset.p === 'a';
          pop.hidden = !pop.hidden && pop.dataset.k === b.dataset.p ? true : false;
          pop.dataset.k = b.dataset.p;
          pop.innerHTML = `<b>${a ? 'Cần bạn (Ctrl+Shift+A)' : 'Chờ trả lời (Ctrl+Alt+Q)'}</b>` + (a ? items.slice(1, 4) : items.slice(0, 1)).map(i => `<div class="wf-li">${i.s}: ${i.t}</div>`).join('');
        }));
        hint(stage, '2 chip đếm 2 tập khác nhau, rail còn badge riêng. Bấm từng chip để thấy.');
      },
      stage => {
        let list = items.map((x, i) => ({ ...x, i }));
        let sel = 0;
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-bar"><span class="wf-tab">api-server</span><span class="wf-tab">web-ui</span><span class="wf-sp"></span>
          <button type="button" class="wf-chip warn" data-inbox aria-expanded="false">${ICON.inbox}Inbox <b class="n">5</b></button></div>
          <div class="wf-body wf-rel"><div class="wf-rail"><div class="wf-row">api-server <span class="wf-dot warn"></span></div><div class="wf-row">web-ui <span class="wf-dot warn"></span></div><div class="wf-row">core <span class="wf-dot err"></span></div></div>
          <div class="wf-pop wide" hidden tabindex="-1"></div></div>`;
        stage.appendChild(win);
        const chip = win.querySelector('[data-inbox]');
        const pop = win.querySelector('.wf-pop');
        const status = hint(stage, 'Bấm Inbox. Trong danh sách: bấm dòng hoặc Enter để mở pane, S để hoãn, mũi tên lên / xuống để chọn.');
        const render = () => {
          chip.querySelector('.n').textContent = list.length;
          if (!list.length) { pop.innerHTML = '<div class="wf-empty-s">Không còn gì chờ bạn</div>'; return; }
          sel = Math.min(sel, list.length - 1);
          const groups = [...new Set(list.map(x => x.g))];
          pop.innerHTML = groups.map(g => `<div class="wf-gh">${g}</div>` + list.filter(x => x.g === g).map(x => `<div class="wf-li sel-able ${list.indexOf(x) === sel ? 'sel' : ''}" data-i="${x.i}"><span><b>${x.s}</b> ${x.t}</span><span class="wf-time">${x.w}</span><button type="button" class="wf-btn sm" data-snooze="${x.i}">Hoãn</button></div>`).join('')).join('');
        };
        const open = i => { const x = list.find(y => y.i === i); list = list.filter(y => y.i !== i); status.textContent = `Đã mở pane ${x.s} (${x.g}).`; render(); };
        const snooze = i => { const x = list.find(y => y.i === i); list = list.filter(y => y.i !== i); status.textContent = `Đã hoãn ${x.s}, sẽ quay lại sau.`; render(); };
        chip.addEventListener('click', () => { pop.hidden = !pop.hidden; chip.setAttribute('aria-expanded', String(!pop.hidden)); render(); if (!pop.hidden) pop.focus(); });
        pop.addEventListener('click', e => {
          const s = e.target.closest('[data-snooze]');
          if (s) { snooze(+s.dataset.snooze); return; }
          const r = e.target.closest('[data-i]');
          if (r) open(+r.dataset.i);
        });
        pop.addEventListener('keydown', e => {
          if (!list.length) return;
          if (e.key === 'ArrowDown') { sel = (sel + 1) % list.length; render(); e.preventDefault(); }
          if (e.key === 'ArrowUp') { sel = (sel - 1 + list.length) % list.length; render(); e.preventDefault(); }
          if (e.key === 'Enter') open(list[sel].i);
          if (e.key === 's' || e.key === 'S') snooze(list[sel].i);
        });
      }
    ]);
  };

  WF.settings = host => {
    const pagesNow = ['Chung', 'Giao diện', 'Workspace', 'Thông báo', 'Nhắc nhở', 'Terminal', 'Text', 'Tích hợp', 'Điện thoại', 'Telegram', 'GitHub', 'Phím tắt', 'Dữ liệu', 'DeepSeek', 'Giới thiệu'];
    const pagesNew = ['Chung', 'Giao diện', 'Workspace', 'Thông báo + Nhắc nhở', 'Terminal', 'Text', 'Tích hợp', 'Điện thoại', 'Telegram', 'GitHub', 'Phím tắt', 'Dữ liệu'];
    const rows = { 'Workspace': ['File icons', 'Hiệu ứng bụi khi đóng', 'Làm mờ pane không active'], 'Giao diện': ['Bảng màu', 'Sáng / tối', 'Giảm trong suốt'] };
    const build = (stage, fixed) => {
      const pages = fixed ? pagesNew : pagesNow;
      const win = h('div', 'wf-win');
      win.innerHTML = `<div class="wf-bar"><label class="wf-search">${ICON.search}<input type="search" value="" placeholder="Tìm setting, thử: file icons" aria-label="Tìm setting"></label><span class="wf-sp"></span><span class="wf-count">${pages.length} trang</span></div>
        <div class="wf-body wf-set"><ul class="wf-nav">${pages.map((p, i) => `<li class="${i === 1 ? 'on' : ''}" data-page="${p}">${p}</li>`).join('')}</ul><div class="wf-page"></div></div><div class="wf-res" hidden></div>`;
      stage.appendChild(win);
      const st = hint(stage, fixed ? 'Gõ "file icons" rồi bấm kết quả: nhảy sang trang Workspace và sáng dòng đó.' : 'Gõ "file icons" rồi bấm kết quả: ở UI next dòng đó bị ẩn, không có gì xảy ra.');
      const page = win.querySelector('.wf-page');
      const nav = win.querySelector('.wf-nav');
      const res = win.querySelector('.wf-res');
      const go = (p, flash) => {
        nav.querySelectorAll('li').forEach(li => li.classList.toggle('on', li.dataset.page === p));
        const list = rows[p] || ['Các setting của trang'];
        const shown = !fixed && p === 'Giao diện' ? ['Bảng màu', 'Sáng / tối', 'Giảm trong suốt'] : list;
        page.innerHTML = `<div class="wf-ph">${p}</div>` + shown.map(r => `<div class="wf-srow ${r === flash ? 'flash' : ''}"><span>${r}</span><span class="wf-sw on"></span></div>`).join('');
      };
      go('Giao diện');
      nav.addEventListener('click', e => { const li = e.target.closest('li'); if (li) go(li.dataset.page); });
      win.querySelector('input').addEventListener('input', e => {
        const q = e.target.value.trim().toLowerCase();
        if (!q) { res.hidden = true; return; }
        const hit = 'file icons'.includes(q) || q.includes('icon');
        res.hidden = false;
        res.innerHTML = hit ? `<button type="button" class="wf-li sel-able" data-go>File icons <span class="wf-time">${fixed ? 'Workspace' : 'Giao diện'}</span></button>` : '<div class="wf-empty-s">Không có kết quả</div>';
        const b = res.querySelector('[data-go]');
        if (b) b.addEventListener('click', () => {
          res.hidden = true;
          if (fixed) { go('Workspace', 'File icons'); st.textContent = 'Đã mở Workspace, dòng File icons sáng lên.'; }
          else { go('Giao diện'); page.classList.add('shake'); setTimeout(() => page.classList.remove('shake'), 400); st.textContent = 'flash() không tìm thấy [data-setting-row], im lặng return. User không biết vì sao.'; }
        });
      });
    };
    compare(host, [s => build(s, false), s => build(s, true)]);
  };

  WF.classic = host => {
    const steps = [
      { t: 'Bước 1, ngay', e: 'S', d: ['ui-gen.cjs mặc định next', 'Sửa hoặc xoá v51-compact.cjs, v25.cjs', 'E2E chỉ cho classic gắn --ui=classic'] },
      { t: 'Bước 2, bản kế', e: 'S', d: ['Classic đóng băng, chỉ fix vỡ', 'Changelog báo trước'] },
      { t: 'Bước 3, sau 1 đến 2 bản', e: 'L', d: ['Xoá nhánh isNext (tokens.css 214, WorkspaceView 43, CommandPalette 23 chỗ)', 'Gộp 2 Appearance', 'WorkspaceView.vue 2440 xuống khoảng 1200 dòng'] }
    ];
    const done = new Set();
    let cur = 0;
    const wrap = h('div', 'wf-steps');
    const detail = h('div', 'wf-win wf-detail');
    const meter = h('div', 'wf-meter');
    host.append(wrap, detail, meter);
    const draw = () => {
      wrap.innerHTML = steps.map((s, i) => `<button type="button" class="wf-step ${i === cur ? 'on' : ''} ${done.has(i) ? 'done' : ''}" data-i="${i}"><span class="wf-node">${done.has(i) ? ICON.check : i + 1}</span><span class="wf-st">${s.t}</span><span class="wf-time">Công sức ${s.e}</span></button>`).join('');
      const s = steps[cur];
      detail.innerHTML = `<div class="wf-ph">${s.t}</div><ul>${s.d.map(x => `<li>${x}</li>`).join('')}</ul><button type="button" class="wf-btn ${done.has(cur) ? 'on' : ''}" data-done>${done.has(cur) ? 'Đã xong' : 'Đánh dấu xong'}</button>`;
      const branches = Math.round(300 - (done.has(2) ? 300 : 0));
      meter.innerHTML = `<span>Nhánh classic / next còn lại: <b>${branches}</b> chỗ</span><span>E2E chạy next: <b>${done.has(0) ? 'khoảng 90' : '5'}</b> / khoảng 90 file</span>`;
      detail.querySelector('[data-done]').addEventListener('click', () => { done.has(cur) ? done.delete(cur) : done.add(cur); if (done.has(cur) && cur < 2) cur++; draw(); });
    };
    wrap.addEventListener('click', e => { const b = e.target.closest('[data-i]'); if (b) { cur = +b.dataset.i; draw(); } });
    draw();
    hint(host, 'Số nhánh "khoảng 300" là tổng các chỗ đếm được trong file đề xuất, minh hoạ.');
  };

  WF.electron = host => {
    const box = (t, d, cls = '') => `<div class="wf-box ${cls}" tabindex="0"><b>${t}</b><span>${d}</span></div>`;
    compare(host, [
      stage => {
        stage.insertAdjacentHTML('beforeend', `<div class="wf-dia">
          <div class="wf-col">${box('Electron', 'apps/desktop/src, khoảng 54.8k dòng TS', 'old')}${box('Tauri shell', 'apps/shell, Rust')}</div>
          <div class="wf-arrow"><svg width="28" height="16" viewBox="0 0 28 16" aria-hidden="true"><path d="M2 8h22M18 3l6 5-6 5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg></div>
          <div class="wf-col">${box('Collector Node', 'khoảng 14.7k dòng, fallback CCM_CORE=node', 'old')}${box('orrery-core', 'Rust, mặc định từ 10/11')}</div>
        </div><div class="wf-meter"><span>Twin test: <b>33</b></span><span>Shell trong E2E mặc định: <b>Electron</b></span></div>`);
        hint(stage, 'Mỗi sửa đổi desktop làm 2 lần (Node + Rust). Hover hoặc focus từng khối.');
      },
      stage => {
        stage.insertAdjacentHTML('beforeend', `<div class="wf-dia">
          <div class="wf-col">${box('Tauri shell', 'apps/shell, preload.ts giữ làm contract cho gen-desk-shim')}</div>
          <div class="wf-arrow"><svg width="28" height="16" viewBox="0 0 28 16" aria-hidden="true"><path d="M2 8h22M18 3l6 5-6 5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg></div>
          <div class="wf-col">${box('orrery-core', 'Rust, không còn CCM_CORE_*=node')}</div>
        </div><div class="wf-meter"><span>Twin test: <b>0</b></span><span>Bớt khoảng <b>50 đến 70k</b> dòng</span><span>Rollback: <b>git tag</b> bản cuối có Electron</span></div>`);
        hint(stage, 'Làm sau khi Tauri chạy ổn trên Mac.');
      }
    ]);
  };

  WF.deadcode = host => {
    const groups = [
      { t: 'Component không ai import', n: '10 file, 1502 dòng', items: ['ActivityFeed.vue', 'ToolTimeline.vue', 'MetricsStrip.vue', 'WaitingQueue.vue', 'SessionList.vue', 'SessionSummary.vue', 'FlipClock.vue', 'monitor/OrbitRings.vue', 'next/ui/GlassDialog.vue', 'workspace/DevServerChip.vue'] },
      { t: 'Comment nhắc Mission Control', n: '2 chỗ', items: ['components/workspace/DropTerminal.vue:21', 'lib/paneSnapshots.ts:1'] },
      { t: 'DeepSeek đang dừng', n: '55 file web', items: ['SettingsDeepSeek.vue 18.5 KB', 'apps/desktop/src/deepseek/ khoảng 1.1k dòng', 'LiveRail.vue:16 import'], pick: ['Xoá hẳn, giữ git tag', 'Tách features/deepseek/ lazy'] },
      { t: '.code_index', n: '761 KB', bars: [['web.md', 172], ['_meta.md', 137], ['desktop.md', 134]] }
    ];
    const list = h('div', 'wf-acc');
    list.innerHTML = groups.map((g, i) => `<div class="wf-acc-i"><button type="button" class="wf-acc-h" aria-expanded="${i === 0}" data-i="${i}"><span class="wf-chev">${ICON.chev}</span><b>${g.t}</b><span class="wf-time">${g.n}</span></button>
      <div class="wf-acc-b" ${i === 0 ? '' : 'hidden'}>
        ${g.items ? `<ul class="wf-files">${g.items.map(x => `<li><label><input type="checkbox" checked> <code>${x}</code></label></li>`).join('')}</ul>` : ''}
        ${g.pick ? `<div class="wf-pick">${g.pick.map((p, j) => `<label><input type="radio" name="ds" ${j === 1 ? 'checked' : ''}> ${p}</label>`).join('')}<span class="wf-time">Cần chủ nhân chốt</span></div>` : ''}
        ${g.bars ? `<div class="wf-bars">${g.bars.map(([f, kb]) => `<div class="wf-barrow"><code>${f}</code><span class="wf-track"><i style="width:${kb / 172 * 100}%"></i><em style="left:${40 / 172 * 100}%"></em></span><span class="wf-kb">${kb} KB</span></div>`).join('')}<p class="wf-hint">Vạch dọc là mục tiêu dưới khoảng 40 KB mỗi file; phần trước v90 của _meta.md sang archive/.</p></div>` : ''}
      </div></div>`).join('');
    host.appendChild(list);
    const sum = hint(host, '');
    const upd = () => { const n = list.querySelectorAll('.wf-files input:checked').length; sum.textContent = `Đang chọn xoá ${n} mục. Typecheck + vitest bắt nếu còn chỗ dùng.`; };
    list.addEventListener('click', e => {
      const b = e.target.closest('.wf-acc-h');
      if (!b) return;
      const open = b.getAttribute('aria-expanded') !== 'true';
      b.setAttribute('aria-expanded', String(open));
      b.nextElementSibling.hidden = !open;
    });
    list.addEventListener('change', upd);
    upd();
  };

  WF.clash = host => {
    compare(host, [
      stage => {
        stage.insertAdjacentHTML('beforeend', `<div class="wf-win"><div class="wf-body wf-split">
          <div class="wf-rail"><div class="wf-row">agent-a <span class="wf-dot ok"></span></div><div class="wf-row">agent-b <span class="wf-dot ok"></span></div></div>
          <div class="wf-pane"><div class="wf-ph">Explorer</div><div class="wf-tree"><div>src/</div><div class="ind">api.ts <span class="wf-tag warn">${ICON.warn}2 agent</span></div><div class="ind">ui.ts</div></div></div>
        </div></div>`);
        hint(stage, 'Chỉ thấy khi mở Explorer đúng thư mục. Rail, header pane không báo, không có notification.');
      },
      stage => {
        const win = h('div', 'wf-win');
        win.innerHTML = `<div class="wf-body wf-split wf-rel">
          <div class="wf-rail"><div class="wf-row" data-r="a">agent-a <span class="wf-dot ok"></span></div><div class="wf-row" data-r="b">agent-b <span class="wf-dot ok"></span></div></div>
          <div class="wf-pane"><div class="wf-phead">agent-a <span class="wf-sp"></span><span class="wf-slot"></span></div><div class="wf-term">Edit src/api.ts<br>Edit src/ui.ts</div></div>
          <div class="wf-toast" hidden></div>
        </div>`;
        const go = h('button', 'wf-btn', 'Mô phỏng: agent-b sửa src/api.ts');
        go.type = 'button';
        stage.append(win, go);
        const st = hint(stage, 'agent-a đang sửa src/api.ts trong lượt hiện tại.');
        const toast = win.querySelector('.wf-toast');
        const chip = (who) => `<span class="wf-tag warn">${ICON.warn}Đụng file với ${who}: <code>api.ts</code></span>`;
        let ignored = false;
        const clear = () => { win.querySelectorAll('.wf-row .wf-tag').forEach(t => t.remove()); win.querySelector('.wf-slot').innerHTML = ''; toast.hidden = true; };
        go.addEventListener('click', () => {
          if (ignored) { st.textContent = 'File này đã bỏ qua, không cảnh báo lại.'; return; }
          clear();
          win.querySelector('[data-r="a"]').insertAdjacentHTML('beforeend', chip('agent-b'));
          win.querySelector('[data-r="b"]').insertAdjacentHTML('beforeend', chip('agent-a'));
          win.querySelector('.wf-slot').innerHTML = chip('agent-b');
          toast.hidden = false;
          toast.innerHTML = `<div><b>${ICON.warn} clash</b> agent-a và agent-b cùng sửa <code>src/api.ts</code></div><div class="wf-acts"><button type="button" class="wf-btn sm" data-a="diff">Mở diff</button><button type="button" class="wf-btn sm" data-a="wt">Tách worktree</button><button type="button" class="wf-btn sm" data-a="ign">Bỏ qua cho file này</button></div>`;
          st.textContent = 'Rail, header pane và toast cùng báo. Chỉ cảnh báo, không chặn.';
        });
        toast.addEventListener('click', e => {
          const a = e.target.closest('[data-a]');
          if (!a) return;
          if (a.dataset.a === 'diff') st.textContent = 'Mở diff src/api.ts (minh hoạ).';
          if (a.dataset.a === 'wt') st.textContent = 'Gợi ý tách worktree cho agent-b qua wt.add có sẵn (minh hoạ).';
          if (a.dataset.a === 'ign') { ignored = true; clear(); st.textContent = 'Đã bỏ qua src/api.ts.'; }
        });
      }
    ]);
  };

  WF.changes = host => {
    const files = [
      { f: 'src/api.ts', s: 'agent-a', k: 'M' }, { f: 'src/routes.ts', s: 'agent-a', k: 'A' },
      { f: 'web/App.vue', s: 'agent-b', k: 'M' }, { f: 'web/store.ts', s: 'agent-b', k: 'M' },
      { f: 'pnpm-lock.yaml', s: 'Không rõ', k: 'M' }, { f: 'gen/proto.ts', s: 'Không rõ', k: 'M' }
    ];
    compare(host, [
      stage => {
        stage.insertAdjacentHTML('beforeend', `<div class="wf-win"><div class="wf-bar"><b>Changes</b><span class="wf-sp"></span><span class="wf-count">Chưa xem 6</span></div>
          <ul class="wf-cl">${files.map(x => `<li><span class="wf-k k-${x.k}">${x.k}</span><code>${x.f}</code></li>`).join('')}</ul></div>`);
        hint(stage, 'Không biết file nào do agent nào sửa, phải tự lọc bằng mắt.');
      },
      stage => {
        const sess = ['Tất cả', 'agent-a', 'agent-b', 'Không rõ'];
        const win = h('div', 'wf-win wf-rel');
        win.innerHTML = `<div class="wf-toast top"><div><b>${ICON.check} done</b> agent-b đã xong lượt</div><div class="wf-acts"><button type="button" class="wf-btn sm on" data-see>Xem thay đổi</button></div></div>
          <div class="wf-bar"><b>Changes</b><span class="wf-sp"></span><div class="wf-cmp sm" role="group" aria-label="Theo session">${sess.map((s, i) => `<button type="button" data-s="${s}" class="${i === 0 ? 'on' : ''}">${s}</button>`).join('')}</div></div>
          <ul class="wf-cl"></ul>`;
        stage.appendChild(win);
        const st = hint(stage, 'Bấm "Xem thay đổi" trên toast, hoặc đổi session ở bộ lọc. Nhóm "Không rõ" (file do Bash sửa) luôn hiện.');
        const ul = win.querySelector('.wf-cl');
        const seg = win.querySelector('.wf-cmp');
        const pick = s => {
          seg.querySelectorAll('button').forEach(b => b.classList.toggle('on', b.dataset.s === s));
          const shown = files.filter(x => s === 'Tất cả' || x.s === s);
          ul.innerHTML = shown.map(x => `<li class="in"><span class="wf-k k-${x.k}">${x.k}</span><code>${x.f}</code><span class="wf-time">${x.s}</span></li>`).join('');
        };
        seg.addEventListener('click', e => { const b = e.target.closest('[data-s]'); if (b) pick(b.dataset.s); });
        win.querySelector('[data-see]').addEventListener('click', () => { pick('agent-b'); win.querySelector('.wf-toast').hidden = true; st.textContent = 'Mở Changes đã lọc theo agent-b, chỉ còn 2 file của lượt vừa xong.'; });
        pick('Tất cả');
      }
    ]);
  };
})();
