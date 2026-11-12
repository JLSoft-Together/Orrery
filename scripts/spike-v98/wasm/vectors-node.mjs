import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkVectors } from "./web/vectors.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const vectorsPath = resolve(here, "../../../apps/core/crates/core-remote-proto/tests/vectors.json");
const wasm = await import("./pkg-vectors/core_remote_wasm.js");
wasm.initSync({ module: readFileSync(join(here, "pkg-vectors/core_remote_wasm_bg.wasm")) });

const results = checkVectors(wasm, JSON.parse(readFileSync(vectorsPath, "utf8")));
const failed = results.filter((r) => !r.ok);
for (const r of results) console.log(`${r.ok ? "pass" : "FAIL"}  ${r.name}${r.ok ? "" : `\n      got  ${r.got}\n      want ${r.want}`}`);
console.log(`vectors (node ${process.version}): ${results.length - failed.length} pass, ${failed.length} fail`);
process.exit(failed.length === 0 ? 0 : 1);
