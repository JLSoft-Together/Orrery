import { execFileSync, spawn, spawnSync } from "node:child_process";
import { createReadStream, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, extname, join, normalize, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { chromium, webkit } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const relayDir = join(root, "apps/relay");
const vectorsPath = join(root, "apps/core/crates/core-remote-proto/tests/vectors.json");
const peerExe = join(root, "apps/core/target/debug", process.platform === "win32" ? "spike-peer.exe" : "spike-peer");
const flags = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, "").split("=");
    return [k, v ?? "1"];
  }),
);
const relayPort = Number(flags.port ?? 8787);
const relayHttp = `http://127.0.0.1:${relayPort}`;
const iterations = Number(flags.iterations ?? 20);
const browsers = (flags.browsers ?? "webkit,chromium").split(",");
const children = new Set();
const MIME = { ".html": "text/html", ".mjs": "text/javascript", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json" };

function killTree(child) {
  if (!child || child.exitCode !== null) return;
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  else child.kill("SIGTERM");
}

function cleanup() {
  for (const c of children) killTree(c);
  children.clear();
}

async function waitHealth(ms) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    try {
      const r = await fetch(`${relayHttp}/health`);
      if (r.ok && (await r.text()) === "ok") return;
    } catch {}
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error("relay did not come up");
}

function startRelay(persist) {
  const wrangler = join(relayDir, "node_modules/wrangler/bin/wrangler.js");
  if (!existsSync(wrangler)) throw new Error("apps/relay not installed: npm install --workspaces=false --include-workspace-root=false");
  const args = [wrangler, "dev", "--ip", "127.0.0.1", "--port", String(relayPort), "--persist-to", persist, "--show-interactive-dev-session=false"];
  const child = spawn(process.execPath, args, { cwd: relayDir, env: { ...process.env, WRANGLER_SEND_METRICS: "false" }, stdio: ["ignore", "pipe", "pipe"] });
  children.add(child);
  const log = [];
  child.stdout.on("data", (d) => log.push(String(d)));
  child.stderr.on("data", (d) => log.push(String(d)));
  return { child, log };
}

function startPeer() {
  const child = spawn(peerExe, ["--relay", relayHttp], { stdio: ["pipe", "pipe", "pipe"] });
  children.add(child);
  const log = [];
  child.stderr.on("data", (d) => log.push(String(d).trim()));
  return new Promise((resolveReady, reject) => {
    const rl = createInterface({ input: child.stdout });
    rl.once("line", (line) => resolveReady({ child, log, info: JSON.parse(line) }));
    child.once("exit", (code) => reject(new Error(`peer exited ${code}: ${log.join(" ")}`)));
  });
}

function startStatic() {
  const server = createServer((req, res) => {
    const path = decodeURIComponent(new URL(req.url, "http://x").pathname);
    let file;
    if (path === "/favicon.ico") {
      res.writeHead(204).end();
      return;
    }
    if (path === "/") file = join(here, "web/index.html");
    else if (path === "/vectors.json") file = vectorsPath;
    else {
      file = normalize(join(here, path));
      const allowed = ["web", "pkg", "pkg-vectors"].some((d) => file.startsWith(join(here, d) + sep));
      if (!allowed) file = null;
    }
    if (!file || !existsSync(file)) {
      res.writeHead(404).end();
      return;
    }
    res.writeHead(200, { "content-type": MIME[extname(file)] ?? "application/octet-stream", "cache-control": "no-store" });
    createReadStream(file).pipe(res);
  });
  return new Promise((r) => server.listen(0, "127.0.0.1", () => r(server)));
}

async function launch(name) {
  if (name === "webkit") return webkit.launch();
  try {
    return await chromium.launch({ channel: flags.channel ?? "chrome" });
  } catch {
    return chromium.launch();
  }
}

async function runBrowser(name, base) {
  const peer = await startPeer();
  const browser = await launch(name);
  const errors = [];
  try {
    const page = await (await browser.newContext()).newPage();
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
    await page.goto(base);
    await page.waitForFunction(() => window.spikeReady === true, null, { timeout: 15000 });
    const cfg = { qr: peer.info.qr, bootToken: peer.info.bootToken, name: `${name} spike`, iterations };
    const result = await page.evaluate((c) => window.runSpike(c), cfg);
    return { browser: name, version: browser.version(), ...result, errors, peerLog: peer.log };
  } catch (e) {
    return { browser: name, version: browser.version(), failed: String(e?.message ?? e), errors, peerLog: peer.log };
  } finally {
    await browser.close();
    peer.child.stdin.end();
    killTree(peer.child);
    children.delete(peer.child);
  }
}

const f1 = (n) => (typeof n === "number" ? n.toFixed(1) : "-");

function summary(vectorsNode, results) {
  const lines = [`vectors node: ${vectorsNode}`];
  for (const r of results) {
    if (r.failed) {
      lines.push(`${r.browser} ${r.version}: FAILED ${r.failed}`);
      continue;
    }
    const pass = r.checks.filter((c) => c.ok).length;
    const t = r.timings;
    lines.push(
      `${r.browser} ${r.version}: checks ${pass}/${r.checks.length}, vectors ${r.vectors.pass} pass ${r.vectors.fail.length} fail`,
      `  wasm load ${f1(t.wasmLoadMs)} ms · pair ws ${f1(t.pairWsOpenMs)} + hs ${f1(t.pairHandshakeMs)} ms (crypto ${f1(t.pairCryptoMs)}) · IK ws ${f1(t.ikWsOpenMs)} + hs ${f1(t.ikHandshakeMs)} ms (crypto ${f1(t.ikCryptoMs)}) · hello ${f1(t.helloRoundTripMs)} ms`,
      `  64 KiB rtt median ${f1(r.rtt64k.median)} p95 ${f1(r.rtt64k.p95)} min ${f1(r.rtt64k.min)} max ${f1(r.rtt64k.max)} ms (n=${r.rtt64k.n}) · ping ${f1(t.pingRoundTripMs)} ms · seal ${f1(t.sealMiBps)} MiB/s`,
    );
    for (const c of r.checks.filter((c) => !c.ok)) lines.push(`  FAIL ${c.name} ${JSON.stringify(c.detail ?? "")}`);
    if (r.errors.length) lines.push(`  page errors: ${r.errors.join(" | ")}`);
  }
  return lines.join("\n");
}

async function main() {
  if (flags.build || !existsSync(join(here, "pkg/core_remote_wasm.js")) || !existsSync(join(here, "pkg-vectors/core_remote_wasm.js")) || !existsSync(peerExe)) {
    execFileSync(process.execPath, [join(here, "build.mjs")], { stdio: "inherit" });
  }
  const vec = spawnSync(process.execPath, [join(here, "vectors-node.mjs")], { encoding: "utf8" });
  const vectorsNode = (vec.stdout.trim().split("\n").at(-1) ?? "") + (vec.status === 0 ? "" : " (FAILED)");
  const persist = mkdtempSync(join(tmpdir(), "orrery-relay-"));
  const relay = startRelay(persist);
  const server = await startStatic();
  const results = [];
  try {
    await waitHealth(60000).catch((e) => {
      throw new Error(`${e.message}\n${relay.log.join("")}`);
    });
    const base = `http://127.0.0.1:${server.address().port}/`;
    for (const name of browsers) results.push(await runBrowser(name, base));
  } finally {
    server.close();
    cleanup();
    try {
      rmSync(persist, { recursive: true, force: true });
    } catch {}
  }
  const qa = join(root, "docs/qa/v98/WP-50a");
  mkdirSync(qa, { recursive: true });
  writeFileSync(join(qa, "results.json"), JSON.stringify({ at: new Date().toISOString(), iterations, vectorsNode, results }, null, 2) + "\n");
  console.log(summary(vectorsNode, results));
  const bad = vec.status !== 0 || results.some((r) => r.failed || r.checks.some((c) => !c.ok) || r.vectors.fail.length);
  process.exitCode = bad ? 1 : 0;
}

process.on("SIGINT", () => {
  cleanup();
  process.exit(130);
});
main().catch((e) => {
  cleanup();
  console.error(e);
  process.exit(1);
});
