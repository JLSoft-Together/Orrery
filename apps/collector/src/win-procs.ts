/** One row of the Windows process table; `start` is the creation FILETIME, null when the process can't be opened. */
export interface WinProc {
  pid: number
  ppid: number
  name: string
  start: bigint | null
}

export interface ProcUsage {
  privateBytes: number | null
  workingSet: number | null
  cpuMs: number | null
}
