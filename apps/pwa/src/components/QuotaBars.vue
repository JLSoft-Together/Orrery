<script setup lang="ts">
import type { RemoteQuota } from '@ccm/shared'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatPct, formatReset, quotaByAgent } from '../lib/quotaModel'
import { quotaTone } from '../lib/projectModel'

const props = defineProps<{ rows: readonly RemoteQuota[]; now: number }>()
const { t, locale } = useI18n()

const groups = computed(() => quotaByAgent(props.rows))

function windowLabel(row: RemoteQuota): string {
  return t(row.window === '5h' ? 'quota.window5h' : 'quota.windowWeek')
}

function resetText(row: RemoteQuota): string {
  return formatReset(row.resetAt, props.now, locale.value, t)
}
</script>

<template>
  <div v-if="groups.length" class="quota" data-testid="quota" :aria-label="t('quota.label')" role="group">
    <div v-for="group in groups" :key="group.agent" class="agent" :class="{ stale: group.stale }" :data-agent="group.agent" :data-stale="group.stale ? '1' : '0'">
      <span class="name">{{ t(`quota.agent.${group.agent}`) }}<span v-if="group.stale" class="old">{{ t('quota.stale') }}</span></span>
      <div class="rows">
        <div v-for="row in group.windows" :key="row.window" class="row" :data-window="row.window" :title="resetText(row)">
          <span class="win">{{ windowLabel(row) }}</span>
          <span class="track" role="progressbar" :aria-valuenow="row.pct" aria-valuemin="0" aria-valuemax="100" :aria-label="`${t(`quota.agent.${group.agent}`)} ${windowLabel(row)}`">
            <span class="fill" :class="`tone-${quotaTone(row.pct)}`" :style="{ width: `${row.pct}%` }" />
          </span>
          <span class="pct" :class="`tone-${quotaTone(row.pct)}`">{{ formatPct(row.pct, locale) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.quota {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.agent {
  display: grid;
  grid-template-columns: 6.5rem 1fr;
  align-items: start;
  gap: 8px;
}

.agent.stale {
  opacity: 0.62;
}

.name {
  display: flex;
  flex-direction: column;
  font-size: 0.8125rem;
  font-weight: 700;
  line-height: 1.3;
  padding-top: 1px;
}

.old {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--ccm-warning);
}

.rows {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.row {
  display: grid;
  grid-template-columns: 2.4rem 1fr 2.6rem;
  align-items: center;
  gap: 8px;
  font-size: 0.8125rem;
}

.win {
  color: var(--ccm-ink-muted);
}

.track {
  position: relative;
  height: 5px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--ccm-fill-strong);
}

.fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--ccm-accent);
  transition: width var(--ccm-dur-slow) var(--ccm-ease-standard), background-color var(--ccm-dur-base) var(--ccm-ease-standard);
}

.fill.tone-warn {
  background: var(--ccm-warning);
}

.fill.tone-bad {
  background: var(--ccm-error);
}

.pct {
  text-align: right;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.pct.tone-warn {
  color: var(--ccm-warning);
}

.pct.tone-bad {
  color: var(--ccm-error);
}
</style>
