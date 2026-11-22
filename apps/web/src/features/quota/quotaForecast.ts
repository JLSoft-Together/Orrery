import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { PlanLimits } from '@ccm/shared'
import { useExtrasStore } from '../../stores/extras'
import { forecastFullAt, pushSample, type QuotaSample } from './forecast'
import type { QuotaAgentId, QuotaKey } from '../../lib/next/quotaBar'

const id = (agent: QuotaAgentId, key: QuotaKey): string => `${agent}:${key}`

export const useQuotaForecastStore = defineStore('quotaForecast', () => {
  const extras = useExtrasStore()
  const samples = ref<Record<string, QuotaSample[]>>({})

  function take(agent: QuotaAgentId, limits: PlanLimits | null): void {
    if (!limits) return
    const at = Date.now()
    for (const [key, w] of [
      ['5h', limits.fiveHour],
      ['7d', limits.sevenDay],
    ] as const) {
      if (!w || !Number.isFinite(w.usedPct)) continue
      samples.value[id(agent, key)] = pushSample(samples.value[id(agent, key)] ?? [], at, w.usedPct)
    }
  }

  watch(() => extras.limits, (l) => take('claude', l), { immediate: true })
  watch(() => extras.codexLimits, (l) => take('codex', l), { immediate: true })

  function fullAt(agent: QuotaAgentId, key: QuotaKey, nowMs: number, resetsAt: number | null): number | null {
    return forecastFullAt(samples.value[id(agent, key)] ?? [], nowMs, resetsAt)
  }

  return { samples, fullAt }
})
