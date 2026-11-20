<script setup lang="ts">
import { ArrowUp, ChevronLeft, CircleStop, Ellipsis, FolderTree, LayoutGrid, Link2Off, RefreshCw, Signal, Square, SquareTerminal, TriangleAlert, WifiOff } from 'lucide-vue-next'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RemoteChatFile } from '../../../../packages/shared/src/remote'
import AgentGlyph from '../components/AgentGlyph.vue'
import BottomSheet from '../components/BottomSheet.vue'
import ChatAsk, { type AskPick } from '../components/ChatAsk.vue'
import ChatDiffSheet from '../components/ChatDiffSheet.vue'
import ChatFiles from '../components/ChatFiles.vue'
import ChatMarkdown from '../components/ChatMarkdown.vue'
import ChatTools from '../components/ChatTools.vue'
import ContextRing from '../components/ContextRing.vue'
import QuotaBars from '../components/QuotaBars.vue'
import RailText from '../components/RailText.vue'
import { ChatFeed, type FeedState } from '../lib/chatFeed'
import { liveOf, projectOf } from '../lib/chatInbox'
import { chatSupported, chatView, emptyLog } from '../lib/chatModel'
import { settleOutbox, setState, type OutMsg } from '../lib/chatOutbox'
import { parseSessions } from '../lib/monitorModel'
import { createAnswerFlow } from '../lib/paneAnswer'
import { quotaOf } from '../lib/quotaModel'
import { cachedLog, cacheLog } from '../lib/chatCache'
import { findPane, parseWorkspace, type WsPane } from '../lib/workspaceModel'
import { randomHex } from '../remote/bytes'
import { RemoteCommandError } from '../remote/types'
import { back, navigate } from '../router'
import { useHubStore } from '../stores/hub'
import ViewerPage from './ViewerPage.vue'

const props = defineProps<{ machine: string; pane: string; embedded?: boolean }>()
const { t } = useI18n()
const hub = useHubStore()
const initial = cachedLog(props.machine, props.pane)
const state = shallowRef<FeedState>({ status: initial ? 'ready' : 'loading', log: initial ?? emptyLog(), older: false, error: null })
const now = ref(Date.now())
const WINDOW = 60
const WINDOW_STEP = 40
const start = ref(0)
let ticker: ReturnType<typeof setInterval> | null = null
const outbox = ref<OutMsg[]>([])
const draft = ref('')
const msgs = ref<HTMLElement | null>(null)
const input = ref<HTMLTextAreaElement | null>(null)
const sheet = ref<'more' | 'ctx' | null>(null)
const viewer = ref(false)
const diff = ref<{ root: string; path: string } | null>(null)
const toast = ref<string | null>(null)
const pendingLabel = ref<string | null>(null)
const SLOW_MS = 150
const MAX_TEXT = 2000
let feed: ChatFeed | null = null
let offEvents: (() => void) | null = null
let toastTimer: ReturnType<typeof setTimeout> | null = null
let stick = true

const known = computed(() => hub.machines.some((m) => m.id === props.machine))
const view = computed(() => hub.viewOf(props.machine))
const link = computed(() => hub.linkOf(props.machine))
const online = computed(() => link.value.status === 'ready')
const viewOnly = computed(() => hub.isViewOnly(props.machine))
const rtt = computed(() => hub.rtt[props.machine])
const tabs = computed(() => parseWorkspace(view.value.workspace, parseSessions(view.value.snapshot)))
const found = computed(() => findPane(tabs.value, props.pane))
const pane = computed<WsPane | null>(() => found.value?.pane ?? null)
const project = computed(() => (found.value ? projectOf(found.value.tab, found.value.pane) : null))
const live = computed(() => liveOf(view.value.snapshot, pane.value?.terminalId ?? null))
const agentName = computed(() => t(`provider.${pane.value?.provider ?? 'claude'}`))
const all = computed(() => chatView(state.value.log))
const items = computed(() => all.value.slice(Math.min(start.value, Math.max(0, all.value.length - WINDOW))))
const hiddenCount = computed(() => all.value.length - items.value.length)
const shownOut = computed(() => settleOutbox(outbox.value, all.value, now.value))
const working = computed(() => pane.value?.rail === 'working')
const asked = computed(() => pane.value?.rail === 'asked' || !!pane.value?.menu)
const stopping = computed(() => draft.value.trim() === '' && working.value)
const canWrite = computed(() => !viewOnly.value && pane.value !== null)
const empty = computed(() => state.value.status === 'ready' && items.value.length === 0 && shownOut.value.length === 0)
const quota = computed(() => quotaOf(view.value.snapshot, now.value).filter((q) => q.agent === pane.value?.provider))
const quotaHot = computed(() => quota.value.some((q) => q.pct >= 80 && !q.stale))
const chips = ['summary', 'test', 'git', 'typecheck'] as const
const liveText = computed(() => {
  const l = live.value
  if (l.tool) return l.toolSummary ? `${l.tool} ${l.toolSummary}` : l.tool
  return t('chat.live')
})
const liveTime = computed(() => {
  const start = live.value.toolAt ?? pane.value?.since ?? null
  if (!start) return ''
  const sec = Math.max(0, Math.floor((now.value - start) / 1000))
  return `${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, '0')}`
})

function say(text: string): void {
  toast.value = text
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => {
    toast.value = null
  }, 3500)
}

function errText(error: unknown): string {
  return t(`err.${error instanceof RemoteCommandError ? error.code : 'internal'}`)
}

function toTerminal(replace = false): void {
  navigate({ name: 'workspace', machine: props.machine, pane: props.pane }, { replace })
}

function atBottom(): boolean {
  const el = msgs.value
  return !el || el.scrollHeight - el.scrollTop - el.clientHeight < 80
}

function toBottom(): void {
  void nextTick(() => {
    const el = msgs.value
    if (el) el.scrollTop = el.scrollHeight
  })
}

async function older(): Promise<void> {
  const el = msgs.value
  if (!el) return
  const before = el.scrollHeight
  const top = el.scrollTop
  if (hiddenCount.value > 0) {
    start.value = Math.max(0, all.value.length - items.value.length - WINDOW_STEP)
  } else {
    if (!feed || state.value.older || state.value.log.older === null) return
    await feed.loadOlder()
    start.value = 0
  }
  await nextTick()
  if (msgs.value) msgs.value.scrollTop = msgs.value.scrollHeight - before + top
}

function onScroll(): void {
  stick = atBottom()
  const el = msgs.value
  if (el && el.scrollTop < 120) void older()
}

function begin(): void {
  if (feed || !known.value) return
  feed = new ChatFeed(
    props.pane,
    {
      command: (type, fields) => hub.command(props.machine, type, fields),
      onChange: (next) => {
        if (next.log !== state.value.log) cacheLog(props.machine, props.pane, next.log)
        if (next.log.gen !== state.value.log.gen) start.value = 0
        state.value = next
      },
    },
    initial,
  )
  offEvents = hub.events(props.machine, (event) => feed?.handle(event))
  if (document.visibilityState === 'hidden') feed.pause()
  else if (online.value) void feed.watch()
}

function syncTicker(): void {
  const want = working.value && document.visibilityState === 'visible'
  if (want && !ticker) {
    now.value = Date.now()
    ticker = setInterval(() => {
      now.value = Date.now()
    }, 1000)
  } else if (!want && ticker) {
    clearInterval(ticker)
    ticker = null
  }
}

function onVisibility(): void {
  if (document.visibilityState === 'hidden') feed?.pause()
  else {
    now.value = Date.now()
    if (online.value) void feed?.resume()
  }
  syncTicker()
}

async function send(text: string): Promise<void> {
  const body = text.trim().slice(0, MAX_TEXT)
  if (!body || !canWrite.value) return
  const id = randomHex(6)
  outbox.value = [...outbox.value, { id, text: body, state: 'sending', at: Date.now() }]
  stick = true
  toBottom()
  await deliver(id, body)
}

async function deliver(id: string, body: string): Promise<void> {
  try {
    await hub.command(props.machine, 'ws.send', { paneKey: props.pane, text: body })
    outbox.value = setState(outbox.value, id, 'sent', Date.now())
  } catch (error) {
    outbox.value = setState(outbox.value, id, 'unsent')
    if (!(error instanceof RemoteCommandError && error.code === 'offline')) say(errText(error))
  }
}

function resend(msg: OutMsg): void {
  outbox.value = setState(outbox.value, msg.id, 'sending')
  void deliver(msg.id, msg.text)
}

function submit(): void {
  if (stopping.value) {
    void stop()
    return
  }
  const text = draft.value
  draft.value = ''
  resize()
  void send(text)
}

async function stop(): Promise<void> {
  const p = pane.value
  if (!p) return
  try {
    await hub.command(props.machine, 'ws.keys', { paneKey: p.key, keys: 'stop', since: p.since ?? 0 })
    say(t('chat.stopped'))
  } catch (error) {
    say(errText(error))
  }
}

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Enter' && (event.ctrlKey || event.metaKey) && !event.isComposing) {
    event.preventDefault()
    submit()
  }
}

function resize(): void {
  const el = input.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 140)}px`
}

function readPane(key: string): WsPane | null {
  return findPane(tabs.value, key)?.pane ?? null
}

const answers = createAnswerFlow({
  now: () => Date.now(),
  dataAt: () => (view.value.stale ? 0 : (view.value.updatedAt ?? 0)),
  refresh: async () => {
    await Promise.all([hub.command(props.machine, 'ws.get'), hub.refresh(props.machine)])
  },
  read: readPane,
  send: async (target, keys) => {
    if (keys.startsWith('menu:')) {
      const sig = target.menu?.sig
      if (!sig) throw new RemoteCommandError('busy')
      await hub.command(props.machine, 'ws.menu', { paneKey: target.key, sig, index: Number(keys.slice(5)) })
    } else {
      await hub.command(props.machine, 'ws.keys', { paneKey: target.key, keys, since: target.since ?? 0 })
    }
  },
})

async function pick(choice: AskPick): Promise<void> {
  const p = pane.value
  if (!p || !online.value || viewOnly.value) return
  pendingLabel.value = choice.label
  const result = await answers.press(p, choice.keys)
  pendingLabel.value = null
  if (result.outcome === 'changed') say(t('chat.changed'))
  else if (result.outcome === 'failed') say(errText(result.error))
}

function openFile(file: RemoteChatFile, root: string): void {
  if (file.out || !root) return
  diff.value = { root, path: file.path }
}

function more(target: 'terminal' | 'files' | 'workspace'): void {
  sheet.value = null
  if (target === 'terminal') toTerminal()
  else if (target === 'files') viewer.value = true
  else navigate({ name: 'workspace', machine: props.machine })
}

watch(
  () => [view.value.hello, found.value !== null] as const,
  ([hello, has]) => {
    if (!hello || !has) return
    if (!chatSupported(hello) || pane.value?.provider === 'shell') toTerminal(true)
    else begin()
  },
  { immediate: true },
)

watch(
  () => state.value.status,
  (status) => {
    if (status === 'unsupported') toTerminal(true)
  },
)

watch(online, (up, was) => {
  if (up && !was && feed && !feed.isPaused) void feed.watch()
})

watch(
  () => [items.value.length, shownOut.value.length, asked.value, working.value] as const,
  () => {
    if (stick) toBottom()
  },
  { flush: 'post' },
)

watch(
  () => state.value.log.gen,
  () => {
    stick = true
    toBottom()
  },
)

watch(working, syncTicker, { immediate: true })

onMounted(() => {
  document.addEventListener('visibilitychange', onVisibility)
  toBottom()
})

onBeforeUnmount(() => {
  document.removeEventListener('visibilitychange', onVisibility)
  if (ticker) clearInterval(ticker)
  answers.dispose()
  offEvents?.()
  feed?.stop()
  if (toastTimer) clearTimeout(toastTimer)
})
</script>

<template>
  <section class="page" data-testid="chat-view" :data-machine="machine" :data-pane="pane?.key ?? props.pane" :data-status="state.status">
    <header class="bar">
      <div class="side">
        <button v-if="!embedded" type="button" class="icon-btn" :aria-label="t('common.back')" data-testid="back" @click="back()">
          <ChevronLeft :size="24" :stroke-width="2" aria-hidden="true" />
        </button>
        <button type="button" class="icon-btn" :aria-label="t('chat.viewTerminal')" :title="t('chat.viewTerminal')" data-testid="chat-terminal" @click="toTerminal()">
          <SquareTerminal :size="22" :stroke-width="1.9" aria-hidden="true" />
        </button>
      </div>
      <div class="who">
        <span class="who-name"><AgentGlyph v-if="pane" :provider="pane.provider" :size="14" /><span data-testid="chat-title">{{ pane?.title ?? t('common.loading') }}</span></span>
        <span class="who-sub">
          <RailText v-if="pane" :rail="pane.rail" :size="12" />
          <span v-if="project" class="proj">{{ project }}</span>
          <span v-if="!online" class="lat" data-testid="chat-latency" data-kind="offline"><WifiOff :size="12" :stroke-width="2.2" aria-hidden="true" />{{ t('chat.offline') }}</span>
          <span v-else-if="rtt !== undefined && rtt > SLOW_MS" class="lat" data-testid="chat-latency" data-kind="slow"><Signal :size="12" :stroke-width="2.2" aria-hidden="true" />{{ t('chat.slow', { ms: rtt }) }}</span>
        </span>
      </div>
      <div class="side end">
        <ContextRing v-if="live.contextPct !== null" :pct="live.contextPct" :flag="quotaHot" :label="t('chat.ctxLabel', { pct: live.contextPct })" @open="sheet = 'ctx'" />
        <button type="button" class="icon-btn" :aria-label="t('chat.more')" data-testid="chat-view-more" @click="sheet = 'more'">
          <Ellipsis :size="22" :stroke-width="1.9" aria-hidden="true" />
        </button>
      </div>
    </header>
    <p v-if="!online" class="reconnect" role="status" data-testid="chat-offline"><RefreshCw :size="13" :stroke-width="2.2" aria-hidden="true" />{{ t('chat.reconnecting') }}</p>

    <div ref="msgs" class="msgs" aria-live="polite" data-testid="chat-msgs" @scroll.passive="onScroll">
      <p v-if="state.older" class="older" role="status">{{ t('chat.older') }}</p>
      <p v-if="state.status === 'loading' && items.length === 0" class="center" role="status">{{ t('chat.loading') }}</p>
      <div v-else-if="state.status === 'error' && items.length === 0" class="center" role="alert" data-testid="chat-error">
        <TriangleAlert :size="20" :stroke-width="2" aria-hidden="true" />
        <span>{{ t('chat.failed') }}</span>
        <button type="button" class="mini" @click="feed?.watch()">{{ t('common.retry') }}</button>
      </div>
      <div v-else-if="empty" class="empty-chat" data-testid="chat-empty">
        <h2 class="hello">{{ t('chat.hello') }}</h2>
        <p><AgentGlyph v-if="pane" :provider="pane.provider" :size="16" />{{ project ? t('chat.helloSub', { agent: agentName, project }) : agentName }}</p>
      </div>
      <template v-for="item in items" :key="item.id">
        <div v-if="item.k === 'me'" class="me" data-testid="chat-me" :data-id="item.id"><div class="bub">{{ item.text }}</div></div>
        <div v-else-if="item.k === 'ai'" class="ai" data-testid="chat-ai" :data-id="item.id">
          <ChatMarkdown :text="item.text" />
          <button v-if="item.cut" type="button" class="mini cut" @click="toTerminal()">{{ t('chat.cut') }}</button>
        </div>
        <ChatTools v-else-if="item.k === 'tools'" :item="item" />
        <ChatFiles v-else-if="item.k === 'files'" :item="item" @open="openFile" />
        <p v-else-if="item.k === 'sys'" class="sys" :class="item.kind" data-testid="chat-sys">
          <CircleStop v-if="item.kind === 'stop'" :size="14" :stroke-width="2.2" aria-hidden="true" />
          <TriangleAlert v-else-if="item.kind !== 'compact'" :size="14" :stroke-width="2.2" aria-hidden="true" />
          {{ item.text || t(`chat.sys.${item.kind}`) }}
        </p>
      </template>
      <div v-for="msg in shownOut" :key="msg.id" class="me" :class="{ unsent: msg.state === 'unsent', sending: msg.state === 'sending' }" data-testid="chat-out" :data-state="msg.state">
        <div class="bub">{{ msg.text }}</div>
        <div v-if="msg.state === 'unsent'" class="unsent-row" data-testid="chat-unsent">
          <Link2Off :size="15" :stroke-width="2.2" aria-hidden="true" />{{ t('chat.unsent') }}
          <button type="button" class="mini" :disabled="!online" data-testid="chat-resend" @click="resend(msg)">{{ t('chat.resend') }}</button>
        </div>
      </div>
      <div v-if="working && !asked" class="live" role="status" data-testid="chat-live">
        <span class="pulse" aria-hidden="true" />
        <span class="what">{{ liveText }}</span>
        <time>{{ liveTime }}</time>
      </div>
      <ChatAsk v-if="asked && pane" :pane="pane" :cmd="live.toolSummary" :enabled="online && !viewOnly" :pending="answers.pending.value ? pendingLabel : null" @pick="pick" @undo="answers.undo()" @terminal="toTerminal()" />
    </div>

    <div class="composer">
      <div v-if="empty && canWrite" class="chips" role="group" :aria-label="t('chat.chipsLabel')">
        <button v-for="c in chips" :key="c" type="button" class="chip" :disabled="!online" :data-testid="`chat-chip-${c}`" @click="send(t(`chat.chips.${c}`))">{{ t(`chat.chips.${c}`) }}</button>
      </div>
      <p v-if="viewOnly" class="ro" role="status">{{ t('term.viewOnly') }}</p>
      <form v-else class="compose-row" @submit.prevent="submit">
        <textarea ref="input" v-model="draft" rows="1" :placeholder="t('chat.message', { agent: agentName })" :aria-label="t('chat.messageLabel')" :maxlength="MAX_TEXT" data-testid="chat-input" @input="resize" @keydown="onKey" />
        <button type="submit" class="send" :class="{ stopping }" :disabled="!stopping && (draft.trim() === '' || !canWrite)" :aria-label="stopping ? t('chat.stop') : t('chat.send')" data-testid="chat-send" :data-mode="stopping ? 'stop' : 'send'">
          <Square v-if="stopping" :size="16" :stroke-width="0" fill="currentColor" aria-hidden="true" />
          <ArrowUp v-else :size="20" :stroke-width="2.4" aria-hidden="true" />
        </button>
      </form>
    </div>

    <p v-if="toast" class="toast" role="status" data-testid="chat-toast">{{ toast }}</p>

    <BottomSheet v-if="sheet === 'more'" :title="pane?.title ?? ''" testid="chat-view-more-sheet" @close="sheet = null">
      <div class="s-list">
        <button type="button" class="s-row" data-testid="chat-more-terminal" @click="more('terminal')"><SquareTerminal :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.terminal') }}</span></button>
        <button type="button" class="s-row" :disabled="!pane?.cwd" data-testid="chat-more-files" @click="more('files')"><FolderTree :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.files') }}</span></button>
        <button type="button" class="s-row" data-testid="chat-more-workspace" @click="more('workspace')"><LayoutGrid :size="20" :stroke-width="2" aria-hidden="true" /><span>{{ t('chat.menu.workspace') }}</span></button>
      </div>
    </BottomSheet>
    <BottomSheet v-if="sheet === 'ctx'" :title="t('chat.ctxTitle')" testid="chat-ctx-sheet" @close="sheet = null">
      <div v-if="live.contextPct !== null" class="meter" :class="{ warm: live.contextPct >= 80 }">
        <div class="meter-top"><span>{{ t('chat.context') }}</span><b>{{ live.contextPct }}%</b></div>
        <div class="meter-bar" aria-hidden="true"><i :style="{ width: `${live.contextPct}%` }" /></div>
      </div>
      <div v-if="live.model" class="kv"><span>{{ t('chat.model') }}</span><b>{{ live.model }}</b></div>
      <div class="kv"><span>{{ t('chat.network') }}</span><b>{{ online ? `${rtt ?? '?'} ms, ${t(link.route === 'lan' ? 'link.lan' : 'link.relay')}` : t('chat.offline') }}</b></div>
      <QuotaBars v-if="quota.length" :rows="quota" :now="now" />
    </BottomSheet>
    <ChatDiffSheet v-if="diff" :machine="machine" :root="diff.root" :path="diff.path" @close="diff = null" />
    <ViewerPage v-if="viewer && pane?.cwd" :machine="machine" :root="pane.cwd" :title="pane.title" @close="viewer = false" />
  </section>
</template>

<style scoped>
.page {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--ccm-canvas);
}

.bar {
  display: grid;
  grid-template-columns: 88px minmax(0, 1fr) 88px;
  align-items: center;
  flex: none;
  min-height: 56px;
  padding: 4px 6px;
}

.side {
  display: flex;
  align-items: center;
  min-width: 0;
}

.side.end {
  justify-content: flex-end;
}

.icon-btn {
  display: grid;
  place-items: center;
  flex: none;
  width: 44px;
  height: 44px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--ccm-ink);
}

.who {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-width: 0;
  min-height: 44px;
  padding: 0 4px;
}

.who-name {
  display: flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
  min-width: 0;
  font-size: 0.97rem;
  font-weight: 700;
  line-height: 1.25;
}

.who-name > span:last-child {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.who-sub {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  max-width: 100%;
  min-width: 0;
  font-size: 0.78rem;
  color: var(--ccm-ink-muted);
  white-space: nowrap;
}

.proj {
  flex: 0 1 auto;
  min-width: 0;
  max-width: 96px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lat {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  flex: none;
  color: var(--ccm-warning);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.reconnect {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  flex: none;
  margin: 0;
  min-height: 28px;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--ccm-warning);
}

.msgs {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 12px 20px 28px;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.msgs > * {
  flex: none;
}

.older,
.center {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: 0;
  font-size: 0.875rem;
  color: var(--ccm-ink-muted);
}

.center {
  flex-direction: column;
  margin: auto;
}

.me {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.bub {
  max-width: 82%;
  padding: 10px 16px;
  border-radius: 22px;
  background: var(--ccm-fill-strong);
  font-size: 1rem;
  line-height: 1.5;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.me.sending .bub {
  opacity: 0.7;
}

.me.unsent .bub {
  background: transparent;
  box-shadow: inset 0 0 0 1px var(--ccm-line-strong);
  color: var(--ccm-ink-muted);
}

.unsent-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--ccm-warning);
}

.mini {
  min-height: 44px;
  padding: 0 12px;
  border: 0;
  border-radius: 22px;
  background: transparent;
  color: var(--ccm-ink);
  font-size: 0.84rem;
  font-weight: 700;
}

.mini:disabled {
  color: var(--ccm-ink-faint);
}

.ai {
  font-size: 1.03rem;
  line-height: 1.7;
  color: var(--ccm-ink);
  overflow-wrap: anywhere;
}

.cut {
  margin-top: 6px;
  padding: 0;
  color: var(--ccm-accent-ink);
}

.sys {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.sys.stop svg,
.sys.error svg,
.sys.limit svg {
  color: var(--ccm-error);
}

.live {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 28px;
  font-size: 0.9rem;
  color: var(--ccm-ink-muted);
}

.pulse {
  flex: none;
  width: 8px;
  height: 8px;
  margin: 0 6px;
  border-radius: 50%;
  background: var(--ccm-working);
  animation: beat 1.6s ease-in-out infinite;
}

@keyframes beat {
  50% {
    opacity: 0.3;
  }
}

.what {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.live time {
  flex: none;
  margin-left: auto;
  font-size: 0.8125rem;
  font-variant-numeric: tabular-nums;
}

.empty-chat {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  margin: auto;
  padding: 24px 8px;
  text-align: center;
  color: var(--ccm-ink-muted);
}

.hello {
  margin: 0;
  font-size: 2.1rem;
  line-height: 1.2;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--ccm-accent-ink);
}

.empty-chat p {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: 0.94rem;
}

.composer {
  flex: none;
  padding: 4px 12px calc(12px + var(--ccm-safe-bottom, 0px));
  background: var(--ccm-canvas);
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0 2px 10px;
}

.chip {
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: 0 16px;
  border: 0;
  border-radius: 22px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
  color: var(--ccm-ink);
  font-size: 0.875rem;
  font-weight: 600;
}

.ro {
  margin: 0;
  padding: 12px 6px;
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--ccm-waiting);
}

.compose-row {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  padding: 6px 6px 6px 16px;
  border-radius: 30px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line-strong), var(--ccm-shadow-control);
}

.compose-row:focus-within {
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ccm-accent) 55%, transparent), var(--ccm-shadow-control);
}

.compose-row textarea {
  flex: 1;
  min-width: 0;
  min-height: 44px;
  max-height: 140px;
  padding: 10px 4px;
  border: 0;
  background: none;
  resize: none;
  outline: none;
  color: var(--ccm-ink);
  font: inherit;
  font-size: 16px;
  line-height: 1.5;
}

.compose-row textarea::placeholder {
  color: var(--ccm-ink-faint);
}

.send {
  display: grid;
  place-items: center;
  flex: none;
  width: 44px;
  height: 44px;
  border: 0;
  border-radius: 50%;
  background: var(--ccm-accent);
  color: var(--ccm-on-accent);
}

.send:disabled {
  background: var(--ccm-fill);
  color: var(--ccm-ink-faint);
}

.send.stopping {
  background: var(--ccm-ink);
  color: var(--ccm-canvas);
}

.toast {
  position: absolute;
  left: 50%;
  bottom: 96px;
  z-index: 5;
  max-width: calc(100% - 32px);
  margin: 0;
  padding: 10px 16px;
  border-radius: 20px;
  background: var(--ccm-ink);
  color: var(--ccm-canvas);
  font-size: 0.875rem;
  font-weight: 600;
  transform: translateX(-50%);
}

.s-list {
  display: flex;
  flex-direction: column;
}

.s-row {
  display: flex;
  align-items: center;
  gap: 14px;
  min-height: 52px;
  padding: 0 12px;
  border: 0;
  border-radius: 16px;
  background: transparent;
  color: var(--ccm-ink);
  font-size: 1rem;
  text-align: left;
}

.s-row svg {
  color: var(--ccm-ink-muted);
}

.s-row:disabled {
  color: var(--ccm-ink-faint);
}

.meter {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 4px;
}

.meter-top {
  display: flex;
  justify-content: space-between;
  gap: 8px;
}

.meter-bar {
  height: 8px;
  overflow: hidden;
  border-radius: 4px;
  background: var(--ccm-fill);
}

.meter-bar i {
  display: block;
  height: 100%;
  border-radius: 4px;
  background: var(--ccm-ink-muted);
}

.meter.warm b {
  color: var(--ccm-warning);
}

.meter.warm .meter-bar i {
  background: var(--ccm-warning);
}

.kv {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  min-height: 44px;
  padding: 0 4px;
}

.kv span {
  color: var(--ccm-ink-muted);
}

.kv b {
  font-weight: 600;
  text-align: right;
  overflow-wrap: anywhere;
}

@media (min-width: 768px) {
  .msgs > *,
  .composer > * {
    width: 100%;
    max-width: 720px;
    margin-left: auto;
    margin-right: auto;
  }

  .msgs > .me {
    align-items: flex-end;
  }
}

@media (prefers-reduced-motion: reduce) {
  .pulse {
    animation: none;
  }
}
</style>
