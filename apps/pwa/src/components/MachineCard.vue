<script setup lang="ts">
import { Clock, Laptop, LoaderCircle, MoonStar } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { MachineView } from '../remote/machineData'
import { statusCounts } from '../remote/summary'
import type { LinkState, Machine, RoutePolicy } from '../remote/types'
import LinkChip from './LinkChip.vue'

const props = defineProps<{ machine: Machine; link: LinkState; view: MachineView; rtt?: number }>()
const emit = defineEmits<{ policy: [policy: RoutePolicy]; forget: [] }>()
const { t, locale } = useI18n()

const counts = computed(() => statusCounts(props.view.snapshot))
const title = computed(() => props.machine.alias ?? props.machine.name)
const hello = computed(() => props.view.hello)
const subtitle = computed(() => {
  const version = typeof hello.value?.version === 'string' ? hello.value.version : null
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
    <header class="head">
      <span class="avatar" aria-hidden="true"><Laptop :size="20" :stroke-width="2" /></span>
      <div class="who">
        <h3 class="name" data-testid="machine-name">{{ title }}</h3>
        <p class="sub">{{ subtitle }}</p>
      </div>
      <LinkChip :link="link" :rtt="rtt" />
    </header>
    <div class="counts" data-testid="machine-counts">
      <template v-if="view.snapshot">
        <span v-if="counts.working" class="count c-working"><LoaderCircle :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.working', { n: counts.working }) }}</span>
        <span v-if="counts.waiting" class="count c-waiting"><Clock :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.waiting', { n: counts.waiting }) }}</span>
        <span v-if="counts.idle" class="count c-idle"><MoonStar :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('machines.idle', { n: counts.idle }) }}</span>
      </template>
      <span v-else class="muted" data-testid="machine-nodata">{{ t('machines.noData') }}</span>
    </div>
    <p v-if="when" class="freshness" :class="{ stale: view.stale }" data-testid="machine-freshness" :data-stale="view.stale ? '1' : '0'">
      {{ view.stale ? t('machines.stale', { time: when }) : t('machines.live', { time: when }) }}
    </p>
    <p v-if="link.reason !== 'none' && link.status !== 'ready'" class="reason" data-testid="machine-reason">{{ t(`reason.${link.reason}`) }}</p>
    <footer class="foot">
      <label class="route">
        <span class="label">{{ t('machines.route') }}</span>
        <select :value="machine.policy" data-testid="route-select" @change="onPolicy">
          <option v-for="policy in policies" :key="policy" :value="policy">{{ t(`policy.${policy}`) }}</option>
        </select>
      </label>
      <button type="button" class="btn btn-danger forget" data-testid="machine-forget" @click="emit('forget')">{{ t('machines.forget') }}</button>
    </footer>
  </article>
</template>

<style scoped>
.machine {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
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
  background: var(--ccm-raised);
  color: var(--ccm-accent-ink);
  flex: none;
}

.who {
  flex: 1;
  min-width: 0;
}

.name {
  margin: 0;
  font-size: 17px;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub,
.freshness,
.reason {
  margin: 0;
  font-size: 13px;
  color: var(--ccm-ink-muted);
}

.freshness.stale {
  color: var(--ccm-warning);
}

.reason {
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
  padding: 4px 10px;
  border-radius: var(--radius-chip);
  border: 1px solid currentColor;
  font-size: 13px;
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
  font-size: 13px;
}

.foot {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
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

.forget {
  min-height: 44px;
}
</style>
