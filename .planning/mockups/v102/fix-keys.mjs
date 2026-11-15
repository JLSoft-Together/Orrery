import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = join(process.argv[2], 'src');
const edits = {
  'Main.dc.html': [['Mở kế tiếp <span class="kbd">Ctrl .</span>', 'Mở kế tiếp <span class="kbd">Ctrl Shift A</span>']],
  'Split.dc.html': [
    [/<div class="mi hot">Chia phải[\s\S]*?<div class="mi">Đóng pane<span class="k">Ctrl W<\/span><\/div>/,
      '<div class="mi hot">Chia phải<span class="k">Alt Shift +</span></div>\n            <div class="mi">Chia dưới<span class="k">Alt Shift -</span></div>\n            <div class="msep"></div>\n            <div class="mi">Sang pane bên cạnh<span class="k">Alt ←↑↓→</span></div>\n            <div class="mi">Đổi chỗ với pane bên cạnh<span class="k">Alt Shift ←↑↓→</span></div>\n            <div class="msep"></div>\n            <div class="mi">Phóng to<span class="k">Ctrl Shift Enter</span></div>\n            <div class="mi">Tách sang tab mới<span class="k">Ctrl Alt T</span></div>\n            <div class="mi">Đóng pane<span class="k">Ctrl Shift W</span></div>'],
    ['<button class="btn">Hoàn tác <span class="kbd">Ctrl Z</span></button>', '<button class="btn">Hoàn tác</button>'],
    ['Pane giữ nguyên, chỉ xếp lại vị trí. Hoàn tác Ctrl+Z.', 'Pane giữ nguyên, chỉ xếp lại vị trí. Hoàn tác ở toast.']
  ]
};
let n = 0;
for (const [f, list] of Object.entries(edits)) {
  const p = join(dir, f);
  let s = readFileSync(p, 'utf8');
  for (const [a, b] of list) {
    const before = s;
    s = s.replace(a, b);
    if (s !== before) n++;
  }
  writeFileSync(p, s);
}
console.log('ok ' + n);
