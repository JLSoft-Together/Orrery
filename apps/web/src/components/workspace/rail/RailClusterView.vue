<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { BellRing, ChevronRight, Ellipsis, Folder, GitBranch, Hand, LoaderCircle, Mail, Plus, X } from 'lucide-vue-next'
import { usePaneInfo } from '../../../composables/usePaneInfo'
import { useRailTabStop } from '../../../composables/useRail'
import { onDustLeave } from '../../../lib/dust'
import { duration, now as clock } from '../../../lib/format'
import { motionMs, pinLeaving } from '../../../lib/motion'
import {
  checkoutId,
  headId,
  isSolo,
  RAIL_BADGE,
  railItems,
  showsCheckouts,
  stateWord,
  type RailCheckout,
  type RailCluster,
  type RailGroup,
  type RailItem,
  type RailRow as Row,
} from '../../../lib/rail/railModel'
import { agoOf, lastActive, monogram, projectTone, runCounts } from '../../../lib/rail/projectRow'
import { folderName } from '../../../lib/sidebarRows'
import { isMac } from '../../../lib/platform'
import { keyCombo } from '../../../lib/shortcuts'
import { useWorkspaceStore } from '../../../stores/workspace'
import StatusIcon from '../../StatusIcon.vue'
import RunBadge from '../RunBadge.vue'
import RailGroupHead from './RailGroupHead.vue'
import RailRow from './RailRow.vue'

const props = defineProps<{
  cluster: RailCluster
  index: number
  collapsed: boolean
  narrow?: boolean
  newOpen?: boolean
  menuOpen?: boolean
  renaming?: string | null
}>()
const emit = defineEmits<{
  toggle: [open?: boolean]
  focus: [paneKey: string]
  close: [row: Row]
  activateGroup: [group: RailGroup]
  fold: [group: RailGroup, folded: boolean]
  closeGroup: [group: RailGroup]
  rename: [tabKey: string]
  renamed: [tabKey: string, saved: boolean]
  contextGroup: [group: RailGroup, event: MouseEvent]
  new: [event: MouseEvent]
  menu: [event: MouseEvent]
  forget: []
  checkout: [checkout: RailCheckout]
  contextHead: [event: MouseEvent]
  contextCheckout: [checkout: RailCheckout, event: MouseEvent]
  contextRow: [row: Row, event: MouseEvent]
}>()

const { t } = useI18n()
const MOVE_KEYS = { up: keyCombo(['Alt', '↑'], isMac), down: keyCombo(['Alt', '↓'], isMac) }
const name = computed(() => (props.cluster.key === 'home' ? t('workspace.side.home') : props.cluster.name))
/** Folder the header acts on: the active checkout, else the main one. Never assumed allowed beyond what is open. */
const letter = computed(() => (props.cluster.key === 'home' ? '~' : monogram(name.value)))
const tone = computed(() => projectTone(props.cluster.key))
const counts = computed(() => (solo.value ? { running: 0, waiting: 0 } : runCounts(props.cluster)))
const ago = computed(() => {
  const at = props.cluster.live ? lastActive(props.cluster) : null
  if (!at) return null
  const a = agoOf(at, clock.value)
  return a.n === undefined ? t(`railV92.ago.${a.key}`) : t(`railV92.ago.${a.key}`, { n: a.n })
})
const hasSecond = computed(() => !!headBranch.value || counts.value.running > 0 || counts.value.waiting > 0)
const headDir = computed(() => (props.cluster.checkouts.find((k) => k.active) ?? props.cluster.checkouts[0])?.path ?? props.cluster.root)
// A lone checkout has no line: its branch sits on the header, and a remembered project opens from the header.
const showCheckouts = computed(() => showsCheckouts(props.cluster))
const opener = computed(() => (!props.cluster.live && !showCheckouts.value ? (props.cluster.checkouts.find((k) => k.path) ?? null) : null))
const headBranch = computed(() => (showCheckouts.value ? null : (props.cluster.checkouts.find((k) => k.branch)?.branch ?? null)))

const solo = computed(() => isSolo(props.cluster))
const soloRow = computed(() => (solo.value ? props.cluster.checkouts[0]!.rows[0]! : null))
const soloWord = computed(() => {
  const r = soloRow.value
  if (!r) return ''
  const w = stateWord(r.state, r.subagents)
  return w.n === undefined ? t(w.key) : t(w.key, { n: w.n }, w.n)
})
const soloTime = computed(() => {
  const r = soloRow.value
  if (!r || !r.since || r.state === 'idle' || r.state === 'parked' || r.state === 'ended') return null
  return duration(new Date(r.since).toISOString(), clock.value)
})
const soloTone = computed(() => (soloRow.value?.state === 'failed' ? 'text-st-error' : soloRow.value?.state === 'asked' ? 'text-st-waiting' : 'text-ink-faint'))

function onHead(): void {
  if (opener.value) emit('checkout', opener.value)
  else if (soloRow.value) emit('focus', soloRow.value.paneKey)
  else emit('toggle')
}
const isStop = useRailTabStop()
const headStop = computed(() => (isStop(headId(props.cluster.key)) ? 0 : -1))
const { peerView } = usePaneInfo()
const news = computed(() => (props.collapsed && props.cluster.peer ? peerView(props.cluster.peer).label : null))

function checkoutLabel(k: RailCheckout): string {
  return k.branch ?? (k.path ? folderName(k.path) : '')
}

type Entry = RailItem | { kind: 'checkout'; key: string; checkout: RailCheckout }
const entries = computed<Entry[]>(() =>
  props.cluster.checkouts.flatMap((k): Entry[] => [...(showCheckouts.value && k.path ? [{ kind: 'checkout' as const, key: `co:${k.key}`, checkout: k }] : []), ...railItems(k)]),
)

const foldIndex = computed(() => {
  const out = new Map<string, number>()
  const seen = new Map<string, number>()
  for (const e of entries.value) {
    if (e.kind !== 'row' || !e.group) continue
    const n = seen.get(e.group.key) ?? 0
    out.set(e.key, n)
    seen.set(e.group.key, n + 1)
  }
  return out
})

const workspace = useWorkspaceStore()
function onLeave(el: Element, done: () => void): void {
  const node = el as HTMLElement
  const pane = node.dataset.leavePane
  const tab = node.dataset.leaveTab
  const gone = (!!pane && !workspace.panes.some((p) => p.key === pane)) || (!!tab && !workspace.tabs.some((t) => t.key === tab))
  if (gone) onDustLeave(el, done)
  else setTimeout(done, motionMs(150))
}

const HEAD_BTN =
  'inline-flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-[6px] text-ink-faint hover:bg-fill-strong hover:text-ink aria-expanded:bg-fill-strong aria-expanded:text-ink'
</script>

<template>
  <section class="flex flex-col" :data-group="cluster.key">
    <div
      class="group/head flex min-h-9 min-w-0 items-center gap-0.5 rounded-control pr-1 transition-colors duration-(--ccm-dur-fast)"
      :class="cluster.active ? 'bg-selection' : 'hover:bg-fill'"
      @contextmenu.prevent.stop="emit('contextHead', $event)"
    >
      <h3 :id="`ws-rail-${index}`" class="min-w-0 flex-1" role="none">
        <button
          type="button"
          data-side-item
          data-side-head
          :data-side-id="headId(cluster.key)"
          :data-order-key="cluster.key"
          class="flex min-h-9 w-full min-w-0 cursor-pointer items-center gap-2 rounded-control py-1 pl-1 text-left text-xs hover:text-ink"
          :class="[cluster.live ? 'text-ink' : 'text-ink-muted', cluster.active ? 'font-semibold' : 'font-medium']"
          role="treeitem"
          aria-level="1"
          :aria-expanded="opener || solo ? undefined : !collapsed"
          :tabindex="headStop"
          :title="[cluster.root, headBranch, t('workspace.rail.moveHint', MOVE_KEYS)].filter(Boolean).join('\n')"
          @click="onHead"
        >
          <ChevronRight
            v-if="!opener && !solo"
            :size="12"
            class="shrink-0 text-ink-faint transition-transform duration-(--ccm-dur-fast) ease-out-quint"
            :class="collapsed ? '' : 'rotate-90'"
            aria-hidden="true"
          />
          <span v-else class="w-3 shrink-0" aria-hidden="true" />
          <span class="ccm-mono" :class="cluster.live ? '' : 'ccm-mono-off'" :data-tone="tone" aria-hidden="true" data-rail-folder>{{ letter }}</span>
          <span class="flex min-w-0 flex-1 flex-col gap-px">
            <span class="flex min-w-0 items-center gap-1.5">
              <span class="min-w-0 flex-1 truncate" data-rail-name>{{ name }}</span>
              <span v-if="soloRow" class="inline-flex shrink-0 items-center gap-1 text-[11px] font-normal tabular-nums" :class="soloTone" data-rail-solo-state :data-state="soloRow.state">
                <LoaderCircle v-if="soloRow.state === 'starting'" :size="11" class="ccm-spin" aria-hidden="true" />
                <StatusIcon v-else-if="RAIL_BADGE[soloRow.state]" :status="RAIL_BADGE[soloRow.state]!" :size="11" />
                <span v-if="soloTime">{{ soloTime }}</span>
                <span class="sr-only">{{ soloWord }}</span>
              </span>
              <span v-else-if="ago" class="shrink-0 text-[11px] font-normal text-ink-faint tabular-nums" :title="t('railV92.agoTitle', { when: ago })" data-rail-ago>{{ ago }}</span>
            </span>
            <span v-if="hasSecond" class="flex min-w-0 items-center gap-2 text-[11px] font-normal text-ink-muted">
              <span v-if="counts.running" class="inline-flex shrink-0 items-center gap-1 text-st-working tabular-nums" data-rail-running>
                <span class="size-1.5 shrink-0 rounded-full bg-st-working" aria-hidden="true" />{{ t('railV92.running', { n: counts.running }) }}
              </span>
              <span v-if="counts.waiting" class="inline-flex shrink-0 items-center gap-1 text-st-waiting tabular-nums" data-rail-waiting>
                <Hand :size="10" aria-hidden="true" />{{ t('railV92.waiting', { n: counts.waiting }) }}
              </span>
              <span v-if="headBranch" class="inline-flex min-w-0 items-center gap-0.5 text-ink-faint" data-rail-branch>
                <GitBranch :size="11" class="shrink-0" aria-hidden="true" />
                <span class="min-w-0 truncate">{{ headBranch }}</span>
              </span>
            </span>
          </span>
        </button>
      </h3>
      <span role="group" class="flex shrink-0 items-center gap-0.5" :aria-label="t('workspace.rail.menu.label', { name })">
        <Transition name="ccm-fade">
          <span v-if="news" class="inline-flex size-4 shrink-0 items-center justify-center text-peer" :title="news" data-rail-cluster-peer>
            <Mail :size="11" aria-hidden="true" /><span class="sr-only">{{ news }}</span>
          </span>
        </Transition>
        <span
          v-if="cluster.needs"
          class="inline-flex shrink-0 items-center gap-0.5 rounded-full px-1.5 text-2xs font-semibold tabular"
          :class="cluster.failed ? 'bg-st-error-soft text-st-error' : 'bg-st-waiting-soft text-st-waiting'"
          :title="t('workspace.rail.needs', { n: cluster.needs }, cluster.needs)"
          data-rail-needs
        >
          <BellRing :size="10" aria-hidden="true" />{{ cluster.needs }}<span class="sr-only"> {{ t('workspace.rail.needs', { n: cluster.needs }, cluster.needs) }}</span>
        </span>
        <RunBadge v-if="headDir" :dir="headDir" :compact="narrow" :tabindex="headStop" />
        <button
          v-if="solo"
          type="button"
          :class="HEAD_BTN"
          :title="t(collapsed ? 'workspace.rail.menu.expand' : 'workspace.rail.menu.collapse')"
          :aria-label="t(collapsed ? 'workspace.rail.menu.expand' : 'workspace.rail.menu.collapse')"
          :aria-expanded="!collapsed"
          :tabindex="headStop"
          data-rail-solo-toggle
          data-no-drag
          @click="emit('toggle')"
        >
          <ChevronRight :size="12" class="transition-transform duration-(--ccm-dur-fast) ease-out-quint" :class="collapsed ? '' : 'rotate-90'" aria-hidden="true" />
        </button>
        <button
          v-if="headDir || cluster.key === 'home'"
          type="button"
          :class="HEAD_BTN"
          class="opacity-0 transition-opacity duration-(--ccm-dur-fast) ease-out-quint group-hover/head:opacity-100 group-focus-within/head:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
          :title="t('workspace.rail.newIn', { name })"
          :aria-label="t('workspace.rail.newIn', { name })"
          aria-haspopup="true"
          :aria-expanded="!!newOpen"
          :tabindex="headStop"
          data-side-new
          data-no-drag
          @click="emit('new', $event)"
        >
          <Plus :size="13" aria-hidden="true" />
        </button>
        <button
          v-if="cluster.live && headDir"
          type="button"
          :class="HEAD_BTN"
          class="opacity-0 transition-opacity duration-(--ccm-dur-fast) ease-out-quint group-hover/head:opacity-100 group-focus-within/head:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
          :title="t('workspace.projects.menuFor', { name })"
          :aria-label="t('workspace.projects.menuFor', { name })"
          aria-haspopup="true"
          :aria-expanded="!!menuOpen"
          :tabindex="headStop"
          data-side-menu
          data-no-drag
          @click="emit('menu', $event)"
        >
          <Ellipsis :size="14" aria-hidden="true" />
        </button>
        <button
          v-if="!cluster.live"
          type="button"
          :class="HEAD_BTN"
          class="opacity-0 transition-opacity duration-(--ccm-dur-fast) ease-out-quint group-hover/head:opacity-100 group-focus-within/head:opacity-100 focus-visible:opacity-100"
          :title="t('workspace.rail.forget', { name })"
          :aria-label="t('workspace.rail.forget', { name })"
          :tabindex="headStop"
          data-rail-forget
          data-no-drag
          @click="emit('forget')"
        >
          <X :size="12" aria-hidden="true" />
        </button>
      </span>
    </div>
    <div class="ccm-rail-fold" :data-folded="collapsed ? '' : undefined" :inert="collapsed" data-rail-cluster-body>
      <div class="flex flex-col">
      <TransitionGroup tag="ul" name="ccm-list" class="relative flex flex-col" role="group" :aria-labelledby="`ws-rail-${index}`" @before-leave="pinLeaving" @leave="onLeave">
        <li
          v-for="item in entries"
          :key="item.key"
          role="none"
          :class="(item.kind === 'row' && item.group) || item.kind === 'sub' ? 'ccm-rail-fold' : ''"
          :data-folded="(item.kind === 'row' || item.kind === 'sub') && item.group?.folded ? '' : undefined"
          :inert="(item.kind === 'row' || item.kind === 'sub') && !!item.group?.folded"
          :style="item.kind === 'row' && item.group ? { '--fold-i': foldIndex.get(item.key) ?? 0 } : undefined"
          :data-leave-pane="item.kind === 'row' ? item.row.paneKey : undefined"
          :data-leave-tab="item.kind === 'group' ? item.group.tabKey : undefined"
        >
          <button
            v-if="item.kind === 'checkout'"
            type="button"
            data-side-item
            data-side-checkout
            :data-side-id="checkoutId(cluster.key, item.checkout.key)"
            :data-checkout-key="item.checkout.key"
            role="treeitem"
            aria-level="2"
            :tabindex="isStop(checkoutId(cluster.key, item.checkout.key)) ? 0 : -1"
            class="flex h-6 w-full min-w-0 cursor-pointer items-center gap-1.5 rounded-inner pr-1.5 pl-9 text-left text-2xs text-ink-muted hover:bg-fill hover:text-ink"
            :title="item.checkout.path ?? undefined"
            @click="emit('checkout', item.checkout)"
            @contextmenu.prevent.stop="emit('contextCheckout', item.checkout, $event)"
          >
            <component :is="item.checkout.branch ? GitBranch : Folder" :size="11" class="shrink-0" :class="item.checkout.branch ? 'text-ink-faint' : 'fill-accent-soft text-accent'" aria-hidden="true" />
            <span class="min-w-0 truncate">{{ checkoutLabel(item.checkout) }}</span>
            <span v-if="item.checkout.tag" class="shrink-0 rounded-[4px] bg-fill px-1 text-ink-faint">{{ t(`workspace.rail.${item.checkout.tag}`) }}</span>
          </button>
          <RailGroupHead
            v-else-if="item.kind === 'group'"
            :group="item.group"
            :renaming="renaming === item.group.tabKey"
            @activate="(g) => emit('activateGroup', g)"
            @fold="(g, f) => emit('fold', g, f)"
            @close="(g) => emit('closeGroup', g)"
            @rename="(g) => emit('rename', g.tabKey)"
            @renamed="(k, saved) => emit('renamed', k, saved)"
            @menu="(g, e) => emit('contextGroup', g, e)"
          />
          <div
            v-else-if="item.kind === 'sub'"
            class="flex h-5 min-w-0 items-center gap-1.5 pr-1.5 pl-9 text-2xs font-semibold text-ink-muted"
            :title="item.project.key"
            data-rail-subgroup
          >
            <span class="ccm-mono ccm-mono-sm" :data-tone="item.project.tone" aria-hidden="true">{{ monogram(item.project.name) }}</span>
            <span class="min-w-0 truncate">{{ item.project.name || t('groupsV99.home') }}</span>
            <span class="ml-auto shrink-0 font-normal tabular-nums text-ink-faint">{{ item.project.count }}</span>
          </div>
          <div v-else>
            <RailRow
              :row="item.row"
              :member="!!item.group"
              :renaming="!item.group && renaming === item.row.tabKey"
              @focus="(key) => emit('focus', key)"
              @close="(r) => emit('close', r)"
              @menu="(r, e) => emit('contextRow', r, e)"
              @renamed="(k, saved) => emit('renamed', k, saved)"
            />
          </div>
        </li>
      </TransitionGroup>
      </div>
    </div>
  </section>
</template>

<style scoped>
.ccm-mono {
  display: inline-flex;
  width: 22px;
  height: 22px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-inner);
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  color: var(--ccm-mono-ink);
  background: color-mix(in srgb, var(--mono) 62%, var(--ccm-mono-mix));
}
.ccm-mono[data-tone='0'] { --mono: var(--ccm-agent-claude); }
.ccm-mono[data-tone='1'] { --mono: var(--ccm-agent-codex); }
.ccm-mono[data-tone='2'] { --mono: var(--ccm-agent-browser); }
.ccm-mono[data-tone='3'] { --mono: var(--ccm-agent-file); }
.ccm-mono[data-tone='4'] { --mono: var(--ccm-success); }
.ccm-mono[data-tone='5'] { --mono: var(--ccm-peer); }
.ccm-mono[data-tone='6'] { --mono: var(--ccm-warning); }
.ccm-mono[data-tone='7'] { --mono: var(--ccm-accent); }
.ccm-mono-sm {
  width: 14px;
  height: 14px;
  font-size: 9px;
}
.ccm-mono-off {
  background: transparent;
  color: var(--ccm-ink-muted);
  box-shadow: inset 0 0 0 1px var(--ccm-line-strong);
}
</style>
