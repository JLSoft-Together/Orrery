<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { now } from '../../lib/format'
import { QUOTA_TONE_CLASS, quotaAgents, resetLabel, type QuotaAgent } from '../../lib/next/quotaBar'
import { useExtrasStore } from '../../stores/extras'

const { t, locale } = useI18n()
const extras = useExtrasStore()

const agents = computed(() => quotaAgents(extras.limits, extras.codexLimits, now.value))

const keyLabel = (key: '5h' | '7d'): string => t(key === '5h' ? 'mist.shell.quota5h' : 'mist.shell.quotaWeek')

function tip(agent: QuotaAgent): string {
  const parts = agent.windows.map((w) => {
    const reset = resetLabel(w.resetsAt, now.value, locale.value)
    return reset ? t('mist.shell.quotaTip', { key: keyLabel(w.key), pct: w.used, at: reset }) : `${keyLabel(w.key)} ${w.used}%`
  })
  return [agent.name, ...parts, agent.stale ? t('mist.shell.quotaStale') : ''].filter(Boolean).join(', ')
}
</script>

<template>
  <footer class="ccm-next-footer flex h-6 min-w-0 items-center gap-4 px-3 text-2xs text-ink-muted" :aria-label="t('mist.shell.quotaLabel')" data-next-footer>
    <TransitionGroup name="ccm-next-quota" tag="div" class="flex min-w-0 flex-1 items-center gap-4 overflow-hidden" data-next-quota>
      <p v-for="a in agents" :key="a.id" class="flex min-w-0 shrink-0 items-center gap-2 whitespace-nowrap tabular-nums" :class="a.stale ? 'opacity-60' : ''" :title="tip(a)" :data-quota-agent="a.id" :data-stale="a.stale || undefined">
        <span class="font-semibold text-ink">{{ a.name }}</span>
        <span v-for="w in a.windows" :key="w.key" class="inline-flex items-center gap-1.5" :data-quota="w.key" :data-tone="w.tone">
          <span>{{ keyLabel(w.key) }}</span>
          <span class="h-1 w-10 shrink-0 overflow-hidden rounded-sm bg-line-strong" aria-hidden="true">
            <span class="ccm-next-quota-fill block h-full" :class="QUOTA_TONE_CLASS[w.tone]" :style="{ width: `${w.used}%` }" />
          </span>
          <span class="font-semibold" :class="w.tone === 'critical' ? 'text-st-error' : 'text-ink'">{{ w.used }}%</span>
          <span v-if="resetLabel(w.resetsAt, now, locale)" class="text-ink-faint">{{ t('mist.shell.quotaReset', { at: resetLabel(w.resetsAt, now, locale) }) }}</span>
        </span>
      </p>
    </TransitionGroup>
    <span v-if="!agents.length" class="truncate text-ink-faint" data-quota-empty>{{ t('mist.shell.noQuota') }}</span>
  </footer>
</template>

<style>
:root[data-ui='next'] .ccm-next-quota-fill {
  transition: width var(--ccm-dur-base) var(--ccm-ease-enter);
}
:root[data-ui='next'] .ccm-next-quota-enter-active,
:root[data-ui='next'] .ccm-next-quota-leave-active {
  transition:
    opacity var(--ccm-dur-base) var(--ccm-ease-enter),
    transform var(--ccm-dur-base) var(--ccm-ease-enter);
}
:root[data-ui='next'] .ccm-next-quota-enter-from,
:root[data-ui='next'] .ccm-next-quota-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
:root[data-ui='next'][data-motion='off'] .ccm-next-quota-fill,
:root[data-ui='next'][data-motion='off'] .ccm-next-quota-enter-active,
:root[data-ui='next'][data-motion='off'] .ccm-next-quota-leave-active {
  transition: none;
}
</style>
