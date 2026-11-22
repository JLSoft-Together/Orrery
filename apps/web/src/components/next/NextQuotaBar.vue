<script setup lang="ts">
import { computed, ref } from 'vue'
import { Code, TrendingUp } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { useDismiss } from '../../composables/useDismiss'
import { now } from '../../lib/format'
import { LIMIT_ALERT_LEFT, LIMIT_WARN_LEFT } from '../../lib/headerStatus'
import { QUOTA_TONE_CLASS, quotaAgents, resetLabel, resetRelative, type QuotaAgent, type QuotaAgentId } from '../../lib/next/quotaBar'
import { useQuotaForecastStore } from '../../features/quota/quotaForecast'
import FooterSysChip from '../../features/footerSys/FooterSysChip.vue'
import ShareModeChip from '../../features/shareMode/ShareModeChip.vue'
import { useExtrasStore } from '../../stores/extras'
import { BRAND_ICONS } from '../extensions/brandIcons'

const ICONS = { claude: { is: BRAND_ICONS.anthropic, cls: 'text-agent-claude' }, codex: { is: Code, cls: 'text-agent-codex' } } as const
const WARN_AT = 100 - LIMIT_WARN_LEFT
const CRITICAL_AT = 100 - LIMIT_ALERT_LEFT

const { t, locale } = useI18n()
const extras = useExtrasStore()
const forecast = useQuotaForecastStore()

const agents = computed(() => quotaAgents(extras.limits, extras.codexLimits, now.value, (agent, key, resetsAt) => forecast.fullAt(agent, key, now.value, resetsAt)))
const open = ref(false)
const popover = ref<HTMLElement | null>(null)

useDismiss({ active: open, inside: () => [popover.value], dismiss: () => (open.value = false), escape: true, ignore: '[data-quota-agent]' })

const keyLabel = (key: '5h' | '7d'): string => t(key === '5h' ? 'mist.shell.quota5h' : 'mist.shell.quotaWeek')

function tip(agent: QuotaAgent): string {
  const parts = agent.windows.map((w) => {
    const reset = resetLabel(w.resetsAt, now.value, locale.value)
    const base = reset ? t('mist.shell.quotaTip', { key: keyLabel(w.key), pct: w.used, at: reset }) : `${keyLabel(w.key)} ${w.used}%`
    return w.fullAt === null ? base : `${base}, ${fullText(w.fullAt)}`
  })
  return [agent.name, ...parts, agent.stale ? t('mist.shell.quotaStale') : ''].filter(Boolean).join(', ')
}

function fullText(fullAt: number): string {
  return t('v105.quota.forecast', { at: resetLabel(fullAt, now.value, locale.value) })
}

function resetText(resetsAt: number | null): string {
  const rel = resetRelative(resetsAt, now.value, {
    d: t('v105.quota.unitD'),
    h: t('v105.quota.unitH'),
    min: t('v105.quota.unitMin'),
    lessThanMin: t('v105.quota.lessThanMin'),
  })
  if (!rel) return t('v105.quota.resetUnknown')
  return t('v105.quota.resetIn', { rel, abs: resetLabel(resetsAt, now.value, locale.value) })
}

const openFor = ref<QuotaAgentId | null>(null)
function pick(id: QuotaAgentId): void {
  if (open.value && openFor.value === id) {
    open.value = false
    return
  }
  openFor.value = id
  open.value = true
}
</script>

<template>
  <footer class="ccm-next-footer relative flex h-6 min-w-0 items-center gap-4 px-3 text-2xs text-ink-muted" :aria-label="t('mist.shell.quotaLabel')" data-next-footer>
    <TransitionGroup name="ccm-next-quota" tag="div" class="flex min-w-0 flex-1 items-center gap-3 overflow-hidden" data-next-quota>
      <button
        v-for="a in agents"
        :key="a.id"
        type="button"
        class="group/quota flex min-w-0 shrink-0 cursor-pointer items-center gap-2 whitespace-nowrap rounded-control px-1.5 py-0.5 tabular-nums outline-none transition-colors duration-(--ccm-dur-instant) hover:bg-accent-soft focus-visible:ring-2 focus-visible:ring-accent"
        :class="a.stale ? 'opacity-60' : ''"
        :title="tip(a)"
        :aria-label="t('v105.quota.open', { name: a.name })"
        aria-haspopup="dialog"
        :aria-expanded="open && openFor === a.id"
        :data-quota-agent="a.id"
        :data-stale="a.stale || undefined"
        @click="pick(a.id)"
      >
        <component :is="ICONS[a.id].is" :size="12" class="shrink-0" :class="ICONS[a.id].cls" aria-hidden="true" />
        <span v-for="w in a.windows" :key="w.key" class="inline-flex items-center gap-1.5" :data-quota="w.key" :data-tone="w.tone">
          <span
            class="h-1 w-10 shrink-0 overflow-hidden rounded-sm bg-line-strong"
            :class="w.tone === 'ok' ? 'hidden group-hover/quota:block group-focus-visible/quota:block' : ''"
            aria-hidden="true"
            data-quota-track
          >
            <span class="ccm-next-quota-fill block h-full" :class="QUOTA_TONE_CLASS[w.tone]" :style="{ width: `${w.used}%` }" />
          </span>
          <span class="font-semibold" :class="w.tone === 'critical' ? 'text-st-error' : w.tone === 'warn' ? 'text-warning' : 'text-ink'">{{ w.used }}%</span>
          <span v-if="w.tone === 'critical'" class="font-semibold text-st-error" data-quota-level>{{ t('v105.quota.critical') }}</span>
          <TrendingUp v-if="w.fullAt !== null && w.tone !== 'critical'" :size="11" class="shrink-0 text-warning" :aria-label="t('v105.quota.forecastShort')" data-quota-forecast-icon />
        </span>
      </button>
    </TransitionGroup>
    <Transition name="ccm-next-pop">
      <div
        v-if="open && agents.length"
        ref="popover"
        role="dialog"
        :aria-label="t('v105.quota.title')"
        class="glass-pop absolute bottom-full left-2 z-50 mb-1 flex w-72 max-w-[calc(100vw-1rem)] flex-col gap-3 rounded-[14px] p-3 text-xs text-ink"
        data-quota-popover
      >
        <section v-for="a in agents" :key="a.id" class="flex flex-col gap-2" :class="a.stale ? 'opacity-70' : ''" :data-quota-detail="a.id">
          <h3 class="flex items-center gap-1.5 text-xs font-semibold">
            <component :is="ICONS[a.id].is" :size="12" class="shrink-0" :class="ICONS[a.id].cls" aria-hidden="true" />
            {{ a.name }}
          </h3>
          <p v-if="a.stale" class="text-2xs text-ink-faint" data-quota-detail-stale>{{ t('v105.quota.stale') }}</p>
          <div v-for="w in a.windows" :key="w.key" class="flex flex-col gap-1" :data-quota-window="w.key" :data-tone="w.tone">
            <div class="flex items-baseline gap-2">
              <span class="font-medium">{{ keyLabel(w.key) }}</span>
              <span class="font-semibold tabular-nums" :class="w.tone === 'critical' ? 'text-st-error' : w.tone === 'warn' ? 'text-warning' : 'text-ink'">{{ w.used }}%</span>
              <span class="ml-auto text-2xs font-semibold" :class="w.tone === 'critical' ? 'text-st-error' : w.tone === 'warn' ? 'text-warning' : 'text-ink-muted'" data-quota-level-word>{{ t(`v105.quota.level.${w.tone}`) }}</span>
            </div>
            <div class="relative h-1.5 w-full overflow-hidden rounded-sm bg-line-strong" aria-hidden="true">
              <span class="block h-full" :class="QUOTA_TONE_CLASS[w.tone]" :style="{ width: `${w.used}%` }" />
              <span class="absolute inset-y-0 w-px bg-ink-faint" :style="{ left: `${WARN_AT}%` }" />
              <span class="absolute inset-y-0 w-px bg-ink-faint" :style="{ left: `${CRITICAL_AT}%` }" />
            </div>
            <span class="text-2xs text-ink-muted" data-quota-reset>{{ resetText(w.resetsAt) }}</span>
            <span v-if="w.fullAt !== null" class="text-2xs font-semibold text-warning" data-quota-forecast>{{ fullText(w.fullAt) }}</span>
          </div>
        </section>
      </div>
    </Transition>
    <ShareModeChip />
    <FooterSysChip />
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
