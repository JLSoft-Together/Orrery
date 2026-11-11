import type { Env as RelayEnv } from "./env";

declare global {
  namespace Cloudflare {
    interface Env extends RelayEnv {}
  }
}
