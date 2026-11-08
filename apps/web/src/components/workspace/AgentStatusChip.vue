<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ListChecks } from 'lucide-vue-next'
import type { WorkflowStatus } from '@ccm/shared'
import { isStatusStale, progressPct } from '../../lib/agentStatus'
import { now, relativeTime } from '../../lib/format'

/** Workflow phase + progress an agent writes to `.orrery/status.json` (docs §11ap). Stale ones go quiet (ink ladder, not opacity). */
const props = defineProps<{ status: WorkflowStatus }>()
const { t, locale } = useI18n()

const pct = computed(() => progressPct(props.status))
const stale = computed(() => isStatusStale(props.status, now.value))
const tip = computed(() =>
  [
    props.status.phase,
    props.status.note,
    pct.value === null ? '' : t('agentStatus.progress', { pct: pct.value }),
    stale.value ? t('agentStatus.stale') : t('agentStatus.updated', { when: relativeTime(props.status.updatedAt, locale.value, now.value) ?? '' }),
  ]
    .filter(Boolean)
    .join('\n'),
)
</script>

<template>
  <Transition name="ccm-fade" appear>
    <span
      class="inline-flex h-5 min-w-0 shrink items-center gap-1 rounded-full bg-fill px-2 text-2xs font-medium @max-[360px]:hidden"
      :class="stale ? 'text-ink-faint' : 'text-ink'"
      role="status"
      :title="tip"
      :aria-label="`${t('agentStatus.chip', { phase: status.phase })}${pct === null ? '' : `, ${t('agentStatus.progress', { pct })}`}`"
      :data-agent-status="stale ? 'stale' : 'live'"
    >
      <ListChecks :size="12" class="shrink-0" :class="stale ? 'text-ink-faint' : 'text-accent-ink'" aria-hidden="true" />
      <span class="max-w-[16ch] min-w-0 truncate">{{ status.phase }}</span>
      <template v-if="pct !== null">
        <span class="h-1 w-8 shrink-0 overflow-hidden rounded-full bg-line-strong @max-[520px]:hidden" aria-hidden="true">
          <span class="block h-full rounded-full transition-[width] duration-(--ccm-dur-base)" :class="stale ? 'bg-ink-faint' : 'bg-accent'" :style="{ width: `${pct}%` }" />
        </span>
        <span class="shrink-0 tabular-nums" aria-hidden="true">{{ pct }}%</span>
      </template>
    </span>
  </Transition>
</template>
