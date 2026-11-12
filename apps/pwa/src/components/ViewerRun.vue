<script setup lang="ts">
import { onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAsync } from '../lib/useAsync'
import { viewerData, type Commander } from '../lib/viewerApi'
import { parseRunLines, parseRunSession, parseRunSessions, samePath, type RunLine, type RunSession } from '../lib/viewerModel'
import type { JsonMap } from '../remote/types'
import AsyncState from './AsyncState.vue'
import StatusChip from './StatusChip.vue'

const props = defineProps<{ command: Commander; root: string; refresh: number; subscribe: (handler: (event: JsonMap) => void) => () => void }>()
const { t } = useI18n()
const sessions = shallowRef<Record<string, RunSession>>({})
const logs = shallowRef<Record<string, RunLine[]>>({})
const MAX_LINES = 400

const mine = (s: RunSession): boolean => s.cwd === '' || samePath(s.cwd, props.root)

const loaded = useAsync(async () => {
  const list = parseRunSessions(await viewerData(props.command, 'run.sessions'))
  const next: Record<string, RunSession> = {}
  for (const s of list) if (mine(s)) next[s.id] = s
  sessions.value = next
  return list.length
})

const off = props.subscribe((event) => {
  if (event.t !== 'run.event' || !event.event || typeof event.event !== 'object') return
  const body = event.event as JsonMap
  if (body.type === 'session') {
    const s = parseRunSession(body.session)
    if (s && mine(s)) sessions.value = { ...sessions.value, [s.id]: s }
  } else if (body.type === 'log' && typeof body.sessionId === 'string') {
    const id = body.sessionId
    const merged = [...(logs.value[id] ?? []), ...parseRunLines(body.lines)]
    logs.value = { ...logs.value, [id]: merged.length > MAX_LINES ? merged.slice(-MAX_LINES) : merged }
  }
})

const picked = ref<string | null>(null)
watch(
  () => props.refresh,
  () => void loaded.reload(),
)
onBeforeUnmount(off)

function tone(s: RunSession): 'working' | 'failed' | 'idle' {
  if (s.state === 'failed') return 'failed'
  return s.live ? 'working' : 'idle'
}
</script>

<template>
  <div class="run" data-testid="viewer-run">
    <AsyncState :blank="loaded.data.value === null" :error="loaded.error.value" :empty="Object.keys(sessions).length === 0" :empty-text="t('viewer.noRun')" @retry="loaded.reload">
      <ul class="list">
        <li v-for="s in Object.values(sessions)" :key="s.id" class="item" :data-testid="`run-${s.id}`">
          <button type="button" class="head" @click="picked = picked === s.id ? null : s.id">
            <span class="name">{{ s.name || s.id }}</span>
            <StatusChip :rail="tone(s) === 'working' ? 'working' : tone(s) === 'failed' ? 'failed' : 'idle'" />
          </button>
          <p class="state mono" data-testid="run-state">{{ s.state }}{{ s.progress ? ` · ${s.progress}` : '' }}</p>
          <div v-if="picked === s.id" class="log mono" data-testid="run-log">
            <p v-if="!(logs[s.id] ?? []).length" class="none">{{ t('viewer.noLog') }}</p>
            <span v-for="(line, i) in logs[s.id] ?? []" :key="i" class="line" :class="line.level">{{ line.text }}</span>
          </div>
        </li>
      </ul>
    </AsyncState>
  </div>
</template>

<style scoped>
.run {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  overflow-y: auto;
}

.list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0;
  padding: 0 16px 16px;
  list-style: none;
}

.item {
  padding: 12px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
}

.head {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 36px;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.name {
  flex: 1;
  min-width: 0;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.state {
  margin: 4px 0 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.log {
  display: flex;
  flex-direction: column;
  margin-top: 10px;
  padding: 10px 12px;
  max-height: 280px;
  overflow: auto;
  border-radius: var(--radius-control);
  background: var(--ccm-term-bg);
  color: var(--ccm-term-ink);
  font-size: 0.75rem;
  line-height: 1.5;
}

.line {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.line.error {
  color: var(--ccm-diff-del);
}

.none {
  margin: 0;
  color: var(--ccm-term-faint);
}
</style>
