const P={
monitor:'<path d="M3 12h4l3-8 4 16 3-8h4"/>',
layers:'<path d="M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z"/>',
settings:'<path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15" cy="6" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="18" r="2"/>',
terminal:'<path d="M4 6l5 5-5 5M12 18h8"/>',
file:'<path d="M6 3h8l5 5v13H6zM14 3v5h5"/>',
lock:'<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 018 0v3"/>',
wifi:'<path d="M3 9a14 14 0 0118 0M6 12.5a9.5 9.5 0 0112 0M9 16a5 5 0 016 0"/><circle cx="12" cy="19" r="1"/>',
wifioff:'<path d="M3 9a14 14 0 013-2M6 12.5a9.5 9.5 0 012-1.5M9 16a5 5 0 016 0M4 4l16 16"/><circle cx="12" cy="19" r="1"/>',
cloud:'<path d="M7 18a4 4 0 01-.5-8 5.5 5.5 0 0110.7 1.5A3.3 3.3 0 0117 18z"/>',
qr:'<rect x="4" y="4" width="6" height="6"/><rect x="14" y="4" width="6" height="6"/><rect x="4" y="14" width="6" height="6"/><path d="M14 14h2v2h-2zM18 14h2v6h-4M14 18h2"/>',
plus:'<path d="M12 5v14M5 12h14"/>',
back:'<path d="M15 5l-7 7 7 7"/>',
chev:'<path d="M9 5l7 7-7 7"/>',
down:'<path d="M5 9l7 7 7-7"/>',
close:'<path d="M6 6l12 12M18 6L6 18"/>',
refresh:'<path d="M20 11a8 8 0 10-2.3 5.7M20 4v7h-7"/>',
branch:'<circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="8" r="2"/><path d="M6 7v10M18 10c0 5-8 3-12 7"/>',
loader:'<path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8"/>',
clock:'<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
moon:'<path d="M20 14A8 8 0 019.5 4 8 8 0 1020 14z"/>',
check:'<path d="M5 12.5l4.5 4.5L19 7"/>',
alert:'<path d="M12 3l10 18H2z"/><path d="M12 10v4M12 17.5v.5"/>',
bell:'<path d="M6 16v-5a6 6 0 0112 0v5l2 2H4zM10 21h4"/>',
finger:'<path d="M12 11v3M8 14a4 4 0 018 0M6 12a6 6 0 0112 0M5 20c1-2 1-4 1-6M19 14c0 3-1 5-2 6M12 17c0 2-.5 3-1 4"/>',
send:'<path d="M4 12l16-8-6 16-3-7z"/>',
more:'<circle cx="5" cy="12" r="1.6" class="f"/><circle cx="12" cy="12" r="1.6"/><circle cx="19" cy="12" r="1.6"/>',
folder:'<path d="M3 7h6l2 2h10v10H3z"/>',
diff:'<path d="M7 5v8M3 9h8M13 17h8"/>',
laptop:'<rect x="4" y="5" width="16" height="11" rx="1.5"/><path d="M2 19h20"/>',
spark:'<path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6L5.6 18.4"/>',
hex:'<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/>',
orbit:'<circle cx="12" cy="12" r="2.5"/><path d="M12 5a7 7 0 107 7"/><circle cx="19" cy="12" r="1.6"/>',
stop:'<rect x="6" y="6" width="12" height="12" rx="2"/>',
globe:'<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"/>',
copy:'<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 00-2-2H6a2 2 0 00-2 2v8a2 2 0 002 2h2"/>',
trash:'<path d="M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13"/>',
shield:'<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/>',
grid:'<rect x="4" y="4" width="7" height="7" rx="1"/><rect x="13" y="4" width="7" height="7" rx="1"/><rect x="4" y="13" width="7" height="7" rx="1"/><rect x="13" y="13" width="7" height="7" rx="1"/>',
bat:'<rect x="3" y="8" width="16" height="9" rx="2"/><path d="M21 11v3"/>',
undo:'<path d="M9 14L4 9l5-5M4 9h10a6 6 0 010 12h-3"/>',
pause:'<path d="M9 5v14M15 5v14"/>'
};
const ic=(n,c='')=>`<svg class="ic ${c}" viewBox="0 0 24 24" aria-hidden="true">${P[n]}</svg>`;
const ST={working:['loader','Đang chạy'],waiting:['clock','Chờ bạn'],idle:['moon','Rảnh'],done:['check','Xong'],error:['alert','Lỗi'],stale:['wifioff','Mất kết nối'],lan:['wifi','LAN · 12 ms'],relay:['cloud','Relay · 84 ms'],conn:['refresh','Đang nối'],off:['wifioff','Offline']};
const chip=(s,t)=>`<span class="chip st-${s}">${ic(ST[s][0],s==='working'?'spin':'')}${t||ST[s][1]}</span>`;
const AG={claude:['spark','Claude'],codex:['hex','Codex'],shell:['terminal','Shell']};
const agent=k=>`<span class="agent a-${k}">${ic(AG[k][0])}</span>`;
const sb=()=>`<div class="sb"><span>14:32</span><span class="r">${ic('wifi')}${ic('bat')}</span></div>`;
const tbar=(title,o={})=>`<div class="top ${o.big?'big':''}">${o.back?`<span class="ib">${ic('back')}</span>`:''}<h1>${title}</h1>${o.right||''}</div>`;
const nav=a=>`<div class="nav">${[['monitor','Giám sát'],['layers','Workspace'],['settings','Cài đặt']].map(([i,l],n)=>`<div class="${a===n?'on':''}">${ic(i)}${l}${n===0&&o_badge?`<span class="bd">1</span>`:''}</div>`).join('')}</div>`;
let o_badge=true;
const shell=(inner,navIdx,extra='')=>`<div class="scr">${sb()}${inner}${navIdx===undefined?'':nav(navIdx)}</div>${extra}<div class="gest"><i></i></div>`;
const sesCard=(a,title,proj,br,st,tool,t)=>`<div class="card ses a-${a}">${agent(a)}<div class="mid"><div class="row"><div class="t1 ell grow">${title}</div>${chip(st)}</div><div class="meta" style="margin-top:2px">${ic('branch')}<span class="ell mono">${br}</span><span style="margin-left:auto;flex:none">${t}</span></div>${tool?`<div class="tool ell">${tool}</div>`:''}</div></div>`;
const machine=(n,os,conn,stats,sel)=>`<div class="card stack" style="gap:8px"><div class="row"><span class="agent" style="--ac:var(--accent)">${ic('laptop')}</span><div class="grow"><div class="t1 ell">${n}</div><div class="t2">${os}</div></div>${conn}</div>${stats?`<div class="row" style="flex-wrap:wrap;gap:6px">${stats}</div>`:''}</div>`;
const mini=(s,n)=>`<span class="chip st-${s}">${ic(ST[s][0])}${n} ${ST[s][1].toLowerCase()}</span>`;
const mkTerm=(l)=>`<div class="term">${l}</div>`;
const TERM=`<span class="d">$ claude --resume 4f2a</span>
<span class="o">●</span> Đọc <span class="b">apps/mobile/lib/ui/terminal/key_bar.dart</span>
<span class="o">●</span> Đọc <span class="b">apps/mobile/lib/ui/theme/orrery_theme.dart</span>
<span class="o">●</span> Sửa <span class="b">docs/design/v97-mobile-ui/index.html</span>
  <span class="g">+ 214</span> <span class="r">− 0</span>
<span class="o">●</span> Bash(npm run typecheck -w @ccm/web)
  <span class="d">⎿ </span><span class="g">0 lỗi</span>, 2 cảnh báo

<span class="y">✻</span> Đang dựng khung… <span class="d">(2m 14s · esc)</span>

<span class="d">╭──────────────────────────────╮</span>
<span class="d">│</span> <span class="m">&gt;</span> <span class="cur"></span>
<span class="d">╰──────────────────────────────╯</span>`;
const KEYS=['Esc','Tab','Ctrl','Alt','↑','↓','←','→','/','~','|'];
const keyRow=()=>`<div class="keys">${KEYS.map((k,i)=>`<div class="${k==='Ctrl'?'on':''}">${k}</div>`).join('')}</div>`;

const S={};
const def=(id,title,group,fn,note)=>S[id]={id,title,group,fn,note};

def('home-a','Home A — danh sách máy → vào máy','nav',()=>shell(
tbar('Máy của tôi',{big:1,right:`<span class="ib s">${ic('settings')}</span>`})+
`<div class="body pad stack" style="padding-top:6px">
<div class="lbl in" style="padding-top:0">Đang kết nối · 2</div>
${machine('DESKTOP-DAUPH','Windows 11 · Orrery 2.3.0',chip('lan'),mini('working',2)+mini('waiting',1)+mini('idle',2))}
${machine('MacBook-Pro-Vu','macOS 15 · Orrery 2.3.0',chip('relay'),mini('working',1)+mini('idle',1))}
<div class="lbl in">Bản trùng tên · 2</div>
<div class="card stack" style="gap:0;padding:4px 12px">
<div class="row" style="min-height:48px"><span class="agent" style="--ac:var(--stale)">${ic('laptop')}</span><div class="grow"><div class="t1 ell" style="font-size:14px">DESKTOP-DAUPH</div><div class="t2">Ghép 02/11 · đã thu hồi</div></div>${chip('off','Đã thu hồi')}</div>
<div class="row" style="min-height:48px;border-top:1px solid var(--line)"><span class="agent" style="--ac:var(--stale)">${ic('laptop')}</span><div class="grow"><div class="t1 ell" style="font-size:14px">DESKTOP-DAUPH</div><div class="t2">Ghép 28/10 · 9 ngày trước</div></div>${chip('off','Offline')}</div>
<div class="row" style="min-height:48px;border-top:1px solid var(--line);color:var(--accent-ink);font-weight:600;font-size:13px">${ic('trash')}Dọn 2 bản cũ</div></div>
</div><div class="fab">${ic('plus')}Ghép máy</div>`));

def('home-b','Home B — vào thẳng máy, đổi máy bằng sheet','nav',()=>shell(
tbar('<span class="pill" style="height:36px;font-size:15px">'+ic('laptop')+'DESKTOP-DAUPH'+ic('down')+'</span>',{right:chip('lan')})+
`<div class="body pad stack" style="padding-top:8px">
<div class="row"><div class="seg grow"><div class="on">Tất cả 5</div><div>Đang chạy 2</div><div>Chờ bạn 1</div></div></div>
${sesCard('claude','Dựng mockup UI mobile','claude-code-monitor','feat/v97-mobile','working','▸ Sửa docs/design/index.html','2p 14g')}
${sesCard('codex','Sửa lỗi pty.overflow','claude-code-monitor','fix/pty-overflow','waiting','▸ Xin chạy: npm test','6p')}
${sesCard('shell','npm run dev','claude-code-monitor','feat/v97-mobile','idle','','41p')}
</div>`,0,`<div class="scrim"></div><div class="sheet"><div class="grab"></div><h2>Chọn máy</h2><div class="stack" style="gap:8px">
<div class="card row" style="border-color:var(--accent)"><span class="agent" style="--ac:var(--accent)">${ic('laptop')}</span><div class="grow"><div class="t1">DESKTOP-DAUPH</div><div class="t2">Windows 11 · 5 session</div></div>${chip('lan')}</div>
<div class="card row"><span class="agent" style="--ac:var(--accent)">${ic('laptop')}</span><div class="grow"><div class="t1">MacBook-Pro-Vu</div><div class="t2">macOS 15 · 2 session</div></div>${chip('relay')}</div>
<div class="row" style="justify-content:space-between;margin-top:4px"><span class="btn t sm">${ic('plus')}Ghép máy mới</span><span class="btn t sm">Quản lý 2 bản cũ</span></div></div></div>`),
'Sheet đổi máy hiện trên Giám sát; thanh dưới luôn có.');

def('pair-qr','Ghép máy — quét QR','pair',()=>shell(
tbar('Ghép máy',{back:1})+
`<div class="body pad stack" style="gap:14px;padding-top:4px">
<div class="seg"><div class="on">${'Quét QR'}</div><div>Nhập mã</div></div>
<div class="qr">${[[14,14],[0,0]].length?'':''}
<svg viewBox="0 0 100 100" style="position:absolute;inset:0;width:100%;height:100%"><g fill="#2a2a32">${Array.from({length:441},(_,i)=>{const x=i%21,y=(i/21)|0;if((x<8&&y<8)||(x>12&&y<8)||(x<8&&y>12))return '';const h=(x*73856093^y*19349663)>>>0;return h%100<48?'<rect x="'+(11+x*3.6)+'" y="'+(11+y*3.6)+'" width="3.2" height="3.2"/>':''}).join('')}</g><g fill="none" stroke="#3a3a44" stroke-width="2"><rect x="12" y="12" width="21" height="21"/><rect x="67" y="12" width="21" height="21"/><rect x="12" y="67" width="21" height="21"/></g><g stroke="#e8835d" stroke-width="2.4" fill="none" stroke-linecap="round"><path d="M10 24V12a2 2 0 012-2h12M76 10h12a2 2 0 012 2v12M90 76v12a2 2 0 01-2 2H76M24 90H12a2 2 0 01-2-2V76"/></g></svg></div>
<div style="text-align:center"><div class="t1">Đưa mã QR vào khung</div><div class="t2" style="margin-top:2px">Mở Orrery trên máy tính → Cài đặt → Điện thoại.</div></div>
<div class="banner" style="--c:var(--mod)">${ic('shield')}<div><b>Kết nối mã hoá đầu cuối</b><span>Điện thoại và máy dùng chung khoá Noise. Relay chỉ chuyển byte đã mã hoá.</span></div></div>
<div class="grow"></div>
<span class="btn s w">${ic('qr')}Nhập mã thủ công</span><div style="height:20px"></div></div>`));

def('pair-code','Ghép máy — nhập mã','pair',()=>shell(
tbar('Ghép máy',{back:1})+
`<div class="body pad stack" style="gap:14px;padding-top:4px">
<div class="seg"><div>Quét QR</div><div class="on">Nhập mã</div></div>
<div><div class="t1" style="margin-bottom:8px">Mã ghép trên máy tính</div>
<div class="row" style="gap:6px">${['7','K','3','F'].map(c=>`<div class="cell">${c}</div>`).join('')}<span class="t2">–</span>${['9','2'].map(c=>`<div class="cell">${c}</div>`).join('')}<div class="cell on"><span class="cur" style="width:2px;height:24px;background:var(--accent)"></span></div><div class="cell"></div></div>
<div class="t2" style="margin-top:6px">8 ký tự, còn hiệu lực 04:12.</div></div>
<div class="lbl in" style="padding-bottom:0">Máy tìm thấy trong mạng LAN</div>
${machine('DESKTOP-DAUPH','192.168.1.24 · Orrery 2.3.0',chip('lan','Cùng mạng'))}
<div class="row t2">${ic('loader','spin')}Đang tìm thêm máy qua mDNS…</div>
<div class="grow"></div><span class="btn p w">Ghép với DESKTOP-DAUPH</span><div style="height:20px"></div></div>`));

def('mon-a','Monitor A — thẻ session','monitor',()=>shell(
tbar('DESKTOP-DAUPH',{back:1,right:chip('lan')+'<span class="ib s">'+ic('refresh')+'</span>'})+
`<div class="body pad stack" style="padding-top:2px">
<div class="row" style="overflow:hidden;gap:6px"><span class="pill on">Tất cả 5</span><span class="pill">${ic('loader')}Đang chạy 2</span><span class="pill">${ic('clock')}Chờ bạn 1</span><span class="pill">Rảnh 2</span></div>
${sesCard('claude','Dựng mockup UI mobile','claude-code-monitor','feat/v97-mobile','working','▸ Sửa docs/design/index.html','2p 14g')}
${sesCard('codex','Sửa lỗi pty.overflow','claude-code-monitor','fix/pty-overflow','waiting','▸ Xin chạy: npm test -w @ccm/collector','6p')}
${sesCard('claude','Refactor AdsGate','WeJoy-Dumpling','main','working','▸ 3 sub-agent đang chạy','11p')}
${sesCard('shell','npm run dev','claude-code-monitor','feat/v97-mobile','idle','','41p')}
</div>`,0));

def('mon-b','Monitor B — nhóm theo trạng thái, dòng gọn','monitor',()=>{
const r=(a,t,s,tm,st)=>`<div class="li" style="min-height:56px;padding:8px 16px"><span style="color:var(--c)" class="st-${st}">${ic(ST[st][0],st==='working'?'spin':'')}</span>${agent(a)}<div class="grow"><div class="t1 ell" style="font-size:14px">${t}</div><div class="t2 ell mono">${s}</div></div><span class="t2" style="flex:none">${tm}</span></div>`;
const gh=(st,n)=>`<div class="lbl row st-${st}" style="color:var(--c);gap:6px;padding-bottom:4px">${ic(ST[st][0])}${ST[st][1]} · ${n}</div>`;
return shell(tbar('DESKTOP-DAUPH',{back:1,right:chip('lan')})+`<div class="body">
${gh('waiting',1)}<div class="grp" style="margin:0">${r('codex','Sửa lỗi pty.overflow','fix/pty-overflow','6p','waiting')}</div>
${gh('working',2)}<div class="grp" style="margin:0">${r('claude','Dựng mockup UI mobile','feat/v97-mobile','2p','working')}${r('claude','Refactor AdsGate','main · 3 sub-agent','11p','working')}</div>
${gh('idle',2)}<div class="grp" style="margin:0">${r('shell','npm run dev','feat/v97-mobile','41p','idle')}${r('codex','Review PR #482','review/pr-482','1g','idle')}</div>
</div>`,0)},'Ít cuộn hơn, thấy 6+ session một màn.');

def('detail','Chi tiết session','monitor',()=>shell(
tbar('Dựng mockup UI mobile',{back:1,right:`<span class="ib s">${ic('more')}</span>`})+
`<div class="body pad stack" style="padding-top:2px">
<div class="card stack" style="gap:10px"><div class="row">${agent('claude')}<div class="grow"><div class="t1">Claude</div><div class="t2">claude-code-monitor</div></div>${chip('working')}</div>
<div class="row" style="flex-wrap:wrap;gap:6px 16px;font-size:12px;color:var(--muted)"><span class="row" style="gap:4px">${ic('branch')}<span class="mono">feat/v97-mobile</span></span><span class="row" style="gap:4px">${ic('clock')}Bắt đầu 14:30</span><span class="row" style="gap:4px">${ic('laptop')}DESKTOP-DAUPH</span></div></div>
<div class="lbl in" style="padding-bottom:0">Sub-agent · 2</div>
<div class="card stack" style="gap:0;padding:2px 12px">
<div class="row" style="min-height:44px"><span class="st-working" style="color:var(--c)">${ic('loader','spin')}</span><div class="grow t1 ell" style="font-size:13px">Dựng khung điện thoại</div>${chip('working')}</div>
<div class="row" style="min-height:44px;border-top:1px solid var(--line)"><span class="st-done" style="color:var(--c)">${ic('check')}</span><div class="grow t1 ell" style="font-size:13px">Đọc token themes.css</div>${chip('done')}</div></div>
<div class="lbl in" style="padding-bottom:0">Hoạt động gần đây</div>
<div class="card stack mono" style="gap:6px;font-size:11.5px;color:var(--muted)">
<div class="row"><span style="color:var(--claude)">●</span><span class="grow ell">Sửa docs/design/v97-mobile-ui/src/style.css</span><span>14:32</span></div>
<div class="row"><span style="color:var(--claude)">●</span><span class="grow ell">Đọc apps/web/src/themes.css</span><span>14:31</span></div>
<div class="row"><span style="color:var(--claude)">●</span><span class="grow ell">Bash · node build.mjs</span><span>14:30</span></div></div>
<div class="grow"></div>
<div class="row"><span class="btn p grow">${ic('terminal')}Mở terminal</span><span class="btn s">${ic('send')}</span><span class="btn d">${ic('stop')}</span></div><div style="height:22px"></div></div>`));

def('ws-a','Workspace A — một pane, chọn pane bằng chip','workspace',()=>shell(
tbar('Workspace',{big:1,right:`<span class="ib s">${ic('plus')}</span>`})+
`<div class="tabs"><div class="on">Mobile UI</div><div>Collector</div><div>Ads</div><div>${ic('plus')}</div></div>
<div class="body pad stack" style="padding-top:12px">
<div class="row" style="overflow:hidden;gap:6px"><span class="pill on a-claude" style="--ac:var(--claude)"><span class="dot" style="background:var(--claude)"></span>Claude</span><span class="pill"><span class="dot" style="background:var(--codex)"></span>Codex</span><span class="pill"><span class="dot" style="background:var(--shell)"></span>Shell</span></div>
<div class="pc a-claude" style="padding-bottom:12px"><div class="row">${agent('claude')}<div class="grow"><div class="t1 ell">Dựng mockup UI mobile</div><div class="meta">${ic('branch')}<span class="mono">feat/v97-mobile</span></div></div>${chip('working')}</div>
<div class="pv" style="height:210px;margin-top:10px;font-size:11px;line-height:16px">${TERM.split('\n').slice(0,9).join('\n')}</div>
<div class="row" style="margin-top:10px"><span class="btn p grow sm">${ic('terminal')}Mở toàn màn hình</span><span class="btn s sm">${ic('send')}</span></div></div>
<div class="t2" style="text-align:center">Vuốt ngang để đổi pane · 3 pane trong tab</div></div>`,1));

def('ws-b','Workspace B — lưới Mission Control','workspace',()=>{
const c=(a,t,st,pv)=>`<div class="pc a-${a}" style="--ac:var(--${a})"><div class="row" style="gap:6px"><div class="grow t1 ell" style="font-size:13px">${t}</div></div><div style="margin-top:6px">${chip(st)}</div><div class="pv">${pv}</div></div>`;
return shell(tbar('Workspace',{big:1,right:`<span class="ib s">${ic('grid')}</span>`})+
`<div class="tabs"><div class="on">Mobile UI</div><div>Collector</div><div>Ads</div><div>${ic('plus')}</div></div>
<div class="body stack" style="padding-top:12px;gap:12px"><div class="row pad" style="overflow:hidden;gap:6px"><span class="pill on">Tất cả 6</span><span class="pill">${ic('loader')}2</span><span class="pill">${ic('clock')}1</span></div>
<div class="mc">${c('claude','Dựng mockup UI','working','● Sửa index.html\n  + 214  − 0\n✻ Đang dựng…')}${c('codex','Sửa pty.overflow','waiting','Xin chạy lệnh:\n npm test\n[y/n]')}${c('shell','npm run dev','idle','VITE ready 312 ms\n➜ Local :5173\n')}${c('claude','Refactor AdsGate','working','● Đọc AdsGate.kt\n● Sửa 3 tệp\n✻ Đang chạy…')}${c('codex','Review PR #482','idle','Đã duyệt 7 tệp\n0 vấn đề\n')}${c('shell','flutter test','error','00:41 +12 -1\nFAILED\nexit 1')}</div></div>
<div class="fab n">${ic('plus')}Pane mới</div>`,1)},'Một nhìn thấy mọi pane, đổi pane 1 chạm.');

def('term','Terminal — thanh phím phụ','terminal',()=>shell(
tbar('<span class="pill" style="height:36px;--ac:var(--claude)"><span class="dot" style="background:var(--claude)"></span>Claude · Mobile UI'+ic('down')+'</span>',{back:1,right:chip('working')})+
mkTerm(TERM)+keyRow()+`<div class="inp"><div class="f">Gõ vào pane…</div><div class="go">${ic('send')}</div></div>`));

def('term-pick','Terminal — chọn pane (sheet)','terminal',()=>shell(
tbar('<span class="pill" style="height:36px"><span class="dot" style="background:var(--claude)"></span>Claude · Mobile UI'+ic('down')+'</span>',{back:1,right:chip('working')})+
mkTerm(TERM)+keyRow()+`<div class="inp"><div class="f">Gõ vào pane…</div><div class="go">${ic('send')}</div></div>`,undefined,
`<div class="scrim"></div><div class="sheet"><div class="grab"></div><h2>Pane trong tab Mobile UI</h2><div class="stack" style="gap:8px">
<div class="card row a-claude" style="border-color:var(--claude)">${agent('claude')}<div class="grow"><div class="t1 ell">Dựng mockup UI mobile</div><div class="t2">Claude</div></div>${chip('working')}</div>
<div class="card row a-codex">${agent('codex')}<div class="grow"><div class="t1 ell">Sửa lỗi pty.overflow</div><div class="t2">Codex</div></div>${chip('waiting')}</div>
<div class="card row a-shell">${agent('shell')}<div class="grow"><div class="t1 ell">npm run dev</div><div class="t2">Shell</div></div>${chip('idle')}</div></div></div>`));

def('file','Xem file','viewer',()=>{
const L=[['<span class="k">import</span> <span class="s">\'package:flutter/material.dart\'</span>;',''],['',''],['<span class="k">class</span> <span class="ty">KeyBar</span> <span class="k">extends</span> <span class="ty">StatelessWidget</span> {',''],['  <span class="k">const</span> <span class="fn">KeyBar</span>({<span class="k">super</span>.key, <span class="k">required</span> <span class="k">this</span>.onKey});',''],['  <span class="k">final</span> <span class="ty">ValueChanged</span>&lt;<span class="ty">String</span>&gt; onKey;',''],['',''],['  <span class="ty">Widget</span> <span class="fn">build</span>(<span class="ty">BuildContext</span> context) {',''],['    <span class="k">final</span> c = <span class="ty">Theme</span>.<span class="fn">of</span>(context);',''],['    <span class="k">return</span> <span class="ty">SizedBox</span>(',''],['      height: <span class="nu">48</span>,',''],['      child: <span class="ty">ListView</span>(',''],['        scrollDirection: <span class="ty">Axis</span>.horizontal,',''],['        children: [',''],['          <span class="k">for</span> (<span class="k">final</span> k <span class="k">in</span> _keys) _Key(k, onKey),',''],['        ],',''],['      ),',''],['    );',''],['  }',''],['}','']];
return shell(tbar('key_bar.dart',{back:1,right:`<span class="ib s">${ic('copy')}</span>`})+
`<div class="pad" style="padding-bottom:8px"><div class="t2 mono ell">apps/mobile/lib/ui/terminal/key_bar.dart</div><div class="row" style="margin-top:8px"><div class="seg grow"><div class="on">${ic('file')}Tệp</div><div>${ic('diff')}Diff</div></div></div></div>
<div class="code">${L.map((l,i)=>`<div class="ln"><span class="n">${i+1}</span><span>${l[0]}</span></div>`).join('')}</div>
<div class="row pad" style="padding:8px 16px 28px;background:var(--surface);border-top:1px solid var(--line-strong)"><span class="t2 grow">Dart · 19 dòng · chỉ đọc</span><span class="chip st-done">${ic('check')}Đã lưu</span></div>`)});

def('diff','Xem diff','viewer',()=>{
const L=[['','h','@@ -12,6 +12,9 @@ class KeyBar'],['12',' ','    return SizedBox('],['13',' ','      height: 48,'],['14','d','-      child: Row('],['14','a','+      child: ListView('],['15','a','+        scrollDirection: Axis.horizontal,'],['16',' ','        children: ['],['17','d','-          _Key(\'Esc\', onKey),'],['17','a','+          for (final k in _keys) _Key(k, onKey),'],['18',' ','        ],'],['19',' ','      ),'],['','h','@@ -40,3 +43,8 @@'],['43','a','+  static const _keys = ['],['44','a','+    \'Esc\', \'Tab\', \'Ctrl\', \'Alt\','],['45','a','+  ];'],['46',' ','}']];
return shell(tbar('Diff',{back:1})+
`<div class="pad stack" style="gap:8px;padding-bottom:8px"><div class="row"><span class="chip st-done">${ic('plus')}12 thêm</span><span class="chip st-error">${ic('stop')}3 xoá</span><span class="t2" style="margin-left:auto">2 tệp</span></div>
<div class="row"><div class="seg grow"><div>${ic('file')}Tệp</div><div class="on">${ic('diff')}Diff</div></div></div>
<div class="t2 mono ell">key_bar.dart</div></div>
<div class="code">${L.map(l=>l[1]==='h'?`<div class="hunk">${l[2]}</div>`:`<div class="ln ${l[1]==='a'?'add':l[1]==='d'?'del':''}"><span class="n">${l[0]}</span><span>${l[2].replace(/</g,'&lt;')}</span></div>`).join('')}</div>`)});

def('settings','Cài đặt','settings',()=>{
const li=(i,t,sub,r)=>`<div class="li">${ic(i,'lead')}<div class="grow"><div style="font-size:14px;font-weight:500">${t}</div>${sub?`<div class="t2">${sub}</div>`:''}</div>${r||''}</div>`;
const sw=o=>`<span class="sw ${o?'on':''}"><i></i></span>`;
return shell(tbar('Cài đặt',{big:1})+`<div class="body" style="padding-top:0">
<div class="lbl">Bảo mật</div><div class="grp">${li('lock','Khoá ứng dụng','Vân tay hoặc khoá màn hình',sw(1))}
<div class="li" style="flex-direction:column;align-items:stretch;gap:8px">${`<div class="row">${ic('clock','lead')}<div class="grow" style="font-size:14px;font-weight:500">Khoá sau khi không dùng</div></div><div class="seg"><div>1 phút</div><div>15 phút</div><div class="on">1 giờ</div><div>4 giờ</div></div>`}</div>
${li('wifioff','Đóng kết nối khi khoá','Tránh lộ nội dung khi điện thoại bị mượn',sw(1))}</div>
<div class="lbl">Thông báo</div><div class="grp">${li('bell','Session chờ bạn','',sw(1))}${li('check','Session xong','',sw(1))}${li('alert','Lỗi hoặc thoát bất thường','',sw(0))}</div>
<div class="lbl">Giao diện</div><div class="grp">${li('globe','Ngôn ngữ','',`<span class="val">Tiếng Việt${ic('chev')}</span>`)}${li('moon','Chủ đề','',`<span class="val">Theo máy${ic('chev')}</span>`)}</div>
</div>`,2)});

def('lock','Màn khoá','security',()=>shell(`<div class="body" style="display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;padding:0 32px;text-align:center">
<svg viewBox="0 0 96 96" width="96" height="96" style="color:var(--accent)"><g fill="none" stroke="currentColor" stroke-width="2"><circle cx="48" cy="48" r="14"/><circle cx="48" cy="48" r="30" opacity=".55"/><circle cx="48" cy="48" r="44" opacity=".3"/></g><circle cx="48" cy="18" r="5" fill="var(--codex)"/><circle cx="78" cy="48" r="4" fill="var(--shell)"/><circle cx="48" cy="48" r="6" fill="currentColor"/></svg>
<h2 style="font-size:22px;font-weight:600;margin-top:12px">Orrery đang khoá</h2>
<p style="font-size:14px;color:var(--muted);max-width:270px">Mở khoá để xem session. Kết nối tới máy đã tạm đóng.</p>
<div style="height:28px"></div>
<span class="ib" style="width:76px;height:76px;border-radius:38px;background:var(--sel);color:var(--accent-ink);border:1px solid var(--accent)">${ic('finger').replace('class="ic "','class="ic" style="width:38px;height:38px"')}</span>
<div class="t1" style="margin-top:4px">Chạm cảm biến vân tay</div>
<span class="btn t">Dùng khoá màn hình</span></div>
<div class="t2" style="text-align:center;padding-bottom:36px;display:flex;justify-content:center;gap:6px;align-items:center">${ic('lock')}Tự khoá sau 1 giờ không dùng</div>`));

def('push','Thông báo push (Android)','security',()=>shell(`<div class="body" style="padding-top:20px"><div style="padding:0 20px 18px"><div style="font-size:56px;font-weight:300;letter-spacing:-.03em;line-height:60px">14:32</div><div class="t2" style="font-size:14px">Th 5, 12 tháng 11</div></div>
<div class="noti"><div class="h"><span class="si">${ic('orbit')}</span>Orrery · DESKTOP-DAUPH · bây giờ</div><div class="tt">Codex chờ bạn · fix/pty-overflow</div><div class="bd2">Xin chạy lệnh: npm test -w @ccm/collector</div><div class="ac"><b>Trả lời</b><b>Mở pane</b></div></div>
<div class="noti"><div class="h"><span class="si">${ic('orbit')}</span>Orrery · DESKTOP-DAUPH · 2 phút</div><div class="tt">Claude xong · Dựng mockup UI mobile</div><div class="bd2">Hoàn tất sau 12 phút 40 giây.</div><div class="ac"><b>Xem session</b></div></div>
<div class="noti"><div class="h"><span class="si" style="background:var(--error);color:#fff">${ic('alert')}</span>Orrery · MacBook-Pro-Vu · 9 phút</div><div class="tt">Shell lỗi · flutter test</div><div class="bd2">Thoát với mã 1 sau 41 giây.</div><div class="ac"><b>Mở terminal</b></div></div>
<div style="display:flex;justify-content:space-between;padding:6px 20px"><span class="t2">Thông báo im lặng</span><span class="t2" style="color:var(--accent-ink);font-weight:600">Xoá tất cả</span></div></div>`));

def('empty','Trạng thái trống — chưa có máy','states',()=>shell(tbar('Máy của tôi',{big:1})+`<div class="body" style="display:flex;align-items:center;justify-content:center"><div class="empty">
<svg viewBox="0 0 120 120" width="120" height="120" style="color:var(--accent)"><g fill="none" stroke="currentColor" stroke-width="2"><circle cx="60" cy="60" r="16"/><circle cx="60" cy="60" r="36" opacity=".5"/><circle cx="60" cy="60" r="54" opacity=".25"/></g><circle cx="60" cy="24" r="6" fill="var(--codex)"/><circle cx="96" cy="60" r="5" fill="var(--shell)"/><circle cx="60" cy="60" r="7" fill="currentColor"/></svg>
<h2>Chưa có máy nào</h2><p>Mở Orrery trên máy tính, vào Cài đặt → Điện thoại rồi quét mã QR.</p>
<div style="height:10px"></div><span class="btn p">${ic('qr')}Ghép máy mới</span><span class="btn t">Nhập mã thủ công</span></div></div>`));

def('offline','Offline — mất kết nối','states',()=>shell(
tbar('DESKTOP-DAUPH',{back:1,right:chip('conn')})+
`<div class="body pad stack" style="padding-top:2px">
<div class="banner" style="--c:var(--warning)">${ic('wifioff')}<div class="grow"><b>Mất kết nối tới DESKTOP-DAUPH</b><span>Đang nối lại qua relay. Dữ liệu dưới đây là lúc 14:29.</span></div><span class="btn t sm" style="padding:0 6px;height:28px">Thử lại</span></div>
<div style="opacity:.62" class="stack">
${sesCard('claude','Dựng mockup UI mobile','claude-code-monitor','feat/v97-mobile','stale','▸ Sửa docs/design/index.html','2p')}
${sesCard('codex','Sửa lỗi pty.overflow','claude-code-monitor','fix/pty-overflow','stale','','6p')}
${sesCard('shell','npm run dev','claude-code-monitor','feat/v97-mobile','stale','','41p')}</div></div>`,0));

def('error','Lỗi — mã ghép hết hạn','states',()=>shell(tbar('Ghép máy',{back:1})+
`<div class="body pad stack" style="gap:14px;padding-top:4px"><div class="seg"><div>Quét QR</div><div class="on">Nhập mã</div></div>
<div><div class="t1" style="margin-bottom:8px">Mã ghép trên máy tính</div><div class="row" style="gap:6px">${['7','K','3','F'].map(c=>`<div class="cell bad">${c}</div>`).join('')}<span class="t2">–</span>${['9','2','Q','X'].map(c=>`<div class="cell bad">${c}</div>`).join('')}</div></div>
<div class="banner" style="--c:var(--error)">${ic('alert')}<div><b>Mã ghép đã hết hạn</b><span>Mã chỉ dùng được 5 phút. Tạo mã mới trên máy tính rồi nhập lại.</span></div></div>
<div class="banner" style="--c:var(--warning)">${ic('cloud')}<div><b>Relay không phản hồi (timeout 5s)</b><span>Thử lại hoặc nối cùng mạng Wi-Fi với máy tính để dùng LAN.</span></div></div>
<div class="grow"></div><span class="btn p w">${ic('refresh')}Thử lại</span><span class="btn s w">Quét QR thay thế</span><div style="height:12px"></div></div>`));

window.S=S;window.ic=ic;window.chip=chip;
