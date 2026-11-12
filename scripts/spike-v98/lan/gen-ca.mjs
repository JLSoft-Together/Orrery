import 'reflect-metadata';
import * as x509 from '@peculiar/x509';
import { webcrypto, randomBytes, randomUUID, createPrivateKey, X509Certificate } from 'node:crypto';
import { networkInterfaces } from 'node:os';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

x509.cryptoProvider.set(webcrypto);

export const STATE = join(dirname(fileURLToPath(import.meta.url)), 'state');
const ALG = { name: 'ECDSA', namedCurve: 'P-256', hash: 'SHA-256' };
const DAY = 86400000;
const HOUR = 3600000;
const CA_DAYS = 825;
const LEAF_DAYS = 397;

const file = (name) => join(STATE, name);

export function lanIps() {
  const out = [];
  for (const list of Object.values(networkInterfaces())) {
    for (const a of list ?? []) {
      if (a.family === 'IPv4' && !a.internal && !a.address.startsWith('169.254.')) out.push(a.address);
    }
  }
  return [...new Set(out)];
}

export function primaryIp(ips = lanIps()) {
  return process.env.SPIKE_IP || ips.find((ip) => ip.startsWith('192.168.')) || ips[0] || '127.0.0.1';
}

function serial() {
  const b = randomBytes(16);
  b[0] &= 0x7f;
  b[0] |= 0x01;
  return b.toString('hex');
}

function pem(label, der) {
  const b64 = Buffer.from(der).toString('base64').match(/.{1,64}/g).join('\n');
  return `-----BEGIN ${label}-----\n${b64}\n-----END ${label}-----\n`;
}

async function exportKey(key) {
  return pem('PRIVATE KEY', await webcrypto.subtle.exportKey('pkcs8', key));
}

async function importKey(pemText) {
  return createPrivateKey(pemText).toCryptoKey(ALG, true, ['sign']);
}

async function createCa() {
  const keys = await webcrypto.subtle.generateKey(ALG, true, ['sign', 'verify']);
  const now = Date.now();
  const cert = await x509.X509CertificateGenerator.createSelfSigned({
    serialNumber: serial(),
    name: 'CN=Orrery spike CA, O=Orrery spike',
    notBefore: new Date(now - HOUR),
    notAfter: new Date(now - HOUR + CA_DAYS * DAY),
    signingAlgorithm: ALG,
    keys,
    extensions: [
      new x509.BasicConstraintsExtension(true, 0, true),
      new x509.KeyUsagesExtension(x509.KeyUsageFlags.keyCertSign | x509.KeyUsageFlags.cRLSign, true),
      await x509.SubjectKeyIdentifierExtension.create(keys.publicKey),
    ],
  });
  writeFileSync(file('ca.key.pem'), await exportKey(keys.privateKey), { mode: 0o600 });
  writeFileSync(file('ca.cert.pem'), cert.toString('pem') + '\n');
  writeFileSync(file('orrery-spike-ca.cer'), Buffer.from(cert.rawData));
  writeFileSync(file('orrery-spike-ca.mobileconfig'), mobileconfig(Buffer.from(cert.rawData)));
}

function mobileconfig(der) {
  const certUuid = randomUUID().toUpperCase();
  const profileUuid = randomUUID().toUpperCase();
  const b64 = der.toString('base64').match(/.{1,52}/g).join('\n\t\t\t');
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>PayloadContent</key>
	<array>
		<dict>
			<key>PayloadCertificateFileName</key>
			<string>orrery-spike-ca.cer</string>
			<key>PayloadContent</key>
			<data>
			${b64}
			</data>
			<key>PayloadDescription</key>
			<string>CA riêng cho spike Orrery v98 (chỉ LAN)</string>
			<key>PayloadDisplayName</key>
			<string>Orrery spike CA</string>
			<key>PayloadIdentifier</key>
			<string>dev.ccm.spike.v98.ca.${certUuid}</string>
			<key>PayloadType</key>
			<string>com.apple.security.root</string>
			<key>PayloadUUID</key>
			<string>${certUuid}</string>
			<key>PayloadVersion</key>
			<integer>1</integer>
		</dict>
	</array>
	<key>PayloadDescription</key>
	<string>Cài CA riêng để Safari tin https / wss tới PC trong LAN. Gỡ sau spike.</string>
	<key>PayloadDisplayName</key>
	<string>Orrery spike CA</string>
	<key>PayloadIdentifier</key>
	<string>dev.ccm.spike.v98.${profileUuid}</string>
	<key>PayloadRemovalDisallowed</key>
	<false/>
	<key>PayloadType</key>
	<string>Configuration</string>
	<key>PayloadUUID</key>
	<string>${profileUuid}</string>
	<key>PayloadVersion</key>
	<integer>1</integer>
</dict>
</plist>
`;
}

async function createLeaf(ips) {
  const caCert = new x509.X509Certificate(readFileSync(file('ca.cert.pem'), 'utf8'));
  const caKey = await importKey(readFileSync(file('ca.key.pem'), 'utf8'));
  const keys = await webcrypto.subtle.generateKey(ALG, true, ['sign', 'verify']);
  const now = Date.now();
  const names = [...ips.map((value) => ({ type: 'ip', value })), { type: 'ip', value: '127.0.0.1' }, { type: 'dns', value: 'localhost' }];
  const cert = await x509.X509CertificateGenerator.create({
    serialNumber: serial(),
    subject: 'CN=Orrery spike LAN',
    issuer: caCert.subject,
    notBefore: new Date(now - HOUR),
    notAfter: new Date(now - HOUR + LEAF_DAYS * DAY),
    signingAlgorithm: ALG,
    publicKey: keys.publicKey,
    signingKey: caKey,
    extensions: [
      new x509.BasicConstraintsExtension(false, undefined, true),
      new x509.KeyUsagesExtension(x509.KeyUsageFlags.digitalSignature, true),
      new x509.ExtendedKeyUsageExtension([x509.ExtendedKeyUsage.serverAuth], false),
      new x509.SubjectAlternativeNameExtension(names, false),
      await x509.SubjectKeyIdentifierExtension.create(keys.publicKey),
      await x509.AuthorityKeyIdentifierExtension.create(caCert, false),
    ],
  });
  writeFileSync(file('leaf.key.pem'), await exportKey(keys.privateKey), { mode: 0o600 });
  writeFileSync(file('leaf.cert.pem'), cert.toString('pem') + '\n');
}

function leafCovers(ips) {
  if (!existsSync(file('leaf.cert.pem'))) return false;
  const leaf = new X509Certificate(readFileSync(file('leaf.cert.pem')));
  const ca = new X509Certificate(readFileSync(file('ca.cert.pem')));
  if (!leaf.verify(ca.publicKey)) return false;
  if (new Date(leaf.validTo).getTime() - Date.now() < 30 * DAY) return false;
  return ips.every((ip) => leaf.checkIP(ip));
}

export async function ensureCerts({ ips = lanIps(), force = false } = {}) {
  mkdirSync(STATE, { recursive: true });
  let caCreated = false;
  let leafCreated = false;
  if (force || !existsSync(file('ca.key.pem')) || !existsSync(file('ca.cert.pem'))) {
    await createCa();
    caCreated = true;
  }
  if (caCreated || !leafCovers(ips)) {
    await createLeaf(ips);
    leafCreated = true;
  }
  return {
    caCreated,
    leafCreated,
    ips,
    caPem: readFileSync(file('ca.cert.pem'), 'utf8'),
    leafPem: readFileSync(file('leaf.cert.pem'), 'utf8'),
    leafKeyPem: readFileSync(file('leaf.key.pem'), 'utf8'),
    mobileconfig: readFileSync(file('orrery-spike-ca.mobileconfig')),
  };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const r = await ensureCerts({ force: process.argv.includes('--force') });
  const leaf = new X509Certificate(r.leafPem);
  console.log(`CA ${r.caCreated ? 'mới tạo' : 'đã có'}, cert lá ${r.leafCreated ? 'mới tạo' : 'đã có'}`);
  console.log(`SAN: ${leaf.subjectAltName}`);
  console.log(`Hạn cert lá: ${leaf.validFrom} → ${leaf.validTo}`);
  console.log(`Thư mục: ${STATE}`);
}
