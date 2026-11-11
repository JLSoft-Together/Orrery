export const MAX_ID_LEN = 128;

const MACHINE_ID = /^[A-Za-z0-9_-]{1,128}$/;
const PANE_KEY = /^[A-Za-z0-9_.:-]{1,128}$/;

export function validMachineId(value: unknown): value is string {
  return typeof value === "string" && MACHINE_ID.test(value);
}

export function validPaneKey(value: unknown): value is string {
  return typeof value === "string" && PANE_KEY.test(value);
}

export function validDeviceId(value: unknown): value is string {
  if (typeof value !== "string" || value.length === 0 || value.length > MAX_ID_LEN) return false;
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (c < 0x20 || (c >= 0x7f && c <= 0x9f)) return false;
  }
  return true;
}
