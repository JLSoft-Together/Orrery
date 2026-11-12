const RM=()=>document.body.classList.contains('rm')||matchMedia('(prefers-reduced-motion: reduce)').matches;
const D=(ms)=>RM()?90:ms;
const anim=(el,kf,ms,ease='cubic-bezier(.215,.61,.355,1)',o={})=>{
  if(RM()){kf=kf.map(f=>{const g={...f};delete g.transform;return g});ms=90;ease='linear'}
  return el.animate(kf,{duration:ms,easing:ease,fill:'both',...o});
};
const el=(t,c,h)=>{const e=document.createElement(t);if(c)e.className=c;if(h!==undefined)e.innerHTML=h;return e};
const rowHtml=(n,t,st)=>`<span class="agent a-${n}">${ic(n==='claude'?'spark':n==='codex'?'hex':'terminal')}</span><div class="grow"><div class="t1 ell nm">${t}</div><div class="t2">claude-code-monitor</div></div>${chip(st)}`;
const demos=[];
const demo=(id,title,sub,build)=>demos.push({id,title,sub,build});

demo('list','Danh sách: thêm / xoá / đổi tên','Dòng mới trượt từ trên xuống và hiện dần; dòng xoá trượt ra phải và mờ, các dòng dưới dịch lên bằng FLIP (chỉ transform).',(box)=>{
  box.innerHTML='<div class="mstage"><div class="mlist"></div></div><div class="mctl"><button data-a="add">Thêm</button><button data-a="ren">Đổi tên</button><button data-a="del">Xoá</button></div>';
  const list=box.querySelector('.mlist');let n=0;const names=['Refactor AdsGate','Review PR #482','flutter test','npm run dev','Sửa lỗi pty.overflow'];
  const mk=(t,a,st)=>{const r=el('div','card row mrow',rowHtml(a,t,st));return r};
  [['claude','Dựng mockup UI mobile','working'],['codex','Sửa lỗi pty.overflow','waiting']].forEach(x=>list.append(mk(x[1],x[0],x[2])));
  const flip=(fn)=>{const rs=[...list.children];const b=new Map(rs.map(r=>[r,r.getBoundingClientRect().top]));fn();[...list.children].forEach(r=>{if(!b.has(r))return;const d=b.get(r)-r.getBoundingClientRect().top;if(d)anim(r,[{transform:`translateY(${d}px)`},{transform:'none'}],D(220))})};
  box.querySelector('.mctl').onclick=e=>{const a=e.target.dataset.a;if(!a)return;
    if(a==='add'){const r=mk(names[n++%5],['claude','codex','shell'][n%3],'idle');flip(()=>list.prepend(r));anim(r,[{opacity:0,transform:'translateY(-14px)'},{opacity:1,transform:'none'}],D(220))}
    if(a==='del'&&list.children.length){const r=list.lastElementChild;const t=list.children[0];const x=list.children.length>1?list.children[1]:r;anim(x,[{opacity:1,transform:'none'},{opacity:0,transform:'translateX(40px)'}],D(150),'cubic-bezier(.55,.055,.675,.19)').finished.then(()=>flip(()=>x.remove()))}
    if(a==='ren'&&list.firstElementChild){const nm=list.firstElementChild.querySelector('.nm');anim(nm,[{opacity:1},{opacity:0}],D(150)).finished.then(()=>{nm.textContent=names[n++%5];anim(nm,[{opacity:0},{opacity:1}],D(150));anim(list.firstElementChild,[{backgroundColor:'color-mix(in srgb,var(--accent) 22%,transparent)'},{backgroundColor:'transparent'}],D(300),'linear')})}
  };
});

demo('chip','Chip đổi trạng thái','Màu và icon chuyển mượt: icon cũ xoay mờ đi, icon mới hiện từ scale 0.7; màu viền và chữ đổi trong 220 ms.',(box)=>{
  box.innerHTML='<div class="mstage c"><span class="chip st-idle" id="mc">'+ic('moon')+'<b>Rảnh</b></span></div><div class="mctl"><button>Đổi trạng thái</button></div>';
  const seq=['idle','working','waiting','done','error'];let i=0;const c=box.querySelector('#mc');
  c.style.transition='color 220ms,background-color 220ms,border-color 220ms';
  box.querySelector('button').onclick=()=>{i=(i+1)%seq.length;const s=seq[i];const svg=c.querySelector('svg');
    anim(svg,[{opacity:1,transform:'scale(1)'},{opacity:0,transform:'scale(.7)'}],D(150),'cubic-bezier(.55,.055,.675,.19)').finished.then(()=>{c.className='chip st-'+s;c.innerHTML=ic(ST[s][0],s==='working'?'spin':'')+'<b>'+ST[s][1]+'</b>';anim(c.querySelector('svg'),[{opacity:0,transform:'scale(.7)'},{opacity:1,transform:'scale(1)'}],D(220))})};
});

demo('nav','Chuyển màn: push / pop','Push: màn mới vào từ phải 300 ms, màn cũ lùi 24% và tối đi. Pop: ngược lại, ngắn hơn (210 ms ≈ 70%).',(box)=>{
  box.innerHTML='<div class="mstage mnav"><div class="pg p1"><div class="t1">Giám sát</div><div class="t2">DESKTOP-DAUPH</div><div class="card" style="margin-top:12px">Dựng mockup UI mobile</div></div><div class="pg p2" style="transform:translateX(100%)"><div class="t1">Chi tiết session</div><div class="t2">Claude · feat/v97-mobile</div></div></div><div class="mctl"><button data-a="push">Push</button><button data-a="pop">Pop</button></div>';
  const p1=box.querySelector('.p1'),p2=box.querySelector('.p2');
  box.querySelector('.mctl').onclick=e=>{const a=e.target.dataset.a;if(a==='push'){anim(p2,[{transform:'translateX(100%)'},{transform:'translateX(0)'}],D(300));anim(p1,[{transform:'translateX(0)',opacity:1},{transform:'translateX(-24%)',opacity:.6}],D(300))}
    if(a==='pop'){anim(p2,[{transform:'translateX(0)'},{transform:'translateX(100%)'}],D(210),'cubic-bezier(.55,.055,.675,.19)');anim(p1,[{transform:'translateX(-24%)',opacity:.6},{transform:'translateX(0)',opacity:1}],D(210))}}
});

demo('sheet','Sheet spring','Scrim mờ vào 220 ms; sheet trượt lên 300 ms với spring nhẹ (overshoot ~4%), đóng bằng ease-in 210 ms.',(box)=>{
  box.innerHTML='<div class="mstage mnav"><div class="pg"><div class="t1">Giám sát</div><div class="t2">Chạm “Mở sheet”</div></div><div class="scr0" style="opacity:0;pointer-events:none"></div><div class="sh0" style="transform:translateY(100%)"><div class="grab"></div><div class="t1">Chọn máy</div><div class="t2">DESKTOP-DAUPH · MacBook-Pro-Vu</div></div></div><div class="mctl"><button data-a="o">Mở sheet</button><button data-a="c">Đóng</button></div>';
  const sc=box.querySelector('.scr0'),sh=box.querySelector('.sh0');
  box.querySelector('.mctl').onclick=e=>{const a=e.target.dataset.a;if(a==='o'){anim(sc,[{opacity:0},{opacity:1}],D(220));anim(sh,[{transform:'translateY(100%)'},{transform:'translateY(0)'}],D(300),'cubic-bezier(.3,1.25,.5,1)')}
    if(a==='c'){anim(sc,[{opacity:1},{opacity:0}],D(210));anim(sh,[{transform:'translateY(0)'},{transform:'translateY(100%)'}],D(210),'cubic-bezier(.55,.055,.675,.19)')}}
});

demo('dots','Chấm đang nối','Chấm nhịp opacity 1 → 0.35 và scale 1 → 0.7, chu kỳ 1.2 s, lặp khi trạng thái là Đang nối; dừng ngay khi nối xong.',(box)=>{
  box.innerHTML='<div class="mstage c"><span class="chip st-conn" id="cn"><span class="dot pulse"></span><b>Đang nối…</b></span></div><div class="mctl"><button>Nối xong</button><button>Mất kết nối</button></div>';
  const c=box.querySelector('#cn');const b=box.querySelectorAll('button');
  const set=(cls,html,pulse)=>{c.className='chip '+cls;c.innerHTML=html;if(pulse&&!RM()){}};
  b[0].onclick=()=>{c.className='chip st-lan';c.innerHTML=ic('wifi')+'<b>LAN · 12 ms</b>'};
  b[1].onclick=()=>{c.className='chip st-conn';c.innerHTML='<span class="dot pulse"></span><b>Đang nối…</b>'};
});

demo('lock','Màn khoá: hiện dần và rung nhẹ','Mở app: nội dung hiện dần 250 ms. Sai vân tay: logo rung ngang ±6 px, 3 nhịp trong 240 ms, kèm haptic nhẹ.',(box)=>{
  box.innerHTML='<div class="mstage c"><div id="lk" style="text-align:center"><svg viewBox="0 0 96 96" width="64" height="64" style="color:var(--accent)"><g fill="none" stroke="currentColor" stroke-width="3"><circle cx="48" cy="48" r="14"/><circle cx="48" cy="48" r="34" opacity=".5"/></g><circle cx="48" cy="14" r="6" fill="var(--codex)"/></svg><div class="t1" style="margin-top:6px">Orrery đang khoá</div></div></div><div class="mctl"><button data-a="in">Hiện dần</button><button data-a="bad">Sai vân tay</button></div>';
  const l=box.querySelector('#lk');
  box.querySelector('.mctl').onclick=e=>{const a=e.target.dataset.a;if(a==='in')anim(l,[{opacity:0},{opacity:1}],D(250));
    if(a==='bad')anim(l,[{transform:'translateX(0)'},{transform:'translateX(-6px)'},{transform:'translateX(6px)'},{transform:'translateX(-4px)'},{transform:'translateX(0)'}],D(240),'linear')}
});

demo('ptr','Pull to refresh','Kéo xuống: list dịch theo ngón tay (transform), icon xoay theo quãng kéo; thả quá ngưỡng 64 px thì list giữ ở 56 px, icon quay đến khi xong rồi list về 0 bằng spring.',(box)=>{
  box.innerHTML='<div class="mstage mnav"><div class="ind" style="opacity:0">'+ic('refresh')+'</div><div class="pl"><div class="card" style="margin-bottom:8px">Dựng mockup UI mobile</div><div class="card" style="margin-bottom:8px">Sửa lỗi pty.overflow</div><div class="card">npm run dev</div></div></div><div class="mctl"><button>Kéo để làm mới</button></div>';
  const l=box.querySelector('.pl'),i=box.querySelector('.ind');
  box.querySelector('button').onclick=()=>{
    anim(l,[{transform:'translateY(0)'},{transform:'translateY(56px)'}],D(220),'cubic-bezier(.215,.61,.355,1)');
    anim(i,[{opacity:0,transform:'rotate(0)'},{opacity:1,transform:'rotate(180deg)'}],D(220));
    setTimeout(()=>{const s=i.animate([{transform:'rotate(180deg)'},{transform:'rotate(540deg)'}],{duration:RM()?1:800,iterations:RM()?1:2,fill:'both'});
      s.finished.then(()=>{anim(l,[{transform:'translateY(56px)'},{transform:'translateY(0)'}],D(300),'cubic-bezier(.3,1.25,.5,1)');anim(i,[{opacity:1},{opacity:0}],D(150))})},D(240))};
});

window.demos=demos;
