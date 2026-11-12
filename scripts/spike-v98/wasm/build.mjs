import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const core = join(root, "apps/core");
const wasmTarget = join(core, "target/wasm32-unknown-unknown");
const localBindgen = join(here, ".bin", process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen");
const bindgen = existsSync(localBindgen) ? localBindgen : "wasm-bindgen";
const args = new Set(process.argv.slice(2));

function run(cmd, argv, env = {}) {
  execFileSync(cmd, argv, { cwd: core, stdio: "inherit", env: { ...process.env, ...env } });
}

function cargoWasm(profile, features) {
  const argv = ["build", "-p", "core-remote-wasm", "--target", "wasm32-unknown-unknown", "-j", "4"];
  if (profile === "release") argv.push("--release");
  if (features) argv.push("--features", features);
  run("cargo", argv);
  return join(wasmTarget, profile, "core_remote_wasm.wasm");
}

function bind(wasm, outDir) {
  mkdirSync(outDir, { recursive: true });
  run(bindgen, ["--target", "web", "--out-dir", outDir, "--no-typescript", wasm]);
}

function sizes(file) {
  const raw = readFileSync(file);
  return {
    raw: raw.length,
    gzip: gzipSync(raw, { level: 9 }).length,
    brotli: brotliCompressSync(raw, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length,
  };
}

const report = {};
const prodProfile = args.has("--debug") ? "debug" : "release";
const prod = cargoWasm(prodProfile, null);
bind(prod, join(here, "pkg"));
report.prod = {
  profile: prodProfile,
  cargoWasm: sizes(prod),
  bindgenWasm: sizes(join(here, "pkg/core_remote_wasm_bg.wasm")),
  glueJs: sizes(join(here, "pkg/core_remote_wasm.js")),
};

const vec = cargoWasm("debug", "test-vectors");
bind(vec, join(here, "pkg-vectors"));
report.vectors = { profile: "debug", bindgenWasm: sizes(join(here, "pkg-vectors/core_remote_wasm_bg.wasm")) };

if (!args.has("--no-peer")) {
  run("cargo", ["build", "--manifest-path", join(here, "peer/Cargo.toml"), "-j", "4"], {
    CARGO_TARGET_DIR: join(core, "target"),
  });
}

const qa = join(root, "docs/qa/v98/WP-50a");
mkdirSync(qa, { recursive: true });
writeFileSync(join(qa, "sizes.json"), JSON.stringify(report, null, 2) + "\n");
const kb = (n) => (n / 1024).toFixed(1) + " KiB";
const p = report.prod.bindgenWasm;
console.log(`wasm (${prodProfile}): raw ${kb(p.raw)} · gzip ${kb(p.gzip)} · brotli ${kb(p.brotli)}; glue js raw ${kb(report.prod.glueJs.raw)}`);
