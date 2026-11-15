import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = join(process.argv[2], 'w2canvas', 'project');
const main = readFileSync(join(dir, 'Main.dc.html'), 'utf8');
const shared = main.match(/<style>\n([\s\S]*?)<\/style>/)[1];
let n = 0;
for (const f of ['Drag.dc.html', 'Split.dc.html']) {
  const p = join(dir, f);
  const s = readFileSync(p, 'utf8');
  if (!s.includes('/*SHARED*/')) continue;
  writeFileSync(p, s.replace('/*SHARED*/\n', shared));
  n++;
}
const cj = join(dir, 'canvas.json');
const c = JSON.parse(readFileSync(cj, 'utf8'));
c.boards['Drag.dc.html'] = { x: 0, y: 2040, w: 1440, h: 900, title: 'Kéo thả: la bàn, đổi chỗ, quá hẹp, sang tab, từ Explorer', is_interactive: true };
c.boards['Split.dc.html'] = { x: 1520, y: 2040, w: 1440, h: 900, title: 'Chia pane: thanh chia, bố cục, gợi ý tách tab, menu phím, hoàn tác', is_interactive: true };
for (const k of ['Drag.dc.html', 'Split.dc.html']) if (!c.order.includes(k)) c.order.push(k);
writeFileSync(cj, JSON.stringify(c));
console.log('ok ' + n);
