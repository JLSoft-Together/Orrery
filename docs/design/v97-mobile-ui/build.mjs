import {readFileSync,writeFileSync} from 'node:fs';
import {dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
const d=dirname(fileURLToPath(import.meta.url));
const r=f=>readFileSync(join(d,'src',f),'utf8');
const html=r('page.html')
  .replace('/*CSS*/',()=>r('style.css')+'\n'+r('page.css'))
  .replace('/*ICONS_SCREENS*/',()=>r('screens.js'))
  .replace('/*MOTION*/',()=>r('motion.js'))
  .replace('/*PAGE*/',()=>r('page.js'));
writeFileSync(join(d,'index.html'),html);
console.log('index.html',html.length);
