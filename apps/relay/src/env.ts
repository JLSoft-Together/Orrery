import type { Room } from "./room";

export interface Env {
  ROOM: DurableObjectNamespace<Room>;
  LIMITER: RateLimit;
  ANDROID_CERT_SHA256?: string;
  APPLE_APP_ID?: string;
}
