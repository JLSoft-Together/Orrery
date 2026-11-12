import QRCode from 'qrcode';
import { ensureCerts, lanIps, primaryIp, STATE } from './gen-ca.mjs';
import { startServers, PORT_A, PORT_HTTP, PORT_B } from './server.mjs';
import { vapid } from './push.mjs';

const ips = lanIps();
const ip = primaryIp(ips);
const certs = await ensureCerts({ ips: ips.includes(ip) ? ips : [...ips, ip] });
vapid();
const servers = startServers({ certs, ip, ips });
try {
  await servers.ready;
} catch (e) {
  console.error(`Không mở được cổng: ${e.message}`);
  await servers.close();
  process.exit(1);
}

const setupUrl = `http://${ip}:${PORT_HTTP}/`;
console.log('');
console.log(`CA: ${certs.caCreated ? 'mới tạo' : 'dùng lại'}, cert lá: ${certs.leafCreated ? 'mới tạo' : 'dùng lại'} (SAN ${ips.join(', ')}, 127.0.0.1, localhost)`);
console.log(`Thư mục khoá + kết quả: ${STATE}`);
console.log('');
console.log(`  Hồ sơ CA (http, chỉ trước khi tin CA): ${setupUrl}`);
console.log(`  Origin A (trang + /ping + wss /echo):  https://${ip}:${PORT_A}/`);
console.log(`  Origin B (khác origin → wss :${PORT_A}):   https://${ip}:${PORT_B}/`);
console.log(`  Biến thể CSP wildcard:                 https://${ip}:${PORT_A}/?csp=wild`);
console.log('');
console.log(await QRCode.toString(setupUrl, { type: 'terminal', small: true }));
console.log('iPhone (cùng Wi-Fi): quét QR bằng Camera → mở bằng Safari → "Tải hồ sơ CA" → Cho phép.');
console.log('  Cài đặt → Đã tải về hồ sơ → Orrery spike CA → Cài đặt.');
console.log('  Cài đặt → Cài đặt chung → Giới thiệu → Cài đặt tin cậy chứng nhận → bật Orrery spike CA.');
console.log(`  Rồi mở https://${ip}:${PORT_A}/ trong Safari, làm theo các bước trên trang.`);
console.log('Push từ PC: node push.mjs "tiêu đề" "nội dung" [--delay=15]   ·   Ctrl+C để tắt.');
console.log('');

let closing = false;
const stop = async () => {
  if (closing) return;
  closing = true;
  console.log('Đang tắt…');
  await servers.close();
  process.exit(0);
};
process.on('SIGINT', stop);
process.on('SIGTERM', stop);
