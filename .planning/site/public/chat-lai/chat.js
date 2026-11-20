(() => {
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
  const ic = (n, cls = '') => `<svg class="i ${cls}" aria-hidden="true"><use href="#i-${n}"/></svg>`;
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  const strip = (h) => h.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').replace(/\s([.,:])/g, '$1').trim();
  const reduce = matchMedia('(prefers-reduced-motion: reduce)');

  const AGENT = { claude: 'Claude', codex: 'Codex', shell: 'Shell' };
  const STATE = {
    working: ['Đang chạy', 'spin'],
    waiting: ['Chờ bạn', 'clock'],
    idle: ['Rảnh', 'moon'],
    done: ['Xong', 'check'],
    error: ['Lỗi', 'alert'],
  };
  const MACHINES = [
    { id: 'desk', name: 'Orrery Desk', link: 'Relay' },
    { id: 'mac', name: 'MacBook', link: 'LAN' },
  ];
  const PROJECTS = ['api', 'web', 'orrery', 'collector', 'docs'];
  const QUOTA = {
    claude: { h5: 42, week: 91, r5: '2 giờ 10 phút', rw: 'thứ Hai' },
    codex: { h5: 7, week: 34, r5: '4 giờ 5 phút', rw: 'thứ Năm' },
  };

  const codeAuth = `<div class="code"><div class="code-head"><span>auth/google.ts</span><button type="button" data-act="copy">Chép</button></div><pre><code><span class="tk-k">export const</span> google = {
  clientId: env.<span class="tk-p">GOOGLE_ID</span>,
  secret: <span class="tk-m">••••••••</span>,
  redirectUri: <span class="tk-s">\`\${env.BASE_URL}/auth/callback/\`</span>,
};</code></pre></div>`;

  const F_AUTH = [
    {
      p: 'auth/google.ts', a: 2, d: 1, hunks: [[4, [
        ' export const google = {',
        '   clientId: env.GOOGLE_ID,',
        '   secret: ••••••••,',
        '-  redirectUri: `${env.BASE_URL}/auth/callback`,',
        '+  redirectUri: `${env.BASE_URL}/auth/callback/`,',
        '+  scope: ["openid", "email"],',
        ' };',
      ]]],
    },
    {
      p: 'auth/config.ts', a: 1, d: 1, hunks: [[12, [
        ' export function callbackPath() {',
        "-  return '/auth/callback';",
        "+  return '/auth/callback/';",
        ' }',
      ]]],
    },
    {
      p: 'test/auth.test.ts', a: 14, d: 0, hunks: [[40, [
        ' });',
        ' ',
        "+describe('google callback', () => {",
        "+  it('keeps the trailing slash', () => {",
        '+    expect(google.redirectUri.endsWith("/")).toBe(true);',
        '+  });',
        '+',
        "+  it('returns 302 on success', async () => {",
        "+    const res = await app.inject({ url: '/auth/callback/?code=x' });",
        '+    expect(res.statusCode).toBe(302);',
        '+  });',
        '+',
        "+  it('rejects a missing code', async () => {",
        "+    const res = await app.inject({ url: '/auth/callback/' });",
        '+    expect(res.statusCode).toBe(400);',
        '+  });',
        '+});',
      ]]],
    },
  ];
  const F_THEME = [
    {
      p: 'styles/theme.css', a: 38, d: 0, hunks: [[88, [
        ' }',
        ' ',
        '+@media (prefers-color-scheme: dark) {',
        '+  :root {',
        '+    --bg: #1c1c1e;',
        '+    --ink: #f2f2f7;',
        '+    --line: rgb(255 255 255 / 0.1);',
        '+  }',
        '+}',
      ]]],
    },
  ];
  const F_QUOTA = [
    { p: 'test/quota.test.ts', a: 142, d: 0, hunks: [[1, ["+import { readQuota } from '../src/quota';", "+describe('readQuota', () => {", "+  it('reads the 5h window', () => {", '+    expect(readQuota(fx).h5).toBe(42);', '+  });', '+});']]] },
  ];
  const F_RENAME = ['stores/terminals.ts', 'components/Pane.vue', 'components/Rail.vue', 'lib/keys.ts', 'App.vue'].map((p, k) => ({
    p, a: [6, 4, 3, 3, 2][k], d: [6, 4, 3, 3, 2][k], hunks: [[[3, 18, 7, 11, 22][k], ["-import { useTerm } from '@/stores/term';", "+import { useTerminals } from '@/stores/terminals';", ' ', '-const term = useTerm();', '+const term = useTerminals();']]],
  }));
  const F_SETTINGS = [
    { p: 'pages/Settings.vue', a: 9, d: 4, hunks: [[21, ['-<div class="page light">', '+<div class="page">', '   <h1>{{ t("settings") }}</h1>', '-  <Toggle v-model="dark" />', '+  <ThemePicker v-model="theme" />']]] },
    { p: 'lib/theme.ts', a: 12, d: 0, hunks: [[1, ["+export type Theme = 'system' | 'light' | 'dark';", "+export function applyTheme(t: Theme) {", "+  document.documentElement.dataset.theme = t;", '+}']]] },
  ];

  const SEED = [
    {
      id: 's1', m: 'desk', agent: 'claude', name: 'Sửa lỗi đăng nhập', project: 'api', state: 'waiting', time: '2 phút', ago: 2,
      model: 'Opus 4.5', ctx: 64, subs: [['Explore', 'tìm chỗ gọi OAuth', 'working'], ['test-runner', 'chờ quyền', 'waiting']],
      msgs: [
        { t: 'me', text: 'Đăng nhập Google bị lỗi 500, xem giúp.' },
        { t: 'tools', sum: 'Đã đọc 4 file, chạy 1 lệnh', items: ['Đọc <code>auth/google.ts</code>', 'Đọc <code>auth/config.ts</code>', 'Đọc <code>routes/auth.ts</code>', 'Đọc <code>test/auth.test.ts</code>', 'Chạy <code>rg redirect_uri</code>'] },
        { t: 'ai', html: `<p>Lỗi do <code>redirect_uri</code> thiếu dấu <code>/</code> cuối nên Google từ chối callback.</p>${codeAuth}<p>Đã thêm 3 test cho case này. Cần chạy test để chắc chắn.</p>` },
        { t: 'files', files: F_AUTH },
        { t: 'perm', cmd: 'npm test -- auth', status: 'open' },
      ],
      term: [
        '<span class="o">✻</span> <b>Claude Code</b> <span class="dim">~/work/api</span>',
        '',
        '<span class="dim">&gt;</span> Đăng nhập Google bị lỗi 500, xem giúp.',
        '',
        '<span class="g">●</span> Read(auth/google.ts)',
        '<span class="g">●</span> Update(auth/google.ts) <span class="g">+2</span> <span class="r">-1</span>',
        '<span class="g">●</span> Update(test/auth.test.ts) <span class="g">+14</span>',
        '<span class="y">●</span> Bash(npm test -- auth)',
        '',
        '  Cho chạy lệnh này?',
        '  <span class="b">❯ 1. Có</span>',
        '    2. Có, không hỏi lại',
        '    3. Không',
      ],
    },
    {
      id: 's2', m: 'desk', agent: 'codex', name: 'Gộp Inbox và Giám sát', project: 'orrery', state: 'waiting', time: '6 phút', ago: 6,
      model: 'gpt-5-codex', ctx: 38, subs: [],
      msgs: [
        { t: 'me', text: 'Gộp màn Inbox với Giám sát trên điện thoại.' },
        { t: 'tools', sum: 'Đã đọc 6 file', items: ['Đọc <code>InboxView.vue</code>', 'Đọc <code>MonitorView.vue</code>', 'Đọc <code>router.ts</code>', 'Đọc <code>stores/sessions.ts</code>', 'Đọc <code>InboxSheet.vue</code>', 'Đọc <code>PLAN-v107.md</code>'] },
        { t: 'ai', html: '<p>Có 2 cách:</p><ul><li>Một danh sách, Chờ bạn lên đầu. Ít sửa nhất.</li><li>Hai tab trong cùng màn.</li></ul>' },
        { t: 'menu', q: 'Chọn cách làm', options: ['Một danh sách', 'Hai tab', 'Để sau, chỉ ghi plan'], status: 'open' },
      ],
      term: ['<span class="b">codex</span> <span class="dim">~/work/orrery</span>', '', '<span class="dim">›</span> Gộp màn Inbox với Giám sát trên điện thoại.', '', '  Chọn cách làm', '  <span class="b">› 1. Một danh sách</span>', '    2. Hai tab', '    3. Để sau, chỉ ghi plan'],
    },
    {
      id: 's3', m: 'desk', agent: 'claude', name: 'Thêm dark mode', project: 'web', state: 'working', time: 'vừa xong', ago: 0,
      model: 'Sonnet 4.5', ctx: 41, act: ['Đang sửa', 'Settings.vue'], runFor: 72, subs: [['Explore', 'tìm token màu', 'done']],
      msgs: [
        { t: 'me', text: 'Thêm dark mode cho trang cài đặt, theo hệ thống.' },
        { t: 'tools', sum: 'Đã đọc 3 file, chạy 1 lệnh', items: ['Đọc <code>styles/theme.css</code>', 'Đọc <code>pages/Settings.vue</code>', 'Đọc <code>lib/theme.ts</code>', 'Chạy <code>npm run typecheck</code>'] },
        { t: 'ai', html: '<p>Đã thêm token tối vào <code>theme.css</code>. Giờ sửa nốt trang cài đặt.</p>' },
        { t: 'files', files: F_THEME },
      ],
      steps: [['Đang sửa', 'Settings.vue'], ['Đang chạy', 'npm run typecheck'], ['Đang đọc', 'lib/theme.ts'], ['Đang sửa', 'lib/theme.ts']],
      reply: '<p>Xong. Trang cài đặt theo hệ thống, có thêm lựa chọn Sáng và Tối.</p>',
      replyFiles: F_SETTINGS,
      term: ['<span class="o">✻</span> <b>Claude Code</b> <span class="dim">~/work/web</span>', '', '<span class="g">●</span> Update(styles/theme.css) <span class="g">+38</span>', '<span class="g">●</span> Bash(npm run typecheck)', '  <span class="dim">0 errors</span>', '<span class="b">●</span> Update(Settings.vue)', '', '<span class="o">✻ Đang làm…</span> <span class="dim">(1m 12s, esc để dừng)</span>'],
    },
    {
      id: 's4', m: 'desk', agent: 'codex', name: 'Test quota', project: 'collector', state: 'done', time: '14 phút', ago: 14,
      model: 'gpt-5-codex', ctx: 22, subs: [],
      msgs: [
        { t: 'me', text: 'Viết test cho bộ đọc quota.' },
        { t: 'tools', sum: 'Chạy 1 lệnh', items: ['Chạy <code>npm test -w @ccm/collector</code>'] },
        { t: 'ai', html: '<p>Thêm 12 test, tất cả pass.</p>' },
        { t: 'files', files: F_QUOTA },
      ],
      term: ['<span class="b">codex</span> <span class="dim">~/work/collector</span>', '', '<span class="g">✓</span> 12 passed <span class="dim">(1.4s)</span>'],
    },
    {
      id: 's5', m: 'desk', agent: 'shell', name: 'api dev', project: 'api', state: 'idle', time: '20 phút', ago: 20,
      last: 'Server chạy ở cổng 3000',
      msgs: [],
      term: ['<span class="g">~/work/api</span> <span class="dim">$</span> npm run dev', '', '<span class="dim">&gt; api@2.3.0 dev</span>', '<span class="dim">&gt; tsx watch src/server.ts</span>', '', '<span class="b">info</span>  Đọc .env <span class="dim">(4 biến, đã che)</span>', '<span class="g">ready</span> Server chạy ở cổng 3000', '<span class="dim">GET /health 200 3ms</span>', '<span class="dim">POST /auth/callback/ 302 41ms</span>'],
    },
    {
      id: 's6', m: 'desk', agent: 'claude', name: 'Release note 4.0.9', project: 'docs', state: 'error', time: '1 giờ', ago: 60,
      model: 'Opus 4.5', ctx: 100, subs: [],
      msgs: [
        { t: 'me', text: 'Viết release note 4.0.9 từ git log.' },
        { t: 'ai', html: '<p>Hết context giữa chừng. Chạy <code>/compact</code> rồi làm tiếp.</p>' },
      ],
      term: ['<span class="r">●</span> Context đầy (100%)'],
    },
    {
      id: 's7', m: 'desk', agent: 'claude', name: 'Đổi tên store', project: 'orrery', state: 'idle', time: 'Hôm qua', ago: 1440,
      model: 'Sonnet 4.5', ctx: 18, subs: [],
      msgs: [
        { t: 'me', text: 'Đổi useTerm thành useTerminals ở mọi chỗ.' },
        { t: 'ai', html: '<p>Đã đổi 18 chỗ trong 5 file. Typecheck sạch.</p>' },
        { t: 'files', files: F_RENAME },
      ],
      term: ['<span class="g">●</span> 18 chỗ, 5 file'],
    },
    {
      id: 'm1', m: 'mac', agent: 'claude', name: 'Layout iPhone SE', project: 'ios-app', state: 'working', time: 'vừa xong', ago: 0,
      model: 'Opus 4.5', ctx: 55, act: ['Đang chạy', 'xcodebuild'], runFor: 210, subs: [],
      msgs: [
        { t: 'me', text: 'Màn Home bị tràn trên iPhone SE, sửa giúp.' },
        { t: 'ai', html: '<p>Do <code>HStack</code> không co chữ. Đang build lại để chụp màn.</p>' },
      ],
      term: ['<span class="o">✻</span> <b>Claude Code</b> <span class="dim">~/work/ios-app</span>', '<span class="b">●</span> Bash(xcodebuild -scheme App)'],
    },
    {
      id: 'm2', m: 'mac', agent: 'shell', name: 'simulator', project: 'ios-app', state: 'idle', time: '35 phút', ago: 35,
      last: 'Booted iPhone SE (3rd gen)', msgs: [],
      term: ['<span class="dim">$</span> xcrun simctl boot "iPhone SE (3rd generation)"', '<span class="g">Booted</span>'],
    },
  ];

  const STEPS = {
    claude: [['Đang đọc', 'auth/google.ts'], ['Đang sửa', 'auth/google.ts'], ['Đang chạy', 'npm test']],
    codex: [['Đang đọc', 'router.ts'], ['Đang sửa', 'InboxView.vue'], ['Đang chạy', 'npm run typecheck']],
  };
  const F_REPLY = [
    { p: 'src/app.ts', a: 5, d: 2, hunks: [[30, [' app.use(cors());', '-app.use(json());', '+app.use(json({ limit: "1mb" }));', '+app.use(rateLimit({ max: 60 }));', ' app.listen(port);']]] },
  ];

  const clone = (o) => JSON.parse(JSON.stringify(o));
  let sessions = clone(SEED);
  const now = () => Date.now();
  sessions.forEach(prep);

  function prep(s) {
    if (s.runFor) { s.runStart = now() - s.runFor * 1000; s._running = true; }
    if (s.steps) s.stepI = 0;
    if (!s.last) s.last = lastLine(s);
    s.termMode = s.agent === 'shell';
    if (s.state === 'working' && s.steps) { s.loop = true; s.timer = setTimeout(() => step(s), 2400); }
  }

  function actText(s) { return s.act ? `${s.act[0]} ${s.act[1]}` : 'Đang làm'; }

  function lastLine(s) {
    for (let i = s.msgs.length - 1; i >= 0; i--) {
      const m = s.msgs[i];
      if (m.t === 'perm' && m.status === 'open') return `Cho chạy ${m.cmd}?`;
      if (m.t === 'menu' && m.status === 'open') return `${m.q}, ${m.options.length} lựa chọn`;
      if (m.t === 'ai') return strip(m.html);
    }
    return '';
  }

  const ui = { machine: 'desk', open: null, offline: false, query: '', newProject: 'api', sheetStack: [], lat: 42, diff: null };

  const app = $('#app');
  const device = $('#device');
  const list = $('#list');
  const msgs = $('#msgs');
  const input = $('#input');
  const byId = (id) => sessions.find((s) => s.id === id);
  const cur = () => byId(ui.open);

  function stHTML(state, withText = true) {
    const [label, icon] = STATE[state];
    const g = icon === 'spin' ? '<span class="spin" aria-hidden="true"></span>' : ic(icon);
    return `<span class="st st-${state}">${g}${withText ? label : `<span class="sr">${label}</span>`}</span>`;
  }

  function fmt(sec) { return `${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, '0')}`; }

  function renderQuota() {
    const hot = [];
    Object.entries(QUOTA).forEach(([a, q]) => {
      if (q.h5 >= 80) hot.push([`${AGENT[a]}, 5 giờ`, q.h5]);
      if (q.week >= 80) hot.push([`${AGENT[a]}, tuần`, q.week]);
    });
    $('#quotaLine').innerHTML = hot.map(([l, p]) => `<button class="quota-line" data-quota>${ic('gauge', 'sm')}<span>Quota ${l} <b>${p}%</b></span><span class="q-bar" aria-hidden="true"><i style="width:${p}%"></i></span></button>`).join('');
  }

  function renderList() {
    const q = ui.query.trim().toLowerCase();
    const mine = sessions.filter((s) => s.m === ui.machine && (!q || (s.name + ' ' + s.last + ' ' + s.project).toLowerCase().includes(q)));
    const wait = mine.filter((s) => s.state === 'waiting').sort((a, b) => a.ago - b.ago);
    const rest = mine.filter((s) => s.state !== 'waiting').sort((a, b) => a.ago - b.ago);
    const row = (s) => {
      let bot;
      if (s.state === 'working') bot = `${stHTML('working', false)}<span class="row-last">${esc(actText(s))}</span>`;
      else if (s.state === 'idle') bot = `<span class="row-last">${esc(s.last)}</span>`;
      else bot = `${stHTML(s.state, s.state !== 'waiting')}<span class="row-last">${esc(s.last)}</span>`;
      return `<button class="row" data-open="${s.id}"${ui.open === s.id ? ' aria-current="true"' : ''}>
      <span class="glyph av ${s.agent}" role="img" aria-label="${AGENT[s.agent]}">${ic(s.agent)}</span>
      <span class="row-main">
        <span class="row-top"><span class="row-name">${esc(s.name)}</span><span class="row-meta"><span class="proj">${esc(s.project)}</span><span>${s.time}</span></span></span>
        <span class="row-bot">${bot}</span>
      </span>
    </button>`;
    };
    let h = '';
    if (wait.length) h += `<h3 class="group-label wait">Chờ bạn, ${wait.length}</h3><div class="card-list wait">${wait.map(row).join('')}</div>`;
    if (rest.length) h += `<h3 class="group-label">Gần đây</h3><div class="card-list">${rest.map(row).join('')}</div>`;
    if (!mine.length) h = `<p class="empty-list">Không có session nào khớp.</p>`;
    list.innerHTML = h;
    $('#machineName').textContent = MACHINES.find((m) => m.id === ui.machine).name;
    renderQuota();
  }

  function diffStat(files) {
    return files.reduce((t, f) => [t[0] + f.a, t[1] + f.d], [0, 0]);
  }
  function statHTML(a, d) {
    return `<span class="add">+${a}</span>${d ? ` <span class="del">-${d}</span>` : ''}`;
  }
  function splitPath(p) {
    const k = p.lastIndexOf('/');
    return k < 0 ? esc(p) : `<span>${esc(p.slice(0, k + 1))}</span>${esc(p.slice(k + 1))}`;
  }

  function msgHTML(s, m, i) {
    if (m.t === 'me') {
      return `<div class="me${m.pending ? ' unsent' : ''}"><div class="bub">${esc(m.text)}</div>${m.pending ? `<div class="unsent-row">${ic('alert')}Chưa gửi<button class="mini" data-act="resend" data-i="${i}">Gửi lại</button></div>` : ''}</div>`;
    }
    if (m.t === 'ai') return `<div class="ai">${m.html}</div>`;
    if (m.t === 'tools') {
      return `<div class="note"><button class="tool-line" aria-expanded="${!!m.open}" data-act="tools" data-i="${i}">${ic('wrench')}<span>${m.sum}</span>${ic('chev-right', 'chev')}</button><ul class="tool-list"${m.open ? '' : ' hidden'}>${m.items.map((x) => `<li>${x}</li>`).join('')}</ul></div>`;
    }
    if (m.t === 'files') {
      const [a, d] = diffStat(m.files);
      return `<div class="files"><div class="files-head">${ic('pencil')}<span>Đã sửa ${m.files.length} file</span><span class="sum">${statHTML(a, d)}</span></div>${m.files.map((f, k) => `<button class="f-row" data-act="file" data-i="${i}" data-f="${k}"><span class="f-name">${splitPath(f.p)}</span>${statHTML(f.a, f.d)}${ic('chev-right')}</button>`).join('')}</div>`;
    }
    if (m.t === 'sys') return `<div class="sys ${m.kind || ''}">${m.kind === 'ok' ? ic('check') : m.kind === 'stop' ? ic('stop') : ''}${esc(m.text)}</div>`;
    if (m.t === 'perm' || m.t === 'menu') {
      if (m.status === 'pending') {
        const left = Math.max(0, 3000 - (now() - m.at));
        return `<div class="sent"><div class="sent-row">${ic('check')}<span>Đã gửi <b>${esc(m.choice)}</b></span><button class="undo" data-act="undo" data-i="${i}">${ic('undo')}Hoàn tác</button></div><div class="undo-bar"><i style="animation-duration:3000ms;animation-delay:-${3000 - left}ms"></i></div></div>`;
      }
      if (m.status === 'done') {
        const t = m.t === 'perm' ? `${m.choice}, ${m.cmd}` : `Đã chọn ${m.choice}`;
        return `<div class="sys ok">${ic('check')}${esc(t)}</div>`;
      }
      const dis = ui.offline ? ' disabled' : '';
      if (m.t === 'perm') {
        return `<div class="ask"><div class="ask-head">${ic('lock')}<span>Cho chạy lệnh này?</span></div><code class="cmd">${esc(m.cmd)}</code>
          <div class="ask-btns"><button class="btn primary" data-act="pick" data-i="${i}" data-v="Có"${dis}>Có</button><button class="btn" data-act="pick" data-i="${i}" data-v="Không"${dis}>Không</button><button class="btn span2" data-act="pick" data-i="${i}" data-v="Luôn cho phép"${dis}>Luôn cho phép</button></div></div>`;
      }
      return `<div class="ask"><div class="ask-head">${ic('list')}<span>${esc(m.q)}</span></div>
        <div class="ask-opts">${m.options.map((o, k) => `<button class="btn opt" data-act="pick" data-i="${i}" data-v="${k + 1}. ${esc(o)}"${dis}><span class="num">${k + 1}</span>${esc(o)}</button>`).join('')}</div></div>`;
    }
    return '';
  }

  function liveHTML(s) {
    const sec = Math.max(0, Math.floor((now() - (s.runStart || now())) / 1000));
    const [v, t] = s.act || ['Đang làm', ''];
    return `<div class="live" role="status"><span class="pulse-dot" aria-hidden="true"></span><span class="what">${esc(v)} ${esc(t)}</span><time id="liveTime">${fmt(sec)}</time></div>`;
  }

  function quotaHot(agent) {
    const q = QUOTA[agent];
    return q && (q.h5 >= 80 || q.week >= 80);
  }

  function renderCtx(s) {
    const b = $('#ctxBtn');
    b.hidden = s.agent === 'shell';
    if (b.hidden) return;
    const c = 2 * Math.PI * 12;
    const pct = s.ctx || 0;
    const qh = quotaHot(s.agent);
    b.className = `ctx${pct >= 80 || qh ? ' warn' : ''}`;
    b.setAttribute('aria-label', `Context ${pct}%${qh ? ', quota gần hết' : ''}, xem quota`);
    b.innerHTML = `<svg class="ring" viewBox="0 0 28 28" aria-hidden="true"><circle class="tr" cx="14" cy="14" r="12"/><circle class="fg" cx="14" cy="14" r="12" stroke-dasharray="${(c * pct / 100).toFixed(2)} ${c.toFixed(2)}"/></svg><span class="pct" aria-hidden="true">${pct}</span>${qh ? '<span class="flag" aria-hidden="true"></span>' : ''}`;
  }

  function renderSub(s) {
    let lat = '';
    if (ui.offline) lat = `<span class="lat">${ic('wifi-off')}Mất mạng</span>`;
    else if (ui.lat > 150) lat = `<span class="lat">${ic('signal')}Chậm ${ui.lat} ms</span>`;
    const st = s.agent === 'shell' ? '' : stHTML(s.state);
    $('#chatSub').innerHTML = `${st}<span class="proj">${esc(s.project)}</span>${lat}`;
  }

  function renderChat(keepScroll) {
    const s = cur();
    if (!s) {
      msgs.innerHTML = `<div class="empty-chat"><p>Chọn một session</p></div>`;
      return;
    }
    const g = $('#chatIco');
    g.className = `glyph ${s.agent}`;
    g.innerHTML = ic(s.agent);
    g.setAttribute('role', 'img');
    g.setAttribute('aria-label', AGENT[s.agent]);
    $('#chatName').textContent = s.name;
    renderSub(s);
    renderCtx(s);
    const tb = $('#termBtn');
    tb.hidden = s.agent === 'shell';
    tb.setAttribute('aria-pressed', String(s.termMode));
    tb.setAttribute('aria-label', s.termMode ? 'Xem chat' : 'Xem terminal');
    tb.innerHTML = ic(s.termMode ? 'chat' : 'terminal');
    app.classList.toggle('term-mode', s.termMode);
    $('#term').hidden = !s.termMode;
    msgs.hidden = s.termMode;
    if (s.termMode) renderTerm(s);

    const atBottom = msgs.scrollHeight - msgs.scrollTop - msgs.clientHeight < 80;
    let h = s.msgs.map((m, i) => msgHTML(s, m, i)).join('');
    if (s.state === 'working') h += liveHTML(s);
    const empty = !s.msgs.length && s.agent !== 'shell';
    if (empty) h = `<div class="empty-chat"><h2 class="hello">Chào chủ nhân</h2><p><span class="glyph ${s.agent}">${ic(s.agent)}</span>${AGENT[s.agent]} ở ${esc(s.project)}</p></div>`;
    msgs.innerHTML = h;
    const chips = $('#chips');
    chips.hidden = !empty || s.termMode;
    chips.innerHTML = empty ? CHIPS.map((c) => `<button type="button" class="chip" data-chip="${esc(c)}">${esc(c)}</button>`).join('') : '';
    if (!keepScroll || atBottom) msgs.scrollTop = msgs.scrollHeight;

    input.placeholder = s.state === 'waiting' ? `Trả lời ${AGENT[s.agent]}` : `Nhắn ${AGENT[s.agent]}`;
    syncSend();
    syncPanel();
  }

  function renderTerm(s) {
    const out = $('#termOut');
    out.innerHTML = s.term.join('\n') + '\n<span class="cur"></span>';
    out.scrollTop = out.scrollHeight;
  }

  function refresh() {
    renderList();
    if (ui.open) renderChat(true);
    syncPanel();
  }

  function touch(s) {
    s.time = 'vừa xong';
    s.ago = 0;
    sessions.forEach((o) => { if (o !== s && o.ago === 0) o.ago = 1; });
  }

  function openChat(id, opts = {}) {
    const s = byId(id);
    if (!s) return;
    if (s.m !== ui.machine) ui.machine = s.m;
    ui.open = id;
    if (opts.term !== undefined) s.termMode = opts.term || s.agent === 'shell';
    renderChat(false);
    renderList();
    app.dataset.view = 'chat';
    $('#chat').inert = false;
    if (device.dataset.device === 'phone') $('#home').inert = true;
    setTimeout(() => { if (ui.open === id && device.dataset.device === 'phone') $('#backBtn').focus({ preventScroll: true }); }, 320);
  }

  function closeChat() {
    if (device.dataset.device === 'ipad') return;
    ui.open = null;
    app.dataset.view = 'home';
    app.classList.remove('term-mode');
    $('#home').inert = false;
    $('#chat').inert = true;
    renderList();
    syncPanel();
  }

  const CHIPS = ['Tóm tắt project', 'Chạy test', 'Xem git status', 'Sửa lỗi typecheck'];

  const REPLIES = {
    claude: ['<p>Xong. Đã kiểm lại, không lỗi.</p>', '<p>Đã sửa. Chạy lại test thấy pass hết.</p>'],
    codex: ['<p>Xong, đã ghi vào plan.</p>', '<p>Đã làm xong, typecheck sạch.</p>'],
  };

  function runAgent(s, opts = {}) {
    if (s.agent === 'shell') return;
    s.state = 'working';
    if (!s._running) s.runStart = now();
    s._running = true;
    s.loop = false;
    s.steps = opts.steps || STEPS[s.agent];
    s.stepI = 0;
    s.act = s.steps[0];
    s.reply = opts.reply || REPLIES[s.agent][Math.floor(Math.random() * 2)];
    s.replyFiles = opts.files === undefined ? F_REPLY : opts.files;
    s.last = actText(s);
    touch(s);
    clearTimeout(s.timer);
    s.timer = setTimeout(() => step(s), opts.ms || 1100);
    refresh();
  }

  function step(s) {
    s.stepI = (s.stepI || 0) + 1;
    if (s.stepI >= s.steps.length) {
      if (!s.loop) { finish(s); return; }
      s.stepI = 0;
    }
    s.act = s.steps[s.stepI];
    s.last = actText(s);
    s.timer = setTimeout(() => step(s), s.loop ? 2400 : 1100);
    refresh();
  }

  function finish(s) {
    s._running = false;
    s.runStart = 0;
    const r = s.reply || REPLIES[s.agent][0];
    s.msgs.push({ t: 'ai', html: r });
    if (s.replyFiles && s.replyFiles.length) s.msgs.push({ t: 'files', files: clone(s.replyFiles) });
    s.state = 'done';
    s.act = null;
    s.last = strip(r);
    touch(s);
    refresh();
  }

  function stop(s) {
    clearTimeout(s.timer);
    s._running = false;
    s.runStart = 0;
    s.state = 'idle';
    s.act = null;
    s.msgs.push({ t: 'sys', kind: 'stop', text: 'Đã dừng' });
    s.last = 'Đã dừng';
    touch(s);
    refresh();
    toast('Đã gửi Esc');
  }

  function send(text) {
    const s = cur();
    if (!s || !text.trim()) return;
    const m = { t: 'me', text: text.trim() };
    if (ui.offline) {
      m.pending = true;
      s.msgs.push(m);
      renderChat(false);
      return;
    }
    s.msgs.push(m);
    if (s.state === 'working') {
      touch(s);
      renderChat(false);
      renderList();
    } else {
      runAgent(s);
      msgs.scrollTop = msgs.scrollHeight;
    }
  }

  function pick(s, i, v) {
    const m = s.msgs[i];
    m.status = 'pending';
    m.choice = v;
    m.at = now();
    renderChat(true);
    clearTimeout(m._t);
    m._t = setTimeout(() => commit(s, m), 3000);
  }

  function commit(s, m) {
    if (m.status !== 'pending') return;
    m.status = 'done';
    if (m.t === 'perm' && m.choice === 'Không') {
      s.state = 'idle';
      s.msgs.push({ t: 'ai', html: '<p>Đã bỏ qua, chưa chạy test.</p>' });
      s.last = 'Đã bỏ qua, chưa chạy test.';
      touch(s);
      refresh();
      return;
    }
    if (m.t === 'perm') runAgent(s, { steps: [['Đang chạy', m.cmd], ['Đang đọc', 'kết quả test']], reply: '<p>15 test pass. Đăng nhập Google chạy lại bình thường.</p>', files: [] });
    else runAgent(s, { steps: [['Đang đọc', 'InboxView.vue'], ['Đang sửa', 'InboxView.vue'], ['Đang sửa', 'router.ts']], reply: `<p>Làm theo cách ${esc(m.choice.split('. ')[0])}. Inbox giờ là một danh sách, Chờ bạn lên đầu.</p>`, files: [F_REPLY[0], { p: 'src/views/InboxView.vue', a: 31, d: 12, hunks: [[8, ['-<MonitorTabs />', '+<SessionList :pinned="waiting" />']]] }] });
  }

  function undo(s, i) {
    const m = s.msgs[i];
    clearTimeout(m._t);
    m.status = 'open';
    renderChat(true);
    toast('Đã hoàn tác');
  }

  function setAgentState(st) {
    let s = cur();
    if (!s || s.agent === 'shell') {
      s = byId('s1');
      openChat('s1', { term: false });
    }
    clearTimeout(s.timer);
    s.msgs.forEach((m) => { if (m._t && m.status === 'pending') { clearTimeout(m._t); m.status = 'done'; } });
    if (st === 'working') {
      runAgent(s, { steps: [['Đang đọc', 'app.ts'], ['Đang sửa', 'app.ts'], ['Đang chạy', 'npm test'], ['Đang sửa', 'app.test.ts'], ['Đang chạy', 'npm test']] });
      return;
    }
    s._running = false;
    s.runStart = 0;
    s.act = null;
    if (st === 'waiting' && !s.msgs.some((m) => (m.t === 'perm' || m.t === 'menu') && m.status === 'open')) {
      s.msgs.push({ t: 'perm', cmd: s.agent === 'codex' ? 'cargo test' : 'npm test', status: 'open' });
    }
    s.state = st;
    s.last = lastLine(s) || 'Rảnh';
    touch(s);
    refresh();
  }

  let toastT;
  function toast(t) {
    const el = $('#toast');
    el.textContent = t;
    el.classList.add('show');
    clearTimeout(toastT);
    toastT = setTimeout(() => el.classList.remove('show'), 1800);
  }

  function tickRun() {
    const s = cur();
    if (!s || s.state !== 'working') return;
    const el = $('#liveTime');
    if (el) el.textContent = fmt(Math.max(0, Math.floor((now() - (s.runStart || now())) / 1000)));
  }
  setInterval(tickRun, 1000);

  function setLatency() {
    ui.lat = 30 + Math.floor(Math.random() * 30) + (Math.random() < 0.12 ? 140 : 0);
    const s = cur();
    if (s) renderSub(s);
  }
  setInterval(setLatency, 2200);

  let rejoinT;
  function setOffline(v) {
    ui.offline = v;
    app.classList.toggle('offline', v);
    $('#pOffline').checked = v;
    const sw = $('#sOffline');
    if (sw) sw.checked = v;
    clearTimeout(rejoinT);
    $$('.reconnect .rc-text').forEach((e) => { e.textContent = v ? 'Đang nối lại' : 'Đã nối lại'; });
    if (!v) {
      app.classList.add('rejoined');
      rejoinT = setTimeout(() => app.classList.remove('rejoined'), 1600);
    } else app.classList.remove('rejoined');
    if (ui.open) renderChat(true);
  }

  const sheet = $('#sheet');
  const scrim = $('#scrim');
  let sheetReturn = null;

  function meter(label, pct, sub) {
    const cls = pct >= 80 ? ' hot' : pct >= 65 ? ' warm' : '';
    return `<div class="meter${cls}"><div class="meter-top"><span>${label}</span>${sub ? `<span class="sub">${sub}</span>` : ''}<b>${pct >= 80 ? ic('alert', 'xs flag') + ' ' : ''}${pct}%</b></div><div class="meter-bar" role="progressbar" aria-label="${label}" aria-valuenow="${pct}" aria-valuemin="0" aria-valuemax="100"><i style="width:${pct}%"></i></div></div>`;
  }

  function quotaBlock(agent) {
    const q = QUOTA[agent];
    return `<div class="s-label">${AGENT[agent]}</div>${meter('5 giờ', q.h5, `đặt lại sau ${q.r5}`)}${meter('Tuần', q.week, `đặt lại ${q.rw}`)}`;
  }

  function lastFiles(s) {
    for (let i = s.msgs.length - 1; i >= 0; i--) if (s.msgs[i].t === 'files') return i;
    return -1;
  }

  function diffBody(f) {
    let out = '';
    f.hunks.forEach(([start, lines]) => {
      let ln = start;
      out += `<div class="hunk">Dòng ${start}</div>`;
      lines.forEach((l) => {
        const sign = l[0];
        const kind = sign === '+' ? 'a' : sign === '-' ? 'd' : '';
        const n = sign === '-' ? '' : ln++;
        out += `<div class="ln ${kind}"><span class="n">${n}</span><span class="s">${sign === ' ' ? '' : sign}</span><code>${esc(l.slice(1)) || ' '}</code></div>`;
      });
    });
    return out;
  }

  const SHEETS = {
    machine: () => ({
      title: 'Máy',
      body: `<div class="s-list" role="radiogroup">${MACHINES.map((m) => {
        const n = sessions.filter((s) => s.m === m.id && s.state === 'waiting').length;
        return `<button class="s-row" role="radio" aria-checked="${m.id === ui.machine}" data-machine="${m.id}">${ic('laptop')}<span class="grow">${m.name}<span class="sub">${m.link}${n ? `, ${n} chờ bạn` : ''}</span></span><span class="end">${m.id === ui.machine ? ic('check') : ''}</span></button>`;
      }).join('')}</div>`,
    }),
    new: () => ({
      title: 'Session mới',
      body: `<div class="new-opts">
        <button class="new-opt" data-new="claude"><span class="glyph claude">${ic('claude')}</span>Claude</button>
        <button class="new-opt" data-new="codex"><span class="glyph codex">${ic('codex')}</span>Codex</button>
        <button class="new-opt" data-new="shell"><span class="glyph shell">${ic('shell')}</span>Shell mới</button>
      </div>
      <div class="where"><span class="where-label">Ở</span><button class="pchip" aria-pressed="true" aria-expanded="${!!ui.placeOpen}" data-act="place">${ic('folder')}${ui.newProject}${ic(ui.placeOpen ? 'up' : 'chev-down')}</button>${ui.placeOpen ? PROJECTS.filter((p) => p !== ui.newProject).map((p) => `<button class="pchip" aria-pressed="false" data-proj="${p}">${p}</button>`).join('') : ''}</div>`,
    }),
    plus: () => {
      const s = cur();
      return {
        title: 'Thêm',
        body: `<div class="s-list">
          <button class="s-row" data-newshell>${ic('shell')}<span class="grow">Shell mới<span class="sub">${esc(s ? s.project : ui.newProject)}</span></span></button>
          <button class="s-row" data-act="stub">${ic('image')}<span class="grow">Ảnh</span></button>
          <button class="s-row" data-act="stub">${ic('file')}<span class="grow">File</span></button>
          <button class="s-row" data-act="stub">${ic('slash')}<span class="grow">Lệnh</span></button>
        </div>`,
      };
    },
    more: () => ({
      title: 'Thêm',
      body: `<div class="s-list">
        ${[['quota', 'gauge', 'Quota'], ['files', 'file', 'File'], ['git', 'branch', 'Git'], ['run', 'play', 'Run'], ['settings', 'gear', 'Cài đặt']].map(([k, i, l]) => `<button class="s-row" data-sheet="${k}">${ic(i)}<span class="grow">${l}</span><span class="end">${ic('chev-right', 'sm')}</span></button>`).join('')}
      </div>`,
    }),
    quota: () => ({ title: 'Quota', body: `${quotaBlock('claude')}${quotaBlock('codex')}` }),
    usage: () => {
      const s = cur();
      return {
        title: 'Context và quota',
        body: `${meter('Context', s.ctx || 0, s.ctx >= 80 ? 'nên /compact' : '')}${QUOTA[s.agent] ? quotaBlock(s.agent) : ''}<div class="s-sep"></div>
          <div class="kv"><span>Model</span><b>${esc(s.model || '')}</b></div>
          <div class="kv"><span>Mạng</span><b>${ui.offline ? 'Mất mạng' : `${ui.lat} ms, ${MACHINES.find((m) => m.id === s.m).link}`}</b></div>`,
      };
    },
    files: () => ({
      title: 'File',
      body: `<div class="s-list">${['auth/google.ts', 'auth/config.ts', 'test/auth.test.ts', 'styles/theme.css'].map((f) => `<button class="s-row" data-act="stub">${ic('file')}<span class="grow">${f}</span></button>`).join('')}</div>`,
    }),
    git: () => ({
      title: 'Git',
      body: `<div class="kv"><span>Nhánh</span><b>feat/v107-chat</b></div><div class="kv"><span>Chưa commit</span><b>3 file</b></div><div class="s-sep"></div>
        <div class="s-list">${['fix(auth): redirect_uri có dấu / cuối', 'feat(web): dark mode trang cài đặt', 'test(collector): quota reader'].map((c) => `<div class="s-row">${ic('branch', 'sm')}<span class="grow">${c}</span></div>`).join('')}</div>`,
    }),
    run: () => ({
      title: 'Run',
      body: `<div class="s-row">${ic('play')}<span class="grow">npm run dev<span class="sub">api, cổng 3000</span></span>${stHTML('working')}</div><button class="btn danger" style="width:100%;margin-top:8px" data-act="stub">${ic('stop', 'sm')}Dừng</button>`,
    }),
    settings: () => {
      const dark = isDark();
      return {
        title: 'Cài đặt',
        body: `<div class="seg" role="group" aria-label="Sáng hoặc tối"><button data-theme-set="light" aria-pressed="${!dark}">Sáng</button><button data-theme-set="dark" aria-pressed="${dark}">Tối</button></div>
          <label class="switch s-switch"><span>Mất mạng (thử)</span><input type="checkbox" id="sOffline"${ui.offline ? ' checked' : ''}><span class="sw" aria-hidden="true"></span></label>`,
      };
    },
    sess: () => {
      const s = cur();
      const subs = s.subs || [];
      const fi = lastFiles(s);
      const nf = fi >= 0 ? s.msgs[fi].files.length : 0;
      return {
        title: s.name,
        body: `${subs.length ? `<div class="s-label">Subagent</div><div class="s-list">${subs.map(([n, d, st]) => `<div class="s-row">${ic('sub')}<span class="grow">${esc(n)}<span class="sub">${esc(d)}</span></span>${stHTML(st)}</div>`).join('')}</div><div class="s-sep"></div>` : ''}
          <div class="s-list">
            ${s.agent !== 'shell' ? `<button class="s-row" data-sheet="usage">${ic('gauge')}<span class="grow">Context và quota</span><span class="end">${s.ctx || 0}%${ic('chev-right', 'sm')}</span></button>` : ''}
            ${nf ? `<button class="s-row" data-sheet="sfiles">${ic('pencil')}<span class="grow">File đã sửa</span><span class="end">${nf}${ic('chev-right', 'sm')}</span></button>` : ''}
            <button class="s-row" data-sheet="git">${ic('branch')}<span class="grow">Git</span><span class="end">${ic('chev-right', 'sm')}</span></button>
            <button class="s-row danger" data-act="stub">${ic('x')}<span class="grow">Đóng session</span></button>
          </div>`,
      };
    },
    sfiles: () => {
      const s = cur();
      const i = lastFiles(s);
      const files = i >= 0 ? s.msgs[i].files : [];
      return {
        title: 'File đã sửa',
        body: `<div class="s-list">${files.map((f, k) => `<button class="s-row" data-diff="${i}:${k}">${ic('file')}<span class="grow">${esc(f.p)}</span><span class="end">${statHTML(f.a, f.d)}</span></button>`).join('')}</div>`,
      };
    },
    diff: () => {
      const s = cur();
      const [i, k] = ui.diff;
      const files = s.msgs[i].files;
      const f = files[k];
      const name = f.p.split('/').pop();
      return {
        title: name,
        tall: true,
        body: `<div class="diff-head"><span class="path">${esc(f.p)}</span>${statHTML(f.a, f.d)}</div><div class="diff" role="region" aria-label="Diff ${esc(f.p)}">${diffBody(f)}</div>
          ${files.length > 1 ? `<div class="diff-nav"><button class="btn" data-diff="${i}:${k - 1}"${k === 0 ? ' disabled' : ''}>${ic('chev-left', 'sm')}File trước</button><button class="btn" data-diff="${i}:${k + 1}"${k === files.length - 1 ? ' disabled' : ''}>File sau${ic('chev-right', 'sm')}</button></div>` : ''}`,
      };
    },
  };

  function openSheet(key, push) {
    const def = SHEETS[key]();
    if (sheet.hidden) {
      sheetReturn = document.activeElement;
      ui.sheetStack = [];
    }
    if (push) ui.sheetStack.push(push);
    $('#sheetTitle').textContent = def.title;
    $('#sheetBody').innerHTML = def.body;
    $('#sheetBody').scrollTop = 0;
    $('#sheetBack').hidden = !ui.sheetStack.length;
    sheet.dataset.key = key;
    sheet.classList.toggle('tall', !!def.tall);
    scrim.hidden = false;
    sheet.hidden = false;
    const f = key === 'diff' ? $('#sheetClose') : sheet.querySelector('.sheet-body button:not(:disabled), .sheet-body input') || $('#sheetClose');
    f.focus({ preventScroll: true });
  }

  function closeSheet() {
    if (sheet.hidden) return;
    sheet.hidden = true;
    scrim.hidden = true;
    ui.sheetStack = [];
    if (sheetReturn && document.contains(sheetReturn)) sheetReturn.focus({ preventScroll: true });
  }

  function openDiff(i, k, from) {
    ui.diff = [i, k];
    openSheet('diff', from);
  }

  function isDark() {
    const t = document.documentElement.dataset.theme;
    return t ? t === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  }

  function setTheme(t) {
    document.documentElement.dataset.theme = t;
    try { localStorage.setItem('orrery.chat.theme', t); } catch (e) {}
    syncPanel();
    if (!sheet.hidden && sheet.dataset.key === 'settings') openSheet('settings');
  }

  function setDevice(d) {
    device.dataset.device = d;
    if (d === 'ipad') {
      if (!ui.open) openChat('s1');
      $('#home').inert = false;
      $('#chat').inert = false;
    } else if (!ui.open) {
      $('#chat').inert = true;
    } else {
      $('#home').inert = true;
    }
    fit();
    syncPanel();
  }

  function fit() {
    const narrow = innerWidth <= 520;
    if (narrow) { device.style.removeProperty('--fit'); device.style.removeProperty('--sx'); device.style.removeProperty('--sy'); return; }
    const ipad = device.dataset.device === 'ipad';
    const w = ipad ? 1204 : 414;
    const h = ipad ? 844 : 868;
    const panel = innerWidth > 1120 ? 290 : 0;
    const availW = ipad ? innerWidth - panel - 48 : innerWidth - 32;
    const availH = innerHeight - (ipad && innerWidth > 1000 ? 120 : 32);
    const k = Math.min(1, availW / w, availH / h);
    device.style.setProperty('--fit', k.toFixed(3));
    device.style.setProperty('--sx', ipad ? -panel / 2 + 'px' : '0px');
    device.style.setProperty('--sy', ipad && innerWidth > 1000 ? '44px' : '0px');
  }
  addEventListener('resize', fit);
  fit();

  let panelGo = 'home';
  function syncPanel() {
    const dark = isDark();
    $$('#pTheme button').forEach((b) => b.setAttribute('aria-pressed', String((b.dataset.themeSet === 'dark') === dark)));
    $$('#pDevice button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.deviceSet === device.dataset.device)));
    const s = cur();
    const st = s ? (s.state === 'done' || s.state === 'error' ? 'idle' : s.state) : '';
    $$('#pAgent button').forEach((b) => {
      b.setAttribute('aria-pressed', String(!!s && s.agent !== 'shell' && b.dataset.agent === st));
    });
    if (!ui.open) panelGo = 'home';
    $$('#panel [data-go]').forEach((b) => b.setAttribute('aria-current', String(b.dataset.go === panelGo)));
  }

  function resetSession(id) {
    const i = sessions.findIndex((s) => s.id === id);
    const old = sessions[i];
    clearTimeout(old.timer);
    old.msgs.forEach((m) => clearTimeout(m._t));
    const fresh = clone(SEED.find((s) => s.id === id));
    prep(fresh);
    sessions[i] = fresh;
  }

  function scrollToFiles() {
    const el = msgs.querySelector('.files');
    if (el) msgs.scrollTop = Math.max(0, el.offsetTop - 80);
  }

  function go(k) {
    closeSheet();
    panelGo = k;
    if (k === 'home') {
      if (device.dataset.device === 'ipad') setDevice('phone');
      closeChat();
    } else if (k === 'perm') { resetSession('s1'); openChat('s1', { term: false }); }
    else if (k === 'menu') { resetSession('s2'); openChat('s2', { term: false }); }
    else if (k === 'working') { resetSession('s3'); openChat('s3', { term: false }); }
    else if (k === 'files') { resetSession('s1'); openChat('s1', { term: false }); scrollToFiles(); }
    else if (k === 'diff') { resetSession('s1'); openChat('s1', { term: false }); scrollToFiles(); openDiff(3, 0); }
    else if (k === 'usage') { openChat('s1', { term: false }); openSheet('usage'); }
    else if (k === 'term') { openChat('s1', { term: true }); }
    else if (k === 'shell') { openChat('s5'); }
    else if (k === 'new') { closeChat(); ui.placeOpen = false; openSheet('new'); }
    renderList();
    syncPanel();
  }

  let newCount = 0;
  function createSession(agent, project) {
    newCount++;
    const p = project || ui.newProject;
    const s = {
      id: `n${newCount}`, m: ui.machine, agent, project: p,
      name: agent === 'shell' ? `${p} shell` : `${AGENT[agent]} mới`,
      state: 'idle', time: 'vừa xong', ago: 0, model: agent === 'codex' ? 'gpt-5-codex' : 'Opus 4.5', ctx: 0, subs: [],
      msgs: [],
      last: agent === 'shell' ? `~/work/${p}` : 'Phiên mới',
      term: agent === 'shell' ? [`<span class="g">~/work/${p}</span> <span class="dim">$</span>`] : [`<span class="o">✻</span> <b>${AGENT[agent]}</b> <span class="dim">~/work/${p}</span>`, '', '<span class="dim">&gt;</span>'],
    };
    s.termMode = agent === 'shell';
    touch(s);
    sessions.unshift(s);
    closeSheet();
    openChat(s.id);
    if (agent !== 'shell') setTimeout(() => input.focus({ preventScroll: true }), 330);
    else setTimeout(() => $('#termInput').focus({ preventScroll: true }), 330);
  }

  list.addEventListener('click', (e) => {
    const b = e.target.closest('[data-open]');
    if (b) { panelGo = ''; openChat(b.dataset.open); }
  });
  $('#quotaLine').addEventListener('click', (e) => { if (e.target.closest('[data-quota]')) openSheet('quota'); });
  $('#backBtn').addEventListener('click', closeChat);
  $('#searchBtn').addEventListener('click', () => {
    const box = $('#searchBox');
    box.hidden = !box.hidden;
    $('#searchBtn').setAttribute('aria-expanded', String(!box.hidden));
    if (!box.hidden) $('#searchInput').focus();
    else { ui.query = ''; $('#searchInput').value = ''; renderList(); }
  });
  $('#searchInput').addEventListener('input', (e) => { ui.query = e.target.value; renderList(); });
  $('#machineBtn').addEventListener('click', () => openSheet('machine'));
  $('#moreBtn').addEventListener('click', () => openSheet('more'));
  $('#newBtn').addEventListener('click', () => { ui.placeOpen = false; openSheet('new'); });
  $('#sessMoreBtn').addEventListener('click', () => openSheet('sess'));
  $('#ctxBtn').addEventListener('click', () => openSheet('usage'));
  $('#plusBtn').addEventListener('click', () => openSheet('plus'));
  $('#termBtn').addEventListener('click', () => {
    const s = cur();
    if (!s) return;
    s.termMode = !s.termMode;
    renderChat(false);
  });

  function stopMode() {
    const s = cur();
    return !!s && s.state === 'working' && !input.value.trim();
  }

  $('#composeForm').addEventListener('submit', (e) => {
    e.preventDefault();
    if (stopMode()) { stop(cur()); return; }
    send(input.value);
    input.value = '';
    grow();
    syncSend();
  });
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing && matchMedia('(pointer: fine)').matches) {
      e.preventDefault();
      if (input.value.trim()) $('#composeForm').requestSubmit();
    }
  });
  function grow() { input.style.height = 'auto'; input.style.height = Math.min(input.scrollHeight, 140) + 'px'; }
  function syncSend() {
    const b = $('#sendBtn');
    const stopping = stopMode();
    b.classList.toggle('stopping', stopping);
    b.disabled = !stopping && !input.value.trim();
    b.setAttribute('aria-label', stopping ? 'Dừng' : 'Gửi');
    b.innerHTML = ic(stopping ? 'stop' : 'arrow-up');
  }
  input.addEventListener('input', () => { grow(); syncSend(); });
  $('#chips').addEventListener('click', (e) => {
    const c = e.target.closest('[data-chip]');
    if (!c) return;
    send(c.dataset.chip);
    syncSend();
  });
  syncSend();

  msgs.addEventListener('click', (e) => {
    const b = e.target.closest('[data-act]');
    if (!b) return;
    const s = cur();
    const i = Number(b.dataset.i);
    const act = b.dataset.act;
    if (act === 'tools') {
      const m = s.msgs[i];
      m.open = !m.open;
      b.setAttribute('aria-expanded', String(m.open));
      b.nextElementSibling.hidden = !m.open;
    } else if (act === 'file') openDiff(i, Number(b.dataset.f));
    else if (act === 'pick') pick(s, i, b.dataset.v);
    else if (act === 'undo') undo(s, i);
    else if (act === 'copy') toast('Đã chép');
    else if (act === 'resend') {
      if (ui.offline) { toast('Chưa có mạng'); return; }
      s.msgs[i].pending = false;
      if (s.state === 'working') renderChat(true);
      else runAgent(s);
    }
  });

  const termKeys = $('.keys');
  termKeys.addEventListener('click', (e) => {
    const b = e.target.closest('[data-key]');
    if (!b) return;
    const s = cur();
    const k = b.dataset.key;
    if (k === 'Ctrl') { b.setAttribute('aria-pressed', String(b.getAttribute('aria-pressed') !== 'true')); return; }
    b.classList.add('hit');
    setTimeout(() => b.classList.remove('hit'), 140);
    if (!s) return;
    if (k === 'Esc' && s.state === 'working') { stop(s); return; }
    if (k === 'Enter') { s.term.push(s.agent === 'shell' ? '<span class="dim">$</span>' : ''); renderTerm(s); }
  });
  $('#termForm').addEventListener('submit', (e) => {
    e.preventDefault();
    const s = cur();
    const v = $('#termInput').value.trim();
    if (!s || !v) return;
    if (ui.offline) { toast('Chưa gửi, đang nối lại'); return; }
    if (s.agent === 'shell') {
      s.term.push(`<span class="dim">$</span> ${esc(v)}`);
      s.term.push(v === 'ls' ? 'package.json  src  test  tsconfig.json' : `<span class="dim">${esc(v.split(' ')[0])}: xong</span>`);
      s.last = v;
      touch(s);
    } else {
      s.term.push(`<span class="dim">&gt;</span> ${esc(v)}`);
      s.msgs.push({ t: 'me', text: v });
      if (s.state !== 'working') runAgent(s);
    }
    $('#termInput').value = '';
    renderTerm(s);
    renderList();
  });

  scrim.addEventListener('click', closeSheet);
  $('#sheetClose').addEventListener('click', closeSheet);
  $('#sheetBack').addEventListener('click', () => { const k = ui.sheetStack.pop(); if (k) openSheet(k); });
  sheet.addEventListener('click', (e) => {
    const t = e.target;
    const df = t.closest('[data-diff]');
    if (df) {
      const [i, k] = df.dataset.diff.split(':').map(Number);
      const from = sheet.dataset.key === 'diff' ? null : sheet.dataset.key;
      openDiff(i, k, from);
      return;
    }
    const sh = t.closest('[data-sheet]');
    if (sh) { openSheet(sh.dataset.sheet, sheet.dataset.key); return; }
    const mach = t.closest('[data-machine]');
    if (mach) { ui.machine = mach.dataset.machine; closeSheet(); closeChat(); renderList(); return; }
    const nw = t.closest('[data-new]');
    if (nw) { createSession(nw.dataset.new); return; }
    if (t.closest('[data-newshell]')) { const s = cur(); createSession('shell', s ? s.project : null); return; }
    const pr = t.closest('[data-proj]');
    if (pr) { ui.newProject = pr.dataset.proj; ui.placeOpen = false; openSheet('new'); $('[data-act="place"]', sheet).focus(); return; }
    const pl = t.closest('[data-act="place"]');
    if (pl) { ui.placeOpen = !ui.placeOpen; openSheet('new'); $('[data-act="place"]', sheet).focus(); return; }
    const th = t.closest('[data-theme-set]');
    if (th) { setTheme(th.dataset.themeSet); return; }
    if (t.closest('[data-act="stub"]')) toast('Chỉ là mockup');
  });
  sheet.addEventListener('change', (e) => { if (e.target.id === 'sOffline') setOffline(e.target.checked); });

  $('#panel').addEventListener('click', (e) => {
    const g = e.target.closest('[data-go]');
    if (g) { go(g.dataset.go); return; }
    const a = e.target.closest('[data-agent]');
    if (a) { setAgentState(a.dataset.agent); return; }
    const th = e.target.closest('[data-theme-set]');
    if (th) { setTheme(th.dataset.themeSet); return; }
    const d = e.target.closest('[data-device-set]');
    if (d) setDevice(d.dataset.deviceSet);
  });
  $('#pOffline').addEventListener('change', (e) => setOffline(e.target.checked));

  document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    if (!sheet.hidden) { closeSheet(); return; }
    const s = cur();
    if (s && s.state === 'working' && !s.termMode) { e.preventDefault(); stop(s); return; }
    if (ui.open && document.activeElement !== input) closeChat();
  });
  sheet.addEventListener('keydown', (e) => {
    if (e.key !== 'Tab') return;
    const f = $$('button:not([hidden]):not(:disabled), input', sheet).filter((x) => x.offsetParent !== null);
    if (!f.length) return;
    const first = f[0];
    const last = f[f.length - 1];
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  });

  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', syncPanel);
  if (reduce.matches) document.documentElement.classList.add('reduce');

  const q = new URLSearchParams(location.search);
  if (q.get('device') === 'ipad') setDevice('ipad');
  if (q.get('go')) go(q.get('go'));
  if (q.get('offline') === '1') setOffline(true);

  renderList();
  syncPanel();
})();
