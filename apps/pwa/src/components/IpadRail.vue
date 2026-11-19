<script setup lang="ts">
import { Activity, ChevronDown, ChevronLeft, Clock, LayoutGrid, LoaderCircle, Check, Laptop, MoonStar, PanelLeftClose, PanelLeftOpen, QrCode, SlidersHorizontal, TriangleAlert } from 'lucide-vue-next'
import { computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatElapsed, parseSessions, type MonitorSession, type Rail } from '../lib/monitorModel'
import { quotaOf } from '../lib/quotaModel'
import { groupByProject } from '../lib/projectModel'
import { attentionCount } from '../lib/railModel'
import { useNow } from '../lib/useNow'
import { monogram, parseWorkspace, toneOfProject, type WsPane } from '../lib/workspaceModel'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'
import LinkChip from './LinkChip.vue'
import QuotaBars from './QuotaBars.vue'

const props = defineProps<{ machine: string; active: 'monitor' | 'workspace' | 'settings'; terminal?: string; pane?: string }>()
const { t } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()
const now = useNow(20_000)
const folds = reactive(new Map<string, boolean>())

const collapsed = computed(() => prefs.local.railCollapsed)
const known = computed(() => hub.machines.find((m) => m.id === props.machine))
const name = computed(() => known.value?.alias ?? known.value?.name ?? props.machine)
const view = computed(() => hub.viewOf(props.machine))
const link = computed(() => hub.linkOf(props.machine))
const sessions = computed(() => parseSessions(view.value.snapshot))
const groups = computed(() => groupByProject(sessions.value))
const tabs = computed(() => parseWorkspace(view.value.workspace, sessions.value))
const wsMode = computed(() => props.active === 'workspace')
const waiting = computed(() => attentionCount(sessions.value))
const quota = computed(() => quotaOf(view.value.snapshot, now.value))

const STATUS: Record<Rail, { icon: typeof Clock; tone: string; spin: boolean }> = {
  working: { icon: LoaderCircle, tone: 'working', spin: true },
  asked: { icon: Clock, tone: 'waiting', spin: false },
  failed: { icon: TriangleAlert, tone: 'error', spin: false },
  done: { icon: Check, tone: 'done', spin: false },
  idle: { icon: MoonStar, tone: 'idle', spin: false },
}

const isFolded = (key: string): boolean => folds.get(key) === true
const toggle = (key: string): void => {
  folds.set(key, !isFolded(key))
}
const since = (s: { since: number | null }): string => (s.since ? formatElapsed(now.value - s.since, t) : '')
const groupName = (n: string | null): string => n || t('rail.noProject')
const paneLine = (p: WsPane): string => `${t(`provider.${p.provider}`)} · ${t(`status.${p.rail}`)}`
const sessionLine = (s: MonitorSession): string => `${t(`provider.${s.provider}`)} · ${t(`status.${s.rail}`)}`

function go(target: 'monitor' | 'workspace'): void {
  navigate(target === 'monitor' ? { name: 'monitor', machine: props.machine } : { name: 'workspace', machine: props.machine }, { replace: true })
}
</script>

<template>
  <nav class="rail glass-bar" :class="{ collapsed }" :aria-label="t('rail.label')" data-testid="ipad-rail" :data-collapsed="collapsed ? '1' : '0'">
    <header class="top">
      <button type="button" class="icon" :aria-label="t('rail.machines')" data-testid="rail-machines" @click="navigate({ name: 'home' })"><ChevronLeft :size="22" :stroke-width="2" aria-hidden="true" /></button>
      <template v-if="!collapsed">
        <span class="tone-mono big" :data-tone="toneOfProject(machine)" aria-hidden="true">{{ monogram(name) }}</span>
        <span class="who"><span class="mname" data-testid="rail-machine">{{ name }}</span></span>
        <LinkChip :link="link" :rtt="hub.rtt[machine]" />
      </template>
      <button type="button" class="icon" :aria-label="t(collapsed ? 'rail.expand' : 'rail.collapse')" :aria-expanded="!collapsed" data-testid="rail-collapse" @click="prefs.setRailCollapsed(!collapsed)">
        <PanelLeftOpen v-if="collapsed" :size="20" :stroke-width="2" aria-hidden="true" />
        <PanelLeftClose v-else :size="20" :stroke-width="2" aria-hidden="true" />
      </button>
    </header>

    <div class="seg" role="tablist">
      <button type="button" role="tab" class="seg-btn" :class="{ on: active === 'monitor' }" :aria-selected="active === 'monitor'" :aria-label="t('tabs.monitor')" data-testid="rail-seg-monitor" @click="go('monitor')">
        <span class="ico"><Activity :size="18" :stroke-width="2" aria-hidden="true" /><span v-if="waiting > 0 && collapsed" class="dot" aria-hidden="true" /></span>
        <span v-if="!collapsed">{{ t('tabs.monitor') }}</span>
        <span v-if="!collapsed && waiting > 0" class="badge" data-testid="rail-badge">{{ waiting }}</span>
      </button>
      <button type="button" role="tab" class="seg-btn" :class="{ on: active === 'workspace' }" :aria-selected="active === 'workspace'" :aria-label="t('tabs.workspace')" data-testid="rail-seg-workspace" @click="go('workspace')">
        <span class="ico"><LayoutGrid :size="18" :stroke-width="2" aria-hidden="true" /></span>
        <span v-if="!collapsed">{{ t('tabs.workspace') }}</span>
      </button>
    </div>

    <div v-if="!collapsed" class="body" data-testid="rail-body">
      <template v-if="active !== 'workspace'">
        <p v-if="groups.length === 0" class="none">{{ t('rail.empty') }}</p>
        <section v-for="g in groups" :key="g.key" class="grp" :data-testid="`rail-group-${g.key}`">
          <button type="button" class="gh" :aria-expanded="!isFolded(g.key)" @click="toggle(g.key)">
            <ChevronDown class="chev" :class="{ off: isFolded(g.key) }" :size="16" :stroke-width="2.2" aria-hidden="true" />
            <span class="tone-mono" :data-tone="g.tone" aria-hidden="true">{{ g.letter }}</span>
            <span class="gname">{{ groupName(g.name) }}</span>
            <span class="gcount">{{ g.sessions.length }}</span>
          </button>
          <div v-if="!isFolded(g.key)" class="items by-project">
            <button v-for="s in g.sessions" :key="s.id" type="button" class="row" :class="{ on: terminal === s.id }" :aria-current="terminal === s.id ? 'true' : undefined" :data-testid="`rail-session-${s.id}`" :data-rail="s.rail" @click="navigate({ name: 'session', machine, terminal: s.id })">
              <component :is="STATUS[s.rail].icon" class="st" :class="[`tone-${STATUS[s.rail].tone}`, { spin: STATUS[s.rail].spin }]" :size="18" :stroke-width="2.2" aria-hidden="true" />
              <span class="text">
                <span class="l1"><span class="title">{{ s.title }}</span><span class="since">{{ since(s) }}</span></span>
                <span class="l2">{{ sessionLine(s) }}</span>
              </span>
            </button>
          </div>
        </section>
      </template>
      <template v-else>
        <p v-if="tabs.length === 0" class="none">{{ t('rail.emptyWorkspace') }}</p>
        <section v-for="x in tabs" :key="x.key" class="grp" :data-testid="`rail-tab-${x.key}`">
          <button type="button" class="gh" :aria-expanded="!isFolded(`t:${x.key}`)" @click="toggle(`t:${x.key}`)">
            <ChevronDown class="chev" :class="{ off: isFolded(`t:${x.key}`) }" :size="16" :stroke-width="2.2" aria-hidden="true" />
            <span class="tone-mono" :data-tone="x.groups[0]?.tone ?? toneOfProject(x.project ?? x.key)" aria-hidden="true">{{ monogram(x.name) }}</span>
            <span class="gname">{{ x.name }}</span>
            <span class="gcount">{{ x.panes.length }}</span>
          </button>
          <div v-if="!isFolded(`t:${x.key}`)" class="items">
            <template v-if="x.groups.length > 0">
              <div v-for="g in x.groups" :key="g.key" class="sub" :data-testid="`rail-pgroup-${g.key}`">
                <button type="button" class="sh" :aria-expanded="!isFolded(`g:${x.key}:${g.key}`)" @click="toggle(`g:${x.key}:${g.key}`)">
                  <ChevronDown class="chev" :class="{ off: isFolded(`g:${x.key}:${g.key}`) }" :size="14" :stroke-width="2.2" aria-hidden="true" />
                  <span class="dotc" :data-tone="g.tone" aria-hidden="true" />
                  <span class="gname">{{ groupName(g.name) }}</span>
                  <span class="gcount">{{ g.panes.length }}</span>
                </button>
                <div v-if="!isFolded(`g:${x.key}:${g.key}`)" class="items inner">
                  <button v-for="p in g.panes" :key="p.key" type="button" class="row" :class="{ on: pane === p.key }" :aria-current="pane === p.key ? 'true' : undefined" :data-testid="`rail-pane-${p.key}`" :data-rail="p.rail" @click="navigate({ name: 'workspace', machine, pane: p.key })">
                    <component :is="STATUS[p.rail].icon" class="st" :class="[`tone-${STATUS[p.rail].tone}`, { spin: STATUS[p.rail].spin }]" :size="18" :stroke-width="2.2" aria-hidden="true" />
                    <span class="text"><span class="l1"><span class="title">{{ p.title }}</span><span class="since">{{ since(p) }}</span></span><span class="l2">{{ paneLine(p) }}</span></span>
                  </button>
                </div>
              </div>
            </template>
            <button v-for="p in x.groups.length > 0 ? [] : x.panes" :key="p.key" type="button" class="row" :class="{ on: pane === p.key }" :aria-current="pane === p.key ? 'true' : undefined" :data-testid="`rail-pane-${p.key}`" :data-rail="p.rail" @click="navigate({ name: 'workspace', machine, pane: p.key })">
              <component :is="STATUS[p.rail].icon" class="st" :class="[`tone-${STATUS[p.rail].tone}`, { spin: STATUS[p.rail].spin }]" :size="18" :stroke-width="2.2" aria-hidden="true" />
              <span class="text"><span class="l1"><span class="title">{{ p.title }}</span><span class="since">{{ since(p) }}</span></span><span class="l2">{{ paneLine(p) }}</span></span>
            </button>
          </div>
        </section>
      </template>
    </div>
    <div v-else class="body spacer" />

    <footer class="foot">
      <QuotaBars v-if="quota.length > 0 && !collapsed" class="quota" :rows="quota" :now="now" data-testid="rail-quota" />
      <div class="icons">
        <button type="button" class="icon" :aria-label="t('rail.machines')" data-testid="rail-home" @click="navigate({ name: 'home' })"><Laptop :size="20" :stroke-width="2" aria-hidden="true" /></button>
        <button type="button" class="icon" :aria-label="t('rail.pair')" data-testid="rail-pair" @click="navigate({ name: 'pair' })"><QrCode :size="20" :stroke-width="2" aria-hidden="true" /></button>
        <button type="button" class="icon" :class="{ on: active === 'settings' }" :aria-label="t('tabs.settings')" :aria-current="active === 'settings' ? 'page' : undefined" data-testid="rail-settings" @click="navigate({ name: 'settings' }, { replace: true })"><SlidersHorizontal :size="20" :stroke-width="2" aria-hidden="true" /></button>
      </div>
    </footer>
  </nav>
</template>

<style scoped>
.rail {
  display: flex;
  flex: none;
  flex-direction: column;
  width: 300px;
  min-height: 0;
  margin: 12px 0 calc(12px + var(--ccm-safe-bottom)) 12px;
  padding: 10px;
  gap: 8px;
  border: 1px solid var(--ccm-glass-edge);
  border-radius: calc(var(--radius-card) + 4px);
  box-shadow: var(--ccm-shadow-pop), inset 0 1px 0 var(--ccm-glass-edge);
  transition: width var(--ccm-dur-base) var(--ccm-ease-standard);
}

.rail.collapsed {
  width: 68px;
  align-items: center;
}

.top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.collapsed .top {
  flex-direction: column;
}

.who {
  flex: 1;
  min-width: 0;
}

.mname {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 800;
}

.big {
  width: 36px;
  height: 36px;
  font-size: 1rem;
}

.icon {
  display: grid;
  place-items: center;
  flex: none;
  width: 44px;
  height: 44px;
  border: 0;
  border-radius: var(--radius-control);
  background: transparent;
  color: var(--ccm-ink-muted);
}

.icon.on {
  background: var(--ccm-accent-soft);
  color: var(--ccm-accent-ink);
}

.seg {
  display: flex;
  gap: 4px;
  padding: 3px;
  border-radius: var(--radius-control);
  background: var(--ccm-control);
}

.collapsed .seg {
  flex-direction: column;
}

.seg-btn {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 44px;
  min-width: 44px;
  border: 0;
  border-radius: calc(var(--radius-control) - 2px);
  background: transparent;
  color: var(--ccm-ink-muted);
  font-weight: 700;
}

.seg-btn.on {
  background: var(--ccm-surface);
  color: var(--ccm-ink);
  box-shadow: 0 0 0 1px var(--ccm-line);
}

.ico {
  position: relative;
  display: grid;
}

.dot {
  position: absolute;
  top: -2px;
  right: -3px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--ccm-waiting);
}

.badge {
  min-width: 20px;
  padding: 1px 6px;
  border-radius: var(--radius-chip);
  background: var(--ccm-waiting-soft);
  color: var(--ccm-waiting);
  font-size: 0.75rem;
  font-weight: 800;
  text-align: center;
}

.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  scrollbar-width: none;
}

.body::-webkit-scrollbar {
  display: none;
}

.none {
  margin: 12px 8px;
  color: var(--ccm-ink-faint);
}

.gh,
.sh {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 44px;
  padding: 0 6px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
  font-weight: 700;
  text-align: left;
}

.sh {
  min-height: 36px;
  padding-left: 14px;
  color: var(--ccm-ink-muted);
  font-size: 0.8125rem;
}

.gname {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gcount {
  font-size: 0.75rem;
  color: var(--ccm-ink-faint);
}

.chev {
  flex: none;
  color: var(--ccm-ink-faint);
  transition: transform var(--ccm-dur-base) var(--ccm-ease-standard);
}

.chev.off {
  transform: rotate(-90deg);
}

.dotc {
  width: 8px;
  height: 8px;
  flex: none;
  border-radius: 50%;
  background: var(--ccm-agent-claude);
}

.dotc[data-tone='1'] { background: var(--ccm-agent-codex); }
.dotc[data-tone='2'] { background: var(--ccm-working); }
.dotc[data-tone='3'] { background: var(--ccm-agent-shell); }
.dotc[data-tone='4'] { background: var(--ccm-done); }
.dotc[data-tone='5'] { background: var(--ccm-peer); }
.dotc[data-tone='6'] { background: var(--ccm-warning); }
.dotc[data-tone='7'] { background: var(--ccm-accent); }

.items {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding-left: 14px;
}

.items.by-project {
  padding-left: 32px;
}

.items.inner {
  padding-left: 36px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 52px;
  padding: 6px 10px;
  border: 0;
  border-radius: var(--radius-control);
  background: transparent;
  color: var(--ccm-ink);
  text-align: left;
}

.row.on {
  background: var(--ccm-accent-soft);
}

.st {
  flex: none;
}

.text {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 1px;
}

.l1 {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 700;
}

.since {
  flex: none;
  color: var(--ccm-ink-faint);
  font-size: 0.75rem;
}

.l2 {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--ccm-ink-muted);
  font-size: 0.75rem;
}

.tone-working { color: var(--ccm-working); }
.tone-waiting { color: var(--ccm-waiting); }
.tone-error { color: var(--ccm-error); }
.tone-done { color: var(--ccm-done); }
.tone-idle { color: var(--ccm-ink-faint); }

.spin {
  animation: spin 1.1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.foot {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: 8px;
  border-top: 1px solid var(--ccm-line);
}

.quota {
  padding: 0 4px;
}

.icons {
  display: flex;
  justify-content: space-around;
}

.collapsed .icons {
  flex-direction: column;
  align-items: center;
}
</style>
