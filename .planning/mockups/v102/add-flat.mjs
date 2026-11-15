import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = join(process.argv[2], 'w2canvas', 'project');
const css = `
.app.flat{--g1:transparent;--g2:transparent;--g3:transparent;--canvas:#E9EEF1;--glass:#F6F9FA;--glass-pop:#FFFFFF;--surface:#FFFFFF;--control:#FFFFFF;--edge:#D5DEE3;--line:#E1E8EC;--line-strong:#C9D4DA;--hover:#EDF2F4;--shadow1:none;--shadow2:0 10px 30px -14px rgb(20 50 70/.30);--scrim:rgb(20 40 50/.32)}
.app.flat.dark{--canvas:#0E151A;--glass:#131D24;--glass-pop:#1A262E;--surface:#16212A;--control:#1D2A33;--edge:#24323B;--line:#22303A;--line-strong:#2E3E48;--hover:#1C2831;--shadow1:none;--shadow2:0 10px 30px -14px rgb(0 0 0/.7);--scrim:rgb(0 0 0/.55)}
.app.flat *{backdrop-filter:none!important}
.app.flat .glass,.app.flat .pop{box-shadow:none;border:1px solid var(--edge)}
.app.flat .rail,.app.flat .col,.app.flat .pane,.app.flat .ghost,.app.flat .ghost-pane,.app.flat .drawer,.app.flat .flyout{border-radius:10px}
.app.flat .tabs{border-radius:8px}
.app.flat .tab{border-radius:6px}
.app.flat .tab.on{box-shadow:inset 0 -2px 0 var(--accent);background:var(--surface)}
.app.flat .well,.app.flat .ghost-pane i{border-radius:6px}
.app.flat .need{border-radius:8px;background:var(--surface);border:1px solid var(--line);box-shadow:inset 3px 0 0 var(--wait)}
.app.flat .nitem{border-radius:6px;background:var(--wait-soft)}
.app.flat .row,.app.flat .it,.app.flat .tr{border-radius:6px}
.app.flat .seg,.app.flat .seg button{border-radius:6px}
.app.flat .seg button.on{box-shadow:none;border:1px solid var(--line-strong)}
.app.flat .chip{border-radius:4px}
.app.flat .pane.on{box-shadow:0 0 0 2px var(--accent)}
.app.flat .menubar{border-radius:6px}
.app.flat .mb,.app.flat .btn,.app.flat .btn-open{border-radius:6px}
.app.flat .toast,.app.flat .pal,.app.flat .dlg{border-radius:12px;box-shadow:var(--shadow2)}
.app.flat .card{border-radius:10px}
.app.flat .pv{border-radius:6px}
.app.flat .dtab.on{box-shadow:inset 0 -2px 0 var(--accent);border-radius:0}
.app.flat .logo,.app.flat .mono{border-radius:6px}
`;
let n = 0;
for (const f of ['Main.dc.html', 'Compact.dc.html', 'CtrlK.dc.html', 'Settings.dc.html']) {
  const p = join(dir, f);
  let s = readFileSync(p, 'utf8');
  if (s.includes('.app.flat{')) continue;
  s = s.replace('</style>\n</helmet>', css + '</style>\n</helmet>');
  s = s.replace(`data-props='{"theme":`, `data-props='{"style":{"editor":"enum","options":["kính","flat"],"default":"kính"},"theme":`);
  s = s.replace(/themeCls: theme === 'dark' \? 'dark' : '',/, "themeCls: (theme === 'dark' ? 'dark' : '') + ((this.props.style ?? 'kính') === 'flat' ? ' flat' : ''),");
  s = s.replace(/themeCls: \(this\.props\.theme \?\? '(\w+)'\) === 'dark' \? 'dark' : ''/, (m, d) => `themeCls: ((this.props.theme ?? '${d}') === 'dark' ? 'dark' : '') + ((this.props.style ?? 'kính') === 'flat' ? ' flat' : '')`);
  writeFileSync(p, s);
  n++;
}
console.log('ok ' + n);
