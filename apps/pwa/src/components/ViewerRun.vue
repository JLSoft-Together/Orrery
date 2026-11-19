<script setup lang="ts">
import { Play, RefreshCw, RotateCcw, Square } from 'lucide-vue-next'
import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAction } from '../lib/useAction'
import { useAsync } from '../lib/useAsync'
import { reloadRun, startRun, stopRun } from '../lib/viewerActions'
import { viewerData, type Commander } from '../lib/viewerApi'
import { firstReady, parseRunConfigs, parseRunDevices, parseRunLines, parseRunSession, parseRunSessions, samePath, type RunConfig, type RunDevice, type RunLine, type RunSession } from '../lib/viewerModel'
import type { JsonMap } from '../remote/types'
import AsyncState from './AsyncState.vue'
import NoticeLine from './NoticeLine.vue'
import StatusChip from './StatusChip.vue'

const props = defineProps<{ command: Commander; root: string; refresh: number; editable: boolean; subscribe: (handler: (event: JsonMap) => void) => () => void }>()
const { t } = useI18n()
const sessions = shallowRef<Record<string, RunSession>>({})
const logs = shallowRef<Record<string, RunLine[]>>({})
const configs = shallowRef<RunConfig[]>([])
const devices = shallowRef<RunDevice[]>([])
const configId = ref<string | null>(null)
const deviceId = ref<string | null>(null)
const MAX_LINES = 400
const { busy, notice, perform } = useAction()

const mine = (s: RunSession): boolean => s.cwd === '' || samePath(s.cwd, props.root)

const loaded = useAsync(async () => {
  const list = parseRunSessions(await viewerData(props.command, 'run.sessions'))
  const next: Record<string, RunSession> = {}
  for (const s of list) if (mine(s)) next[s.id] = s
  sessions.value = next
  configs.value = parseRunConfigs(await viewerData(props.command, 'run.configs', { root: props.root }).catch(() => []))
  devices.value = parseRunDevices(await viewerData(props.command, 'run.devices').catch(() => []))
  if (!configs.value.some((c) => c.id === configId.value)) configId.value = configs.value[0]?.id ?? null
  if (!devices.value.some((d) => d.id === deviceId.value && d.ready)) deviceId.value = firstReady(devices.value)
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
const ordered = computed(() => Object.values(sessions.value).sort((a, b) => Number(b.live) - Number(a.live)))
watch(
  () => props.refresh,
  () => void loaded.reload(),
)
onBeforeUnmount(off)

function tone(s: RunSession): 'working' | 'failed' | 'idle' {
  if (s.state === 'failed') return 'failed'
  return s.live ? 'working' : 'idle'
}

async function start(): Promise<void> {
  const id = configId.value
  if (!id) return
  const result = await perform(() => startRun(props.command, props.root, id, deviceId.value), t('run.started'))
  if (result?.kind === 'done') {
    const sid = result.data.sessionId
    if (typeof sid === 'string') picked.value = sid
    await loaded.reload()
  }
}

async function stop(s: RunSession): Promise<void> {
  await perform(() => stopRun(props.command, s.id))
}

async function reload(s: RunSession, hot: boolean): Promise<void> {
  await perform(() => reloadRun(props.command, s.id, hot))
}
</script>

<template>
  <div class="run" data-testid="viewer-run">
    <AsyncState :blank="loaded.data.value === null" :error="loaded.error.value" :empty="ordered.length === 0 && configs.length === 0" :empty-text="t('viewer.noRun')" @retry="loaded.reload">
      <NoticeLine :notice="notice" />
      <section v-if="editable && configs.length" class="launch" data-testid="run-launch">
        <label class="field"><span>{{ t('run.config') }}</span>
          <select v-model="configId" class="select" data-testid="run-config"><option v-for="c in configs" :key="c.id" :value="c.id">{{ c.name }}</option></select>
        </label>
        <label class="field"><span>{{ t('run.device') }}</span>
          <select v-model="deviceId" class="select" data-testid="run-device">
            <option :value="null">{{ t('run.noDevice') }}</option>
            <option v-for="d in devices" :key="d.id" :value="d.id" :disabled="!d.ready">{{ d.name }}</option>
          </select>
        </label>
        <button type="button" class="btn btn-primary" :disabled="busy || !configId" data-testid="run-start" @click="start"><Play :size="20" :stroke-width="2" aria-hidden="true" />{{ t('run.start') }}</button>
      </section>
      <TransitionGroup tag="ul" name="run-item" class="list">
        <li v-for="s in ordered" :key="s.id" class="item" :data-testid="`run-${s.id}`" :data-state="s.state">
          <button type="button" class="head" @click="picked = picked === s.id ? null : s.id">
            <span class="name">{{ s.name || s.id }}</span>
            <StatusChip :rail="tone(s) === 'working' ? 'working' : tone(s) === 'failed' ? 'failed' : 'idle'" />
          </button>
          <p class="state mono" data-testid="run-state">{{ s.state }}{{ s.progress ? `, ${s.progress}` : '' }}</p>
          <div v-if="editable && s.live" class="ctl">
            <button type="button" class="btn" :disabled="busy" :data-testid="`run-reload-${s.id}`" @click="reload(s, true)"><RefreshCw :size="18" :stroke-width="2" aria-hidden="true" />{{ t('run.reload') }}</button>
            <button type="button" class="btn" :disabled="busy" :data-testid="`run-restart-${s.id}`" @click="reload(s, false)"><RotateCcw :size="18" :stroke-width="2" aria-hidden="true" />{{ t('run.restart') }}</button>
            <button type="button" class="btn btn-danger" :disabled="busy" :data-testid="`run-stop-${s.id}`" @click="stop(s)"><Square :size="18" :stroke-width="2" aria-hidden="true" />{{ t('run.stop') }}</button>
          </div>
          <div v-if="picked === s.id" class="log mono" data-testid="run-log">
            <p v-if="!(logs[s.id] ?? []).length" class="none">{{ t('viewer.noLog') }}</p>
            <span v-for="(line, i) in logs[s.id] ?? []" :key="i" class="line" :class="line.level">{{ line.text }}</span>
          </div>
        </li>
      </TransitionGroup>
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

.launch {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0 16px 12px;
  padding: 12px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.select {
  min-height: 44px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-size: 16px;
}

.list {
  position: relative;
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

.ctl {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.ctl .btn {
  flex: 1;
  min-width: 0;
  padding: 0 8px;
  font-size: 0.875rem;
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
  color: var(--ccm-term-del);
}

.none {
  margin: 0;
  color: var(--ccm-term-faint);
}

.run-item-enter-active,
.run-item-leave-active,
.run-item-move {
  transition: opacity 220ms var(--ease-out), transform 220ms var(--ease-out);
}

.run-item-enter-from,
.run-item-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
