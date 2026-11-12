<script setup lang="ts">
import { Bell, BellOff, ChevronRight, Clock, Laptop, LoaderCircle, MoonStar } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MachineView } from '../remote/machineData'
import { statusCounts } from '../remote/summary'
import type { LinkState, Machine, RoutePolicy } from '../remote/types'
import LinkChip from './LinkChip.vue'

const props = defineProps<{ machine: Machine; link: LinkState; view: MachineView; rtt?: number; bell: boolean }>()
const emit = defineEmits<{ open: []; policy: [policy: RoutePolicy]; bell: [on: boolean] }>()
const { t, locale } = useI18n()

const counts = computed(() => statusCounts(props.view.snapshot))
const title = computed(() => props.machine.alias ?? props.machine.name)
const subtitle = computed(() => {
  const version = typeof props.view.hello?.version === 'string' ? props.view.hello.version : null
  return version ? `Orrery ${version}` : t('machines.paired', { date: formatDate(props.machine.pairedAt) })
})
const when = computed(() => (props.view.updatedAt ? formatTime(props.view.updatedAt) : null))
const policies: RoutePolicy[] = ['auto', 'lan', 'relay']

function formatTime(ms: number): string {
  return new Intl.DateTimeFormat(locale.value, { hour: '2-digit', minute: '2-digit' }).format(ms)
}

function formatDate(ms: number): string {
  return new Intl.DateTimeFormat(locale.value, { day: '2-digit', month: '2-digit' }).format(ms)
}

function onPolicy(event: Event): void {
  emit('policy', (event.target as HTMLSelectElement).value as RoutePolicy)
}
</script>

<template>
  <article class="card machine" :data-testid="`machine-${machine.id}`">
    <button type="button" class="open" data-testid="machine-open" @click="emit('open')">
      <span class="head">
        <span class="avatar" aria-hidden="true"><Laptop :size="20" :stroke-width="2" /></span>
        <span class="who">
          <span class="name" data-testid="machine-name">{{ title }}</span>
          <span class="sub">{{ subtitle }}</span>
        </span>
        <LinkChip :link="link" :rtt="rtt" />
        <ChevronRight :size="18" :stroke-width="2" class="chev" aria-hidden="true" />
      </span>
      <span class="counts" data-testid="machine-counts">
        <template v-if="view.snapshot">
          <span v-if="counts.working" class="count c-working"><LoaderCircle :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.working', { n: counts.working }) }}</span>
          <span v-if="counts.waiting" class="count c-waiting"><Clock :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.waiting', { n: counts.waiting }) }}</span>
          <span v-if="counts.idle" class="count c-idle"><MoonStar :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.idle', { n: counts.idle }) }}</span>
        </template>
        <span v-else class="muted" data-testid="machine-nodata">{{ t('machines.noData') }}</span>
      </span>
      <span v-if="when" class="note" :class="{ stale: view.stale }" data-testid="machine-freshness" :data-stale="view.stale ? '1' : '0'">
        {{ view.stale ? t('machines.stale', { time: when }) : t('machines.live', { time: when }) }}
      </span>
      <span v-if="link.reason !== 'none' && link.status !== 'ready'" class="note warn" data-testid="machine-reason">{{ t(`reason.${link.reason}`) }}</span>
    </button>
    <footer class="foot">
      <label class="route">
        <span class="label">{{ t('machines.route') }}</span>
        <select :value="machine.policy" data-testid="route-select" @change="onPolicy">
          <option v-for="policy in policies" :key="policy" :value="policy">{{ t(`policy.${policy}`) }}</option>
        </select>
      </label>
      <button type="button" class="bell" :class="{ on: bell }" :aria-pressed="bell" :aria-label="t(bell ? 'machines.bellOn' : 'machines.bellOff')" data-testid="machine-bell" @click="emit('bell', !bell)">
        <component :is="bell ? Bell : BellOff" :size="20" :stroke-width="2" aria-hidden="true" />
      </button>
    </footer>
  </article>
</template>

<style scoped>
.machine {
  display: flex;
  flex-direction: column;
  transition: border-color 180ms var(--ease-out);
}

.open {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  padding: 14px 14px 8px;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.avatar {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: var(--radius-control);
  background: var(--ccm-selection);
  color: var(--ccm-accent-ink);
  flex: none;
}

.who {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.name {
  font-size: 1.0625rem;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub,
.note {
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.chev {
  color: var(--ccm-ink-faint);
  flex: none;
}

.note.stale,
.note.warn {
  color: var(--ccm-warning);
}

.counts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.count {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px;
  border-radius: var(--radius-chip);
  border: 1px solid color-mix(in srgb, currentColor 38%, transparent);
  font-size: 0.8125rem;
  font-weight: 600;
}

.c-working {
  color: var(--ccm-working);
}

.c-waiting {
  color: var(--ccm-waiting);
}

.c-idle {
  color: var(--ccm-idle);
}

.muted {
  color: var(--ccm-ink-faint);
  font-size: 0.8125rem;
}

.foot {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 14px 12px;
}

.route {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

select {
  min-height: 44px;
  padding: 0 12px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
}

.bell {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink-muted);
}

.bell.on {
  color: var(--ccm-accent-ink);
  border-color: var(--ccm-accent);
}
</style>
