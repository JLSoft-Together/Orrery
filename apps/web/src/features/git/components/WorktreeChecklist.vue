<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bot, CircleCheck, CircleDashed, CircleX, Clock, LoaderCircle, Play, Plus, X } from 'lucide-vue-next'
import { checklist, type ChecklistCheck, type ChecklistError, type ChecklistItem, type ChecklistReq } from '../../../core/lib/preview'
import { useSettingsStore } from '../../../core/stores/settings'
import { scriptsIn } from '../../workspace/lib/projects'
import { useWorkspaceStore } from '../../workspace/stores/workspace'
import { checkView, defaultScript, isCheckLive, mergeWarning, onCheckDone, openItems, runCheck, scriptOptions, waitShellResult, type CheckView, type MergeWarning } from '../lib/checklist'

const props = defineProps<{ dir: string; cwd: string }>()
const emit = defineEmits<{ warning: [value: MergeWarning | null] }>()

const { t } = useI18n()
const workspace = useWorkspaceStore()
const settings = useSettingsStore()

const items = ref<ChecklistItem[]>([])
const check = ref<ChecklistCheck>({ script: '', state: 'idle', at: 0, head: '' })
const head = ref('')
const options = ref<{ run: string; name: string }[]>([])
const text = ref('')
const busy = ref(false)
const running = ref(false)
const error = ref<ChecklistError | null>(null)
const ready = ref(false)

const view = computed<CheckView>(() => checkView(check.value, head.value, running.value || isCheckLive(props.dir)))
const open = computed(() => openItems(items.value).length)
const scripts = computed(() => (check.value.script && !options.value.some((o) => o.run === check.value.script) ? [{ run: check.value.script, name: check.value.script }, ...options.value] : options.value))

const viewIcon = { none: CircleDashed, idle: CircleDashed, running: LoaderCircle, pass: CircleCheck, fail: CircleX, stale: Clock } as const
const viewTone: Record<CheckView, string> = { none: 'text-ink-faint', idle: 'text-ink-muted', running: 'text-ink-muted', pass: 'text-success', fail: 'text-st-error', stale: 'text-ink-muted' }

function apply(reply: Awaited<ReturnType<typeof checklist>>): boolean {
  if (!reply.ok) {
    error.value = reply.error
    return false
  }
  error.value = null
  items.value = reply.items
  check.value = reply.check
  head.value = reply.head
  return true
}

async function send(req: ChecklistReq): Promise<boolean> {
  return apply(await checklist(props.cwd, props.dir, req))
}

async function load(): Promise<void> {
  await send({ op: 'list' })
  ready.value = true
  const res = (await scriptsIn(props.dir)) ?? (await scriptsIn(props.cwd))
  options.value = scriptOptions(res?.groups ?? [])
  if (!check.value.script) {
    const pick = defaultScript(options.value)
    if (pick) await send({ op: 'setScript', script: pick })
  }
}

async function add(): Promise<void> {
  const value = text.value.trim()
  if (!value || busy.value) return
  busy.value = true
  if (await send({ op: 'add', text: value })) text.value = ''
  busy.value = false
}

async function toggle(item: ChecklistItem): Promise<void> {
  await send({ op: 'toggle', id: item.id, done: !item.done })
}

async function remove(item: ChecklistItem): Promise<void> {
  await send({ op: 'remove', id: item.id })
}

async function pickScript(event: Event): Promise<void> {
  await send({ op: 'setScript', script: (event.target as HTMLSelectElement).value })
}

async function run(): Promise<void> {
  if (running.value || !check.value.script) return
  running.value = true
  const dir = props.dir
  await runCheck(dir, check.value.script, {
    head: async () => {
      const reply = await checklist(props.cwd, dir, { op: 'list' })
      return reply.ok ? reply.head : ''
    },
    report: async (state, at) => {
      const reply = await checklist(props.cwd, dir, { op: 'setCheck', state, head: at })
      if (dir === props.dir) apply(reply)
      return reply
    },
    launch: (script) => workspace.runScript(dir, script).key,
    wait: (key) => waitShellResult(() => workspace.panes, key),
  })
  running.value = false
  if (dir === props.dir) await send({ op: 'list' })
}

watch([items, view], () => emit('warning', mergeWarning(items.value, view.value)), { immediate: false })
let stopDone: (() => void) | null = null
onMounted(() => {
  stopDone = onCheckDone(props.dir, () => void send({ op: 'list' }))
  void load()
})
onBeforeUnmount(() => stopDone?.())

defineExpose({ reload: load })
</script>

<template>
  <section class="flex flex-col gap-1.5 border-b border-line pb-2" :aria-label="t('workspace.wt.checklist.title')" data-wt-checklist>
    <div class="flex items-center gap-2">
      <span class="text-2xs font-semibold text-ink-faint">{{ t('workspace.wt.checklist.title') }}</span>
      <span v-if="open" class="ml-auto text-2xs tabular text-ink-muted" data-wt-checklist-open>{{ t('workspace.wt.checklist.open', { n: open }, open) }}</span>
    </div>

    <p v-if="ready && !items.length" class="text-2xs text-ink-faint">{{ t('workspace.wt.checklist.empty') }}</p>
    <ul v-else class="flex flex-col gap-0.5">
      <li v-for="item in items" :key="item.id" class="group/item flex min-w-0 items-start gap-2" data-wt-checklist-item>
        <input
          type="checkbox"
          class="mt-0.5 size-3.5 shrink-0 cursor-pointer accent-[var(--accent)]"
          :checked="item.done"
          :aria-label="item.text"
          @change="toggle(item)"
        />
        <span class="min-w-0 flex-1 break-words text-xs" :class="item.done ? 'text-ink-faint line-through' : 'text-ink'">{{ item.text }}</span>
        <Bot v-if="item.by === 'agent'" :size="12" class="mt-0.5 shrink-0 text-ink-faint" :title="t('workspace.wt.checklist.byAgent')" :aria-label="t('workspace.wt.checklist.byAgent')" role="img" />
        <button
          type="button"
          class="inline-flex size-5 shrink-0 cursor-pointer items-center justify-center rounded-inner text-ink-faint hover:bg-well hover:text-st-error focus-visible:text-st-error"
          :title="t('workspace.wt.checklist.remove')"
          :aria-label="`${t('workspace.wt.checklist.remove')}: ${item.text}`"
          @click="remove(item)"
        >
          <X :size="12" aria-hidden="true" />
        </button>
      </li>
    </ul>

    <form class="flex gap-1.5" @submit.prevent="add">
      <input v-model="text" class="ccm-field h-7 min-w-0 flex-1 text-xs" :placeholder="t('workspace.wt.checklist.placeholder')" :aria-label="t('workspace.wt.checklist.placeholder')" maxlength="200" autocomplete="off" data-wt-checklist-input />
      <button type="submit" class="ccm-btn h-7" :disabled="busy || !text.trim()" data-wt-checklist-add>
        <Plus :size="13" aria-hidden="true" />{{ t('workspace.wt.checklist.add') }}
      </button>
    </form>

    <div class="flex min-w-0 items-center gap-1.5">
      <select class="ccm-field h-7 min-w-0 flex-1 cursor-pointer font-mono text-2xs" :value="check.script" :aria-label="t('workspace.wt.checklist.script')" :disabled="running" data-wt-checklist-script @change="pickScript">
        <option value="">{{ t('workspace.wt.checklist.scriptNone') }}</option>
        <option v-for="o in scripts" :key="o.run" :value="o.run">{{ o.run }}</option>
      </select>
      <button type="button" class="ccm-btn h-7 shrink-0" :disabled="running || !check.script" data-wt-checklist-run @click="run">
        <LoaderCircle v-if="running" :size="13" class="ccm-spin" aria-hidden="true" /><Play v-else :size="13" aria-hidden="true" />{{ t('workspace.wt.checklist.run') }}
      </button>
    </div>
    <p v-if="check.script" class="flex items-center gap-1.5 text-2xs" :class="viewTone[view]" role="status" :data-wt-checklist-state="view">
      <component :is="viewIcon[view]" :size="12" class="shrink-0" :class="view === 'running' ? 'ccm-spin' : ''" aria-hidden="true" />
      {{ t(`workspace.wt.checklist.state.${view}`) }}
    </p>
    <p v-if="check.script && !settings.shellIntegration" class="text-2xs text-ink-faint">{{ t('workspace.wt.checklist.needsIntegration') }}</p>
    <p v-if="error" class="text-2xs text-st-error" role="alert">{{ t(`workspace.wt.checklist.error.${error === 'full' || error === 'text' || error === 'unavailable' || error === 'denied' ? error : 'failed'}`) }}</p>
  </section>
</template>
