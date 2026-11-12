const q=new URLSearchParams(location.search);
const root=document.getElementById('root');
const phone=(id,t)=>`<div class="phone" data-theme="${t}" id="p-${id}-${t}">${S[id].fn()}</div>`;
const fig=(id,t)=>`<figure class="fig"><figcaption>${S[id].title.split(' — ')[0]} · ${t==='light'?'Sáng (orrery-light)':'Tối (galaxy)'}</figcaption>${phone(id,t)}</figure>`;
const both=id=>`<div class="pair">${fig(id,'light')}${fig(id,'dark')}</div>`;

if(q.get('s')){
  document.body.classList.add('solo');
  root.innerHTML=phone(q.get('s'),q.get('t')||'light');
}else{
const grp=(title,tags,ids,opt)=>`<div class="scr-grp"><h3>${title}${tags||''}</h3>${opt?`<div class="opt">${opt}</div>`:''}${ids.map(both).join('<div style="height:28px"></div>')}</div>`;
const ab=(a,b,c)=>`<b>${a}</b><p>${b}</p><div class="pros">${c}</div>`;
const pc=(ok,no)=>(ok?`<span class="ok"><b>Ưu:</b> ${ok}</span>`:'')+(no?`<span class="no"><b>Nhược:</b> ${no}</span>`:'');
const A='<span class="tag">Phương án A</span>',B='<span class="tag">Phương án B</span>';
const navHtml=[['screens','Màn hình'],['components','Component'],['motion','Motion'],['tokens','Token → Flutter'],['questions','Cần chọn']].map(x=>`<a href="#${x[0]}">${x[1]}</a>`).join('');
root.innerHTML=`
<nav class="pn">${navHtml}</nav>
<div class="wrap">
<header class="hd"><h1>Orrery Remote — mockup UI + motion</h1>
<p>Bản mockup cho WP-28 (chưa code Flutter). Khung 360×800 dp, mỗi màn có sáng (<code>orrery-light</code>) và tối (<code>galaxy</code>), màu / radius lấy từ token desktop. Phương án A / B đặt cạnh nhau để chủ nhân chọn. Mở thẳng bằng trình duyệt, chạy offline.</p></header>

<section class="sc" id="screens"><h2>Màn hình</h2><p class="lead">Dữ liệu giả nhưng thật: máy DESKTOP-DAUPH (LAN) và MacBook-Pro-Vu (relay), session Claude / Codex / Shell, trạng thái Đang chạy / Chờ bạn / Rảnh. Status luôn có icon + chữ + màu.</p>
${grp('1. Điều hướng gốc — home',A+B,['home-a','home-b'],ab('A: danh sách máy → vào máy (mở app thấy danh sách máy, gộp bản trùng tên). B: mở app vào thẳng máy gần nhất, đổi máy bằng sheet ở thanh trên.','Ảnh hiển thị: A = home-a (danh sách), B = home-b (Giám sát + sheet chọn máy mở sẵn).',pc('A: rõ máy nào đang LAN / relay, dọn bản trùng ngay trên màn; ít chạm ngược khi có nhiều máy.','A: luôn thêm 1 bước chạm trước khi thấy session. B: nhanh nhất cho người chỉ có 1–2 máy; sheet gom luôn "Quản lý bản cũ".')+pc('','B: danh sách máy nằm trong sheet nên bản trùng tên khó thấy hơn.')))}
${grp('2. Ghép máy','',['pair-qr','pair-code'])}
${grp('3. Monitor — list session',A+B,['mon-a','mon-b'],ab('A: thẻ lớn (avatar agent, tên, nhánh, tool đang chạy). B: nhóm theo trạng thái, dòng gọn.','',pc('A: nhiều thông tin, thanh trái màu theo agent như pane desktop. B: thấy 6+ session một màn, nhóm "Chờ bạn" lên đầu.','')+pc('','A: chỉ ~4 session một màn. B: ít chỗ hiện tool đang chạy.')))}
${grp('4. Chi tiết session','',['detail'])}
${grp('5. Workspace — tab + pane',A+B,['ws-a','ws-b'],ab('A: một pane lớn + chip đổi pane (vuốt ngang). B: lưới Mission Control, mỗi pane một thẻ.','',pc('A: xem được nhiều output của một pane, gần terminal thật. B: thấy mọi pane cùng lúc, đổi pane 1 chạm, khớp Mission Control desktop.','')+pc('','A: pane khác chỉ thấy tên. B: output mỗi thẻ chỉ 3 dòng.')))}
${grp('6. Terminal — thanh phím phụ, chọn pane','',['term','term-pick'])}
${grp('7. Xem file / diff','',['file','diff'])}
${grp('8. Cài đặt · màn khoá · thông báo push','',['settings','lock','push'])}
${grp('9. Empty / offline / lỗi','',['empty','offline','error'])}
</section>

<section class="sc" id="components"><h2>Component chung</h2><p class="lead">Một bộ dùng cho mọi màn: chip trạng thái, chip kết nối, nút, banner, sheet, thẻ, switch, segmented. Chạm tối thiểu 44 dp.</p>
<div class="comp">${['light','dark'].map(t=>`<div class="phone" data-theme="${t}"><div class="lbl in" style="padding-top:0">Chip trạng thái · ${t==='light'?'sáng':'tối'}</div><div class="rw">${['working','waiting','idle','done','error','stale'].map(s=>chip(s)).join('')}</div>
<div class="lbl in" style="padding-top:0">Chip kết nối</div><div class="rw">${['lan','relay','conn','off'].map(s=>chip(s)).join('')}</div>
<div class="lbl in" style="padding-top:0">Nút</div><div class="rw"><span class="btn p">Ghép máy</span><span class="btn s">Mở terminal</span><span class="btn d">${ic('stop')}Dừng</span><span class="btn t">Thử lại</span></div>
<div class="lbl in" style="padding-top:0">Agent</div><div class="rw">${['claude','codex','shell'].map(a=>`<span class="pill" style="--ac:var(--${a})">${agent(a)}${AG[a][1]}</span>`).join('')}</div>
<div class="lbl in" style="padding-top:0">Switch · segmented</div><div class="rw"><span class="sw on"><i></i></span><span class="sw"><i></i></span><div class="seg" style="width:260px"><div class="on">1 giờ</div><div>4 giờ</div><div>Không</div></div></div>
<div class="lbl in" style="padding-top:0">Banner</div><div class="banner" style="--c:var(--warning);margin-bottom:8px">${ic('wifioff')}<div><b>Mất kết nối tới DESKTOP-DAUPH</b><span>Đang nối lại qua relay.</span></div></div><div class="banner" style="--c:var(--mod)">${ic('shield')}<div><b>Mã hoá đầu cuối</b><span>Relay chỉ chuyển byte đã mã hoá.</span></div></div></div>`).join('')}</div></section>

<section class="sc" id="motion"><h2>Motion</h2><p class="lead">Mọi thay đổi đều có hiệu ứng; thời lượng 150–300 ms; chỉ animate <code>transform</code> / <code>opacity</code> / màu (A21s). Terminal không có hiệu ứng làm chậm output. Bấm nút trong từng demo để chạy lại.</p>
<div class="tool-row"><label><input type="checkbox" id="rmx"> Mô phỏng “giảm chuyển động” của máy</label><button class="rmbtn" id="thm">Đổi nền demo: sáng / tối</button></div>
<div class="mgrid" id="mg"></div>
<h3 style="margin:28px 0 10px">Bảng spec</h3>
<table class="tb"><thead><tr><th>Phần tử</th><th>Kích hoạt</th><th>Thuộc tính</th><th>Thời lượng</th><th>Curve → Flutter</th><th>Khi giảm chuyển động</th></tr></thead><tbody>
<tr><td>Dòng list thêm</td><td>Session / máy mới xuất hiện</td><td>opacity 0→1, translateY −14→0</td><td>220 ms</td><td><code>Curves.easeOutCubic</code> · <code>AnimatedList.insertItem</code></td><td>Chỉ fade 90 ms</td></tr>
<tr><td>Dòng list xoá</td><td>Session kết thúc, máy bị dọn</td><td>opacity 1→0, translateX 0→40; các dòng dưới dịch bằng FLIP (transform)</td><td>150 ms ra + 220 ms dịch</td><td><code>Curves.easeInCubic</code> ra, <code>easeOutCubic</code> dịch · <code>AnimatedList.removeItem</code></td><td>Fade 90 ms, dịch tức thì</td></tr>
<tr><td>Đổi tên</td><td>Tên session / tab đổi</td><td>opacity crossfade chữ + nháy nền accent 22%→0</td><td>150 + 150 ms, nháy 300 ms</td><td><code>Curves.easeOut</code> · <code>AnimatedSwitcher</code> + <code>TweenAnimationBuilder&lt;Color?&gt;</code></td><td>Chỉ crossfade 90 ms</td></tr>
<tr><td>Chip trạng thái</td><td><code>status</code> đổi</td><td>màu chữ / viền, icon scale 0.7→1 + opacity</td><td>220 ms (icon 150 ra, 220 vào)</td><td><code>Curves.easeOutCubic</code> · <code>AnimatedContainer</code> + <code>AnimatedSwitcher</code></td><td>Đổi màu tức thì, icon crossfade 90 ms</td></tr>
<tr><td>Chuyển màn push</td><td>Mở chi tiết, vào máy</td><td>màn mới translateX 100%→0; màn cũ translateX −24% + opacity 1→0.6</td><td>300 ms</td><td><code>Curves.easeOutCubic</code> · <code>PageRouteBuilder</code> + <code>SlideTransition</code></td><td>Crossfade 90 ms</td></tr>
<tr><td>Chuyển màn pop</td><td>Back, vuốt back</td><td>ngược push; theo ngón tay khi vuốt</td><td>210 ms (≈70%)</td><td><code>Curves.easeInCubic</code></td><td>Crossfade 90 ms</td></tr>
<tr><td>Sheet</td><td>Chọn máy, chọn pane, ghép</td><td>scrim opacity 0→1; sheet translateY 100%→0 (overshoot ~4%)</td><td>300 ms mở, 210 ms đóng</td><td><code>SpringDescription(mass: 1, stiffness: 420, damping: 34)</code> · <code>SpringSimulation</code>; đóng <code>Curves.easeInCubic</code></td><td>Fade sheet + scrim 90 ms, không overshoot</td></tr>
<tr><td>Chấm đang nối</td><td>Trạng thái <code>connecting</code></td><td>opacity 1→0.35, scale 1→0.7, lặp</td><td>1200 ms / chu kỳ</td><td><code>Curves.easeInOut</code> · <code>AnimationController.repeat(reverse: true)</code></td><td>Không nhấp nháy, chấm cố định opacity 0.7</td></tr>
<tr><td>Màn khoá hiện</td><td>Mở app / quay lại nền quá hạn</td><td>opacity 0→1</td><td>250 ms</td><td><code>Curves.easeOut</code> · <code>FadeTransition</code></td><td>Hiện ngay</td></tr>
<tr><td>Màn khoá sai</td><td>Vân tay / PIN sai</td><td>translateX ±6 px × 3 nhịp + <code>HapticFeedback.lightImpact()</code></td><td>240 ms</td><td><code>Curves.linear</code> · <code>TweenSequence</code></td><td>Bỏ rung, giữ haptic và viền đỏ</td></tr>
<tr><td>Pull to refresh</td><td>Kéo list xuống</td><td>list translateY theo ngón (max 56 px), icon rotate theo quãng kéo, quay khi tải</td><td>theo tay; về 0 trong 300 ms</td><td>về 0 bằng <code>SpringDescription(stiffness: 420, damping: 34)</code> · <code>RefreshIndicator</code> tuỳ biến</td><td>Icon không quay, chỉ hiện chip “Đang làm mới”</td></tr>
<tr><td>Nhấn nút / thẻ</td><td>Tap down / up</td><td>scale 1→0.97, opacity 1→0.86</td><td>100 ms xuống, 150 ms lên</td><td><code>Curves.easeOut</code> · <code>AnimatedScale</code></td><td>Chỉ đổi opacity</td></tr>
<tr><td>Terminal</td><td>—</td><td>Không animation; ghi theo frame (WP-25)</td><td>—</td><td>—</td><td>—</td></tr>
</tbody></table></section>

<section class="sc" id="tokens"><h2>Token desktop → Flutter</h2><p class="lead">Nguồn: <code>apps/web/src/themes.css</code> (<code>orrery-light</code>, <code>galaxy</code>), <code>apps/web/src/style.css</code> <code>:root</code>. Tên Dart đề xuất; <code>ColorScheme</code> giữ màu Material, phần còn lại vào <code>ThemeExtension&lt;OrreryColors&gt;</code>.</p>
<table class="tb"><thead><tr><th>Token desktop</th><th>Sáng</th><th>Tối (galaxy)</th><th>Flutter</th></tr></thead><tbody>
${[
['--ccm-canvas','#f3f3f6','#000000','ColorScheme.surface · scaffoldBackgroundColor'],
['--ccm-surface','#ffffff','#08080a','ColorScheme.surfaceContainerLow (Card)'],
['--ccm-raised','#ececf1','#0d0d10','ColorScheme.surfaceContainerHigh (seg track, mã khối)'],
['--ccm-control','#ffffff','#16161a','ColorScheme.surfaceContainer (nút phụ, ô nhập)'],
['--ccm-line','rgba(23,23,28,.09)','#141417','ColorScheme.outlineVariant'],
['--ccm-line-strong','rgba(23,23,28,.18)','#1c1c20','ColorScheme.outline'],
['--ccm-ink','#17171c','#ececf1','ColorScheme.onSurface'],
['--ccm-ink-muted','#55555f','#a6a6b0','ColorScheme.onSurfaceVariant'],
['--ccm-ink-faint','#61616c','#85858f','OrreryColors.inkFaint'],
['--ccm-accent','#bd5128','#e8835d','ColorScheme.primary'],
['--ccm-on-accent','#ffffff','#1a110d','ColorScheme.onPrimary'],
['--ccm-accent-ink','#b04a24','#f0946f','OrreryColors.accentInk (chữ / icon accent)'],
['--ccm-selection','#f6dccf','#35261f','ColorScheme.primaryContainer'],
['--ccm-error','#c4001a','#ff6b6b','ColorScheme.error'],
['--ccm-working','#b02f00','#f5a524','OrreryColors.working'],
['--ccm-waiting','#7f5500','#f7d046','OrreryColors.waiting'],
['--ccm-idle','#61616c','#8e8e9a','OrreryColors.idle'],
['--ccm-done','#1b6f30','#4cc38a','OrreryColors.done'],
['--ccm-stale','#64646e','#8e8e9a','OrreryColors.stale'],
['--ccm-diff-add / del / mod','#1b6f30 / #c4001a / #0b5cad','#4cc38a / #ff6b6b / #6ea8fe','OrreryColors.diffAdd / diffDel / diffMod'],
['--ccm-agent-claude','#c15f3c','#d97757','OrreryColors.agentClaude'],
['--ccm-agent-codex','#6a4fc9','#9d8cf8','OrreryColors.agentCodex'],
['--ccm-agent-shell-mark','#0f7f78','#4cc2b8','OrreryColors.agentShell'],
['--ccm-well / --ccm-well-ink','#18181c / #e5e5ea','#030304 / #e5e5ea','OrreryColors.well / wellInk (terminal, code)'],
['scrim (mới, mobile)','rgba(0,0,0,.45)','rgba(0,0,0,.6)','ColorScheme.scrim, thêm cho sheet'],
['--radius-card','10 px (mockup 10)','10 px','OrreryShape.card → CardTheme / sheet'],
['--radius-control','6 px (mockup 8)','6 px (mockup 8)','OrreryShape.control → nút, ô nhập (nới 8 cho ngón tay)'],
['--radius-inner','4 px (mockup 6)','4 px (mockup 6)','OrreryShape.inner → khối tool, mã'],
['--ccm-chip-fill','0 (chỉ viền)','0 (chỉ viền)','Chip: viền 38% màu, không nền; icon + chữ'],
['--ccm-pane-bar','inset 2px','inset 2px','Thanh trái 3 px màu agent trên thẻ / pane'],
['--font / --font-mono','system stack','system stack','TextTheme fontFamily mặc định + fontFamilyFallback; mono: Cascadia Mono / Menlo / monospace']
].map(r=>{const sw=h=>/^#/.test(h)?`<span class="sw2" style="background:${h}"></span>`:'';return `<tr><td><code>${r[0]}</code></td><td>${sw(r[1])}${r[1]}</td><td>${sw(r[2])}${r[2]}</td><td>${r[3]}</td></tr>`}).join('')}
</tbody></table>
<table class="tb"><thead><tr><th>Cỡ chữ (dp)</th><th>Dùng cho</th><th>TextTheme</th></tr></thead><tbody>
<tr><td>28 / 34 · 600</td><td>Tiêu đề lớn cấp cao nhất</td><td>headlineMedium</td></tr><tr><td>17 / 24 · 600</td><td>Tiêu đề thanh trên</td><td>titleMedium</td></tr><tr><td>15 / 20 · 600</td><td>Tên session / máy</td><td>titleSmall</td></tr><tr><td>14 / 20 · 400–500</td><td>Thân</td><td>bodyMedium</td></tr><tr><td>12 / 16</td><td>Phụ, meta</td><td>bodySmall / labelMedium</td></tr><tr><td>11.5 mono</td><td>Dòng tool, mã, terminal 12</td><td>OrreryText.mono</td></tr></tbody></table>
<p class="lead">Khoảng cách theo lưới 4 / 8 dp; chạm tối thiểu 44 dp (nút, dòng, ô nhập 44–52 dp).</p></section>

<section class="sc" id="questions"><h2>Điểm cần chủ nhân chọn</h2><div class="q"><ol>
<li><b>Điều hướng gốc:</b> A (danh sách máy → vào máy) hay B (vào thẳng máy gần nhất, đổi bằng sheet).</li>
<li><b>Monitor:</b> thẻ lớn (A) hay nhóm theo trạng thái, dòng gọn (B); có thể cho đổi trong Cài đặt.</li>
<li><b>Workspace:</b> một pane + chip (A) hay lưới Mission Control (B).</li>
<li><b>Chip trạng thái:</b> chỉ viền như token desktop (đang dùng), hay thêm nền tint 10–12% cho dễ thấy trên màn nhỏ.</li>
<li><b>Radius nút / ô nhập:</b> giữ 6 px như desktop hay nới 8 px cho cảm giác chạm (mockup đang dùng 8).</li>
<li><b>Khối code / terminal ở chủ đề sáng:</b> hiện nền tối <code>well</code> (đang dùng, đồng bộ desktop) hay nền sáng.</li>
<li><b>Thông báo push:</b> nút “Trả lời” mở app tại pane (không gõ từ notification) — chốt vì quy tắc chỉ submit khi pane idle.</li>
</ol></div></section>
</div>`;

const mg=document.getElementById('mg');
demos.forEach(d=>{const c=document.createElement('div');c.className='mcard';c.innerHTML=`<h4>${d.title}</h4><p>${d.sub}</p><div class="phone dmo" data-theme="light"><div class="bx"></div></div>`;mg.append(c);d.build(c.querySelector('.bx'))});
document.getElementById('thm').onclick=()=>document.querySelectorAll('.dmo').forEach(e=>e.dataset.theme=e.dataset.theme==='light'?'dark':'light');
document.getElementById('rmx').onchange=e=>document.body.classList.toggle('rm',e.target.checked);
document.documentElement.dataset.h=document.documentElement.scrollHeight;document.documentElement.dataset.w=document.documentElement.scrollWidth;
}
if(q.get('only')){const st=document.createElement('style');st.textContent=`section.sc:not(#${q.get('only')}){display:none}header.hd,nav.pn{display:none}`;document.head.append(st)}
