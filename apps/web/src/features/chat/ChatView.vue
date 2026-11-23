<script setup lang="ts">
import { ArrowDown, ChevronRight, LoaderCircle, MessageSquareText, Pencil, TriangleAlert, Wrench } from 'lucide-vue-next'
import { computed, nextTick, onBeforeUnmount, reactive, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RemoteChatTools } from '@ccm/shared'
import ChatExportMenu from './ChatExportMenu.vue'
import ChatMarkdown from './ChatMarkdown.vue'
import { bridgeCall, ChatFeed, hasChatView, type ChatCall, type ChatTarget, type FeedState } from './feed'
import { anchorTop, CHAT_NEAR_TOP, CHAT_STEP, CHAT_WINDOW, chatView, fileTotals, nearBottom, splitPath, joinRoot, toolParts, topAction, windowOf, emptyLog } from './model'

const props = withDefaults(defineProps<{ target: ChatTarget | null; active: boolean; agentLabel: string; call?: ChatCall }>(), { call: undefined })
const emit = defineEmits<{ openFile: [path: string] }>()
const { t } = useI18n()

const state = shallowRef<FeedState>({ status: 'loading', log: emptyLog(), loadingOlder: false })
const scroller = ref<HTMLElement | null>(null)
const shown = ref(CHAT_WINDOW)
const stuck = ref(true)
const open = reactive(new Set<string>())
let feed: ChatFeed | null = null
let anchor: { top: number; height: number } | null = null

const supported = computed(() => !!props.call || hasChatView)
const ready = computed(() => !!props.target?.session && supported.value)
const items = computed(() => chatView(state.value.log))
const rendered = computed(() => windowOf(items.value, shown.value))
const atStart = computed(() => state.value.status === 'ready' && state.value.log.older === null && rendered.value.length === items.value.length && items.value.length > 0)

function onFeed(next: FeedState): void {
  const prev = state.value
  const el = scroller.value
  if (prev.loadingOlder && !next.loadingOlder && next.log !== prev.log && prev.log.gen === next.log.gen) {
    shown.value += Math.max(0, chatView(next.log).length - items.value.length)
    if (el) anchor = { top: el.scrollTop, height: el.scrollHeight }
  }
  state.value = next
  void settle()
}

async function settle(): Promise<void> {
  await nextTick()
  const el = scroller.value
  if (!el) return
  if (stuck.value) {
    if (shown.value > CHAT_WINDOW) shown.value = CHAT_WINDOW
    await nextTick()
    el.scrollTop = el.scrollHeight
    anchor = null
    return
  }
  if (anchor && el.scrollHeight !== anchor.height) {
    const a = anchor
    anchor = null
    if (el.scrollTop <= a.top + 1) el.scrollTop = anchorTop(a.top, a.height, el.scrollHeight)
  }
}

function sync(): void {
  const want = props.active && ready.value && props.target
  if (!want) {
    feed?.stop()
    return
  }
  if (!feed) {
    feed = new ChatFeed(props.target!, props.call ?? bridgeCall, onFeed)
    state.value = feed.current
  } else {
    feed.retarget(props.target!)
  }
  stuck.value = true
  feed.start()
}

watch(
  () => [props.active, ready.value, props.target?.key, props.target?.session, props.target?.agent, props.target?.cwd] as const,
  () => sync(),
  { immediate: true },
)

onBeforeUnmount(() => {
  feed?.stop()
  feed = null
})

async function growTop(): Promise<void> {
  const el = scroller.value
  if (!el) return
  const action = topAction(items.value.length, shown.value, state.value.log.older, state.value.loadingOlder)
  if (action === 'none') return
  if (action === 'fetch') {
    await feed?.older()
    return
  }
  anchor = { top: el.scrollTop, height: el.scrollHeight }
  shown.value = Math.min(items.value.length, shown.value + CHAT_STEP)
  await settle()
}

function onScroll(): void {
  const el = scroller.value
  if (!el) return
  stuck.value = nearBottom(el.scrollTop, el.scrollHeight, el.clientHeight)
  if (el.scrollTop < CHAT_NEAR_TOP) void growTop()
}

function toLatest(): void {
  stuck.value = true
  void settle()
}

function retry(): void {
  void feed?.reload()
}

function toolLine(item: RemoteChatTools): string {
  const parts = toolParts(item.sum).map((p) => t(`chatView.tool.${p.cat}`, { n: p.n }, p.n))
  if (parts.length === 0) {
    const n = item.items.length + (item.more ?? 0)
    parts.push(t('chatView.tool.other', { n }, n))
  }
  return parts.join(', ')
}

function toggle(id: string): void {
  if (open.has(id)) open.delete(id)
  else open.add(id)
}

defineExpose({ scroller })
</script>

<template>
  <div class="flex size-full min-h-0 flex-col" data-chat-view :data-chat-status="ready ? state.status : 'idle'">
    <div v-if="!ready" class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center" data-chat-nosession>
      <MessageSquareText :size="20" class="text-ink-faint" aria-hidden="true" />
      <p class="max-w-72 text-xs text-ink-muted">{{ t('chatView.noSession') }}</p>
    </div>
    <template v-else>
      <div class="relative z-20 flex h-8 shrink-0 items-center justify-end border-b border-line px-2" data-chat-header>
        <ChatExportMenu v-if="props.target" :target="props.target" :call="props.call" />
      </div>
      <div
        ref="scroller"
        class="ccm-scroll min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-3 py-2 text-[13px] leading-relaxed text-ink"
        role="log"
        :aria-label="t('chatView.region', { agent: agentLabel })"
        aria-live="off"
        tabindex="0"
        data-chat-scroll
        @scroll.passive="onScroll"
      >
        <p v-if="state.status === 'loading' && items.length === 0" class="flex items-center justify-center gap-2 py-6 text-xs text-ink-muted" role="status" data-chat-loading>
          <LoaderCircle :size="14" class="ccm-spin" aria-hidden="true" />{{ t('chatView.loading') }}
        </p>
        <div v-else-if="state.status === 'error' && items.length === 0" class="flex flex-col items-center gap-2 py-6 text-xs text-ink-muted" role="alert" data-chat-error>
          <span>{{ t('chatView.failed') }}</span>
          <button type="button" class="ccm-btn h-6 px-2 text-2xs" @click="retry">{{ t('chatView.retry') }}</button>
        </div>
        <p v-else-if="state.status === 'ready' && items.length === 0" class="py-6 text-center text-xs text-ink-muted" data-chat-empty>{{ t('chatView.empty') }}</p>
        <template v-else>
          <p v-if="state.loadingOlder" class="flex items-center justify-center gap-2 py-2 text-2xs text-ink-muted" role="status" data-chat-older>
            <LoaderCircle :size="12" class="ccm-spin" aria-hidden="true" />{{ t('chatView.older') }}
          </p>
          <p v-else-if="atStart" class="py-2 text-center text-2xs text-ink-faint" data-chat-top>{{ t('chatView.top') }}</p>
          <p v-else class="py-2 text-center text-2xs text-ink-faint">{{ t('chatView.olderMore') }}</p>
          <ul class="flex flex-col gap-2.5">
            <li v-for="item in rendered" :key="item.id" class="ccm-chat-row min-w-0" :data-chat-item="item.k" :data-id="item.id">
              <div v-if="item.k === 'me'" class="flex justify-end">
                <div class="max-w-[85%] min-w-0 rounded-card bg-[var(--ccm-control)] px-3 py-1.5 whitespace-pre-wrap text-ink [overflow-wrap:anywhere]" data-chat-me>
                  <span class="sr-only">{{ t('chatView.me') }}: </span>{{ item.text }}
                  <span v-if="item.cut" class="mt-1 block text-2xs text-ink-muted">{{ t('chatView.cut') }}</span>
                </div>
              </div>
              <div v-else-if="item.k === 'ai'" class="min-w-0" data-chat-ai>
                <ChatMarkdown :text="item.text" />
                <span v-if="item.cut" class="mt-1 block text-2xs text-ink-muted">{{ t('chatView.cut') }}</span>
              </div>
              <div v-else-if="item.k === 'tools'" class="min-w-0" data-chat-tools>
                <button
                  type="button"
                  class="inline-flex max-w-full min-w-0 cursor-pointer items-center gap-1.5 rounded-inner px-1 py-0.5 text-xs text-ink-muted transition-colors duration-(--ccm-dur-fast) hover:bg-fill hover:text-ink"
                  :aria-expanded="open.has(item.id)"
                  data-chat-tools-toggle
                  @click="toggle(item.id)"
                >
                  <Wrench :size="12" class="shrink-0" aria-hidden="true" />
                  <span class="min-w-0 truncate">{{ toolLine(item) }}</span>
                  <TriangleAlert v-if="item.items.some((x) => x.err)" :size="12" class="shrink-0 text-st-error" :aria-label="t('chatView.toolErr')" />
                  <ChevronRight :size="12" class="shrink-0 transition-transform duration-(--ccm-dur-fast)" :class="open.has(item.id) ? 'rotate-90' : ''" aria-hidden="true" />
                </button>
                <ul v-if="open.has(item.id)" class="mt-1 ml-2 flex flex-col gap-0.5 border-l border-line pl-3 text-xs text-ink-muted" data-chat-tools-list>
                  <li v-for="(x, i) in item.items" :key="i" class="flex min-w-0 items-baseline gap-1.5">
                    <span class="shrink-0 font-medium text-ink">{{ x.tool }}</span>
                    <code v-if="x.target" class="min-w-0 truncate font-mono text-2xs" :title="x.target">{{ x.target }}</code>
                    <span v-if="x.err" class="shrink-0 text-st-error">{{ t('chatView.toolErr') }}</span>
                  </li>
                  <li v-if="item.more" class="text-ink-faint">{{ t('chatView.toolMore', { n: item.more }) }}</li>
                </ul>
              </div>
              <div v-else-if="item.k === 'files'" class="min-w-0 overflow-hidden rounded-card border border-line bg-surface" data-chat-files>
                <div class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-semibold text-ink">
                  <Pencil :size="12" class="shrink-0 text-ink-muted" aria-hidden="true" />
                  <span class="min-w-0 truncate">{{ t('chatView.files', { n: item.files.length + (item.more ?? 0) }, item.files.length + (item.more ?? 0)) }}</span>
                  <span class="ml-auto shrink-0 font-medium tabular-nums"
                    ><span class="text-diff-add">+{{ fileTotals(item.files).add }}</span
                    ><span v-if="fileTotals(item.files).del" class="text-diff-del"> -{{ fileTotals(item.files).del }}</span></span
                  >
                </div>
                <button
                  v-for="f in item.files"
                  :key="f.path"
                  type="button"
                  class="flex w-full min-w-0 cursor-pointer items-center gap-2 border-t border-line px-2.5 py-1 text-left font-mono text-2xs transition-colors duration-(--ccm-dur-fast) hover:bg-fill disabled:cursor-default disabled:hover:bg-transparent"
                  :disabled="f.out"
                  :title="f.out ? f.path : t('chatView.openFile', { path: f.path })"
                  data-chat-file
                  @click="emit('openFile', joinRoot(item.root, f.path))"
                >
                  <span class="min-w-0 flex-1 truncate"
                    ><span class="text-ink-faint">{{ splitPath(f.path).dir }}</span><span class="text-ink">{{ splitPath(f.path).name }}</span></span
                  >
                  <span v-if="f.out" class="shrink-0 font-sans text-ink-faint">{{ t('chatView.outside') }}</span>
                  <template v-else>
                    <span v-if="f.add" class="shrink-0 text-diff-add tabular-nums">+{{ f.add }}</span>
                    <span v-if="f.del" class="shrink-0 text-diff-del tabular-nums">-{{ f.del }}</span>
                  </template>
                </button>
                <p v-if="item.more" class="border-t border-line px-2.5 py-1 text-2xs text-ink-faint">{{ t('chatView.filesMore', { n: item.more }) }}</p>
              </div>
              <p v-else-if="item.k === 'sys'" class="text-center text-2xs text-ink-faint" data-chat-sys>
                {{ t(`chatView.sys.${item.kind}`) }}<template v-if="item.text">, {{ item.text }}</template>
              </p>
            </li>
          </ul>
        </template>
      </div>
      <div class="relative flex h-6 shrink-0 items-center justify-center border-t border-line px-2 text-2xs text-ink-faint" data-chat-readonly>
        {{ t('chatView.readOnly') }}
        <Transition name="ccm-fade">
          <button
            v-if="!stuck && items.length > 0"
            type="button"
            class="ccm-btn absolute right-2 bottom-8 h-6 gap-1 px-2 text-2xs shadow-card"
            :aria-label="t('chatView.latest')"
            data-chat-latest
            @click="toLatest"
          >
            <ArrowDown :size="12" aria-hidden="true" />{{ t('chatView.latest') }}
          </button>
        </Transition>
      </div>
    </template>
  </div>
</template>

<style scoped>
.ccm-chat-row {
  content-visibility: auto;
  contain-intrinsic-size: auto 48px;
}
</style>
