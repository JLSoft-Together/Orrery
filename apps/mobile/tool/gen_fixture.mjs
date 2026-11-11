import { execSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const ESC = "\x1b";
const sh = (c) => execSync(c, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
const crlf = (s) => s.replace(/\r?\n/g, "\r\n");
const parts = [];

parts.push(`${ESC}[1;36m== claude --help ==${ESC}[0m\r\n`);
parts.push(crlf(sh("claude --help")));
parts.push(`${ESC}[1;36m== truecolor ramp ==${ESC}[0m\r\n`);
for (let row = 0; row < 8; row++) {
  let line = "";
  for (let i = 0; i < 76; i++) {
    const r = Math.round((255 * i) / 75);
    const g = Math.round((255 * row) / 7);
    const b = 255 - r;
    line += `${ESC}[48;2;${r};${g};${b}m ${ESC}[0m`;
  }
  parts.push(line + "\r\n");
}
parts.push(`${ESC}[38;2;255;128;0mcam ${ESC}[38;2;0;200;120mxanh ${ESC}[38;2;120;120;255mtim${ESC}[0m ${ESC}[1mbold${ESC}[0m ${ESC}[3mitalic${ESC}[0m ${ESC}[4munderline${ESC}[0m ${ESC}[7mreverse${ESC}[0m\r\n`);
for (let c = 0; c < 256; c += 1) parts.push(`${ESC}[48;5;${c}m  ${ESC}[0m` + (c % 32 === 31 ? "\r\n" : ""));
parts.push("\r\n");
parts.push(`${ESC}[1;36m== Tiếng Việt ==${ESC}[0m\r\n`);
const vi = [
  "Tiếng Việt có dấu: Nguyễn Văn Ánh, Trường Sơn, Đường Hoàng Diệu, quả thuỷ tinh ướt",
  "Ă Â Ê Ô Ơ Ư Đ ă â ê ô ơ ư đ ạ ả ã á à ậ ẩ ẫ ấ ầ ặ ẳ ẵ ắ ằ ệ ể ễ ế ề ộ ổ ỗ ố ồ ợ ở ỡ ớ ờ ự ử ữ ứ ừ ỵ ỷ ỹ ý ỳ",
  "Xin chào chủ nhân, phiên làm việc đang chạy, bấm Gửi để đưa lệnh vào terminal",
];
for (let i = 0; i < 4; i++) vi.forEach((l) => parts.push(`${ESC}[32m${l}${ESC}[0m\r\n`));
parts.push(`${ESC}[1;36m== Wide / emoji ==${ESC}[0m\r\n`);
parts.push("⬛⬜🟥🟩🟦 ✅ ❌ ⚠️ 🚀 🔥 日本語 中文字符 한국어 ┌──┬──┐ │ab│cd│ └──┴──┘ ▁▂▃▄▅▆▇█\r\n");
parts.push("|日本|中文|한글|⬛⬛|ab|\r\n");
parts.push(`${ESC}[1;36m== git log ==${ESC}[0m\r\n`);
parts.push(crlf(sh("git log --color=always --graph -400 --pretty=format:\"%C(auto)%h%d%x20%s%x20%C(green)(%cr)%C(bold blue)<%an>%C(reset)\"")));
parts.push("\r\n");
let body = parts.join("");
let n = 0;
while (Buffer.byteLength(body) < 256 * 1024) {
  n++;
  body += `${ESC}[33mline ${n}${ESC}[0m ${ESC}[38;2;${(n * 7) % 256};${(n * 13) % 256};${(n * 29) % 256}mtruecolor Tiếng Việt ⬛ 日本 filler ${"x".repeat(40)}${ESC}[0m\r\n`;
}
body += `${ESC}[1;36m== alt screen ==${ESC}[0m\r\n`;
body += `${ESC}[?1049h${ESC}[2J${ESC}[H${ESC}[1;44m ALT SCREEN TUI ${ESC}[0m\r\n${ESC}[5;10HHello from alt screen${ESC}[7;10H${ESC}[7m selected ${ESC}[0m`;
body += `\r\n${ESC}[10;1H(nhan Esc de thoat)\r\n`;
body += `${ESC}[?1049l${ESC}[1;32mback to main screen${ESC}[0m\r\n`;
writeFileSync("assets/fixture.ansi", body);
console.log(Buffer.byteLength(body));
