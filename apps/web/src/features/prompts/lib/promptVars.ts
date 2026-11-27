export const PROMPT_VARS = ['branch', 'project', 'cwd'] as const
export type PromptVar = (typeof PROMPT_VARS)[number]
export type VarContext = Partial<Record<PromptVar, string>>

const VAR = /\{(branch|project|cwd)\}/g

export const usedVars = (body: string): PromptVar[] => [...new Set([...body.matchAll(VAR)].map((m) => m[1] as PromptVar))]

export function expandVars(body: string, ctx: VarContext): { text: string; missing: PromptVar[] } {
  const missing = new Set<PromptVar>()
  const text = body.replace(VAR, (whole, name: PromptVar) => {
    const value = ctx[name]
    if (value) return value
    missing.add(name)
    return whole
  })
  return { text, missing: [...missing] }
}
