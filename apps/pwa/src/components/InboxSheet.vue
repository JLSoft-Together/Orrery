<script setup lang="ts">
import { Check, CircleHelp, FileWarning, ListChecks, ShieldAlert, StickyNote, TriangleAlert } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { inboxSections, inboxSince, type InboxGroup, type InboxRow } from '../lib/inboxModel'
import { formatElapsed } from '../lib/monitorModel'
import BottomSheet from './BottomSheet.vue'

const props = defineProps<{ rows: InboxRow[]; now: number }>()
const emit = defineEmits<{ pick: [row: InboxRow]; close: [] }>()
const { t } = useI18n()

const META: Record<InboxGroup, { icon: typeof Check; tone: string }> = {
  menu: { icon: ListChecks, tone: 'waiting' },
  permission: { icon: ShieldAlert, tone: 'waiting' },
  question: { icon: CircleHelp, tone: 'waiting' },
  clash: { icon: FileWarning, tone: 'warning' },
  error: { icon: TriangleAlert, tone: 'error' },
  note: { icon: StickyNote, tone: 'idle' },
  done: { icon: Check, tone: 'done' },
}

const sections = computed(() => inboxSections(props.rows))

function reasonText(row: InboxRow): string {
  return t(`inbox.reason.${row.reason}`, { file: row.file ?? '', peer: row.peer ?? '' })
}

function age(row: InboxRow): string {
  const at = inboxSince(row)
  return at === null ? '' : formatElapsed(Math.max(0, props.now - at), t)
}
</script>

<template>
  <BottomSheet :title="t('inbox.title')" testid="inbox-sheet" @close="emit('close')">
    <p v-if="sections.length === 0" class="empty" data-testid="inbox-empty">{{ t('inbox.empty') }}</p>
    <section v-for="s in sections" :key="s.group" class="group" :data-testid="`inbox-group-${s.group}`" :data-group="s.group">
      <h3 class="head" :class="`tone-${META[s.group].tone}`">
        <component :is="META[s.group].icon" :size="16" :stroke-width="2.2" aria-hidden="true" />
        <span>{{ t(`inbox.group.${s.group}`) }} {{ s.rows.length }}</span>
      </h3>
      <ul class="list">
        <li v-for="row in s.rows" :key="row.paneKey">
          <button type="button" class="row" :data-testid="`inbox-row-${row.paneKey}`" :data-reason="row.reason" @click="emit('pick', row)">
            <span class="ico" :class="`tone-${META[s.group].tone}`"><component :is="META[s.group].icon" :size="20" :stroke-width="2.2" aria-hidden="true" /></span>
            <span class="who">
              <span class="name">{{ row.title }}</span>
              <span class="sub">{{ reasonText(row) }}</span>
            </span>
            <span v-if="age(row)" class="age">{{ age(row) }}</span>
          </button>
        </li>
      </ul>
    </section>
  </BottomSheet>
</template>

<style scoped>
.group + .group {
  margin-top: 14px;
}

.head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 8px 4px;
  font-size: 0.875rem;
  font-weight: 800;
}

.tone-waiting {
  color: var(--ccm-waiting);
}

.tone-warning {
  color: var(--ccm-warning);
}

.tone-error {
  color: var(--ccm-error);
}

.tone-done {
  color: var(--ccm-done);
}

.tone-idle {
  color: var(--ccm-ink-muted);
}

.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 56px;
  padding: 10px 12px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-control);
  text-align: left;
}

.ico {
  flex: none;
  display: grid;
}

.who {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}

.sub {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.age {
  flex: none;
  font-size: 0.8125rem;
  font-weight: 700;
  color: var(--ccm-ink-muted);
  font-variant-numeric: tabular-nums;
}

.empty {
  margin: 24px 0;
  text-align: center;
  color: var(--ccm-ink-muted);
}
</style>
