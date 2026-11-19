<script setup lang="ts">
import { Bell, BellOff, Check, ChevronDown, ChevronRight, CircleAlert, Clock, Laptop, LoaderCircle, MoonStar } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { parseSessions } from '../lib/monitorModel'
import { cardState, machineSubtitle, railCounts } from '../lib/projectModel'
import { quotaOf } from '../lib/quotaModel'
import type { MachineView } from '../remote/machineData'
import type { LinkState, Machine, RoutePolicy } from '../remote/types'
import LinkChip from './LinkChip.vue'
import QuotaBars from './QuotaBars.vue'

const props = defineProps<{ machine: Machine; link: LinkState; view: MachineView; rtt?: number; bell: boolean; now: number }>()
const emit = defineEmits<{ open: []; policy: [policy: RoutePolicy]; bell: [on: boolean] }>()
const { t, locale } = useI18n()

const counts = computed(() => railCounts(parseSessions(props.view.snapshot)))
const quota = computed(() => quotaOf(props.view.snapshot, props.now))
const title = computed(() => props.machine.alias ?? props.machine.name)
const subtitle = computed(() => machineSubtitle(props.view.hello) ?? t('machines.paired', { date: formatDate(props.machine.pairedAt) }))
const when = computed(() => (props.view.updatedAt ? formatTime(props.view.updatedAt) : null))
const state = computed(() => cardState(props.link.status, props.view.snapshot !== null))
const old = computed(() => props.view.stale && when.value !== null)
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
  <article class="card machine" :class="{ lost: state === 'lost' }" :data-state="state" :data-testid="`machine-${machine.id}`">
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
          <span v-if="counts.asked" class="count c-waiting"><Clock :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.waiting', { n: counts.asked }) }}</span>
          <span v-if="counts.failed" class="count c-error"><CircleAlert :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.failed', { n: counts.failed }) }}</span>
          <span v-if="counts.done" class="count c-done"><Check :size="14" :stroke-width="2.4" aria-hidden="true" />{{ t('machines.done', { n: counts.done }) }}</span>
          <span v-if="counts.idle" class="count c-idle"><MoonStar :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.idle', { n: counts.idle }) }}</span>
        </template>
        <span v-else class="muted" data-testid="machine-nodata">{{ t('machines.noData') }}</span>
      </span>
    </button>
    <div v-if="quota.length" class="quota-wrap" data-testid="machine-quota">
      <QuotaBars :rows="quota" :now="now" />
    </div>
    <footer class="foot">
      <span class="note-col">
        <span v-if="link.reason !== 'none' && link.status !== 'ready'" class="note warn" data-testid="machine-reason">{{ t(`reason.${link.reason}`) }}</span>
        <span v-if="when" class="note" :class="{ stale: old }" data-testid="machine-freshness" :data-stale="view.stale ? '1' : '0'">
          <Clock v-if="old" :size="14" :stroke-width="2.2" aria-hidden="true" />
          {{ view.stale ? t('machines.stale', { time: when }) : t('machines.live', { time: when }) }}
        </span>
      </span>
      <label class="route">
        <span class="sr-only">{{ t('machines.route') }}</span>
        <select :value="machine.policy" :aria-label="t('machines.route')" data-testid="route-select" @change="onPolicy">
          <option v-for="policy in policies" :key="policy" :value="policy">{{ t(`policy.${policy}`) }}</option>
        </select>
        <ChevronDown :size="16" :stroke-width="2.2" class="sel-chev" aria-hidden="true" />
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
  transition:
    opacity var(--ccm-dur-slow) var(--ccm-ease-standard),
    border-color var(--ccm-dur-base) var(--ccm-ease-standard);
}

.machine.lost {
  opacity: 0.74;
}

.open {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
  padding: 16px 16px 14px;
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
  width: 44px;
  height: 44px;
  border-radius: var(--radius-control);
  background: var(--ccm-accent-soft);
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
  font-size: 1.1875rem;
  font-weight: 800;
  letter-spacing: -0.01em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub {
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chev {
  color: var(--ccm-ink-faint);
  flex: none;
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
  min-height: 28px;
  padding: 0 11px;
  border-radius: var(--radius-chip);
  background: color-mix(in srgb, currentColor 12%, transparent);
  font-size: 0.8125rem;
  font-weight: 700;
  animation: chip-in var(--ccm-dur-base) var(--ccm-ease-enter) both;
}

.c-working {
  color: var(--ccm-working);
}

.c-waiting {
  color: var(--ccm-waiting);
}

.c-error {
  color: var(--ccm-error);
}

.c-done {
  color: var(--ccm-done);
}

.c-idle {
  color: var(--ccm-idle);
}

.muted {
  color: var(--ccm-ink-faint);
  font-size: 0.8125rem;
}

.quota-wrap {
  padding: 12px 16px;
  border-top: 1px solid var(--ccm-line);
}

.foot {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px 14px;
  border-top: 1px solid var(--ccm-line);
}

.note-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.note {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.note.stale,
.note.warn {
  color: var(--ccm-warning);
}

.route {
  position: relative;
  display: flex;
  align-items: center;
}

select {
  appearance: none;
  -webkit-appearance: none;
  min-height: 44px;
  padding: 0 32px 0 14px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  font-weight: 600;
}

.sel-chev {
  position: absolute;
  right: 10px;
  color: var(--ccm-ink-muted);
  pointer-events: none;
}

.bell {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink-muted);
  transition:
    color var(--ccm-dur-base) var(--ccm-ease-standard),
    border-color var(--ccm-dur-base) var(--ccm-ease-standard);
}

.bell.on {
  color: var(--ccm-accent-ink);
  border-color: var(--ccm-accent);
}

@keyframes chip-in {
  from {
    opacity: 0;
    transform: scale(0.92);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
</style>
