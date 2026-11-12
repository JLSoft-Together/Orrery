# apps/pwa (@ccm/pwa) — iPhone / iPad web app (v98 WP-54)

- Build: Vite base `/app/`, dist -> relay `/app/*`. `vite/serviceWorker.ts` writes `sw.js` (precache list + cache name from content hash; skip-waiting only on message; `push` / `notificationclick` delegate to `self.orreryPush`, WP-56). `vite.config.ts` `__DEV_TOOLS__` (serve only) gates Eruda + dev log (`vite/devLog.ts`).
- wasm: `npm run wasm -w @ccm/pwa` -> `src/gen/` (gitignored), `scripts/{build-wasm,ensure-wasm}.mjs`.
- `src/remote/`: `noise.ts` (NoiseSuite / NoiseLink over core-remote-wasm, frees wasm objects), `wire.ts` (WireSocket), `session.ts` (RemoteSession: request/event/keepalive), `dialer.ts` (planRoutes, dial), `handshake.ts`, `connection.ts` (MachineConnection: backoff, background/foreground, 4401 -> onRevoked), `pairing.ts` (PairFlow, QR v1/v2 candidates), `jsonPatch.ts`, `monitorReducer.ts`, `machineData.ts` (per-machine view, cache debounce), `hub.ts` (RemoteHub: machines + connections + wipe).
- `src/storage/`: `idb.ts` (Db), `keyStore.ts` (AES-GCM wrapped key, `Unlocker` hook for WebAuthn), `repos.ts` (MachineRepo, IdbSnapshotCache; tombstones stop late saves after forget).
- `src/stores/`: Pinia `hub` (mirrors hub events), `update`. `src/update/`: version.json check, SW registration.
- E2E: `npm run e2e -w @ccm/pwa` (`e2e/run.mjs`, `e2e/peer` Rust fake desktop copied from the spike peer + snapshot/events/patch/revoke stdin commands).
