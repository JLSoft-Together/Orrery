<script setup lang="ts">
import { TriangleAlert, X } from 'lucide-vue-next'
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ConfirmHost from './components/ConfirmHost.vue'
import RemoteConfirmHost from './components/RemoteConfirmHost.vue'
import TabBar, { type TabId } from './components/TabBar.vue'
import UpdateBar from './components/UpdateBar.vue'
import { isWaiting, parseSessions } from './lib/monitorModel'
import { useWide } from './lib/useWide'
import { navigate, route, routeDirection, type Route } from './router'
import { useHubStore } from './stores/hub'
import { useLockStore } from './stores/lock'
import { usePrefsStore } from './stores/prefs'
import DebugView from './views/DebugView.vue'
import HomeView from './views/HomeView.vue'
import LockView from './views/LockView.vue'
import MonitorView from './views/MonitorView.vue'
import PairView from './views/PairView.vue'
import SessionView from './views/SessionView.vue'
import SettingsView from './views/SettingsView.vue'
import TerminalView from './views/TerminalView.vue'
import WorkspaceView from './views/WorkspaceView.vue'

const { t } = useI18n()
const hub = useHubStore()
const lock = useLockStore()
const prefs = usePrefsStore()
const wide = useWide()

const master = computed<Route>(() => {
  const r = route.value
  if (wide.value && r.name === 'session') return { name: 'monitor', machine: r.machine }
  if (wide.value && r.name === 'workspace' && r.pane) return { name: 'workspace', machine: r.machine }
  return r
})
const fullTerminal = computed(() => !wide.value && route.value.name === 'workspace' && route.value.pane !== undefined)
const viewKey = computed(() => JSON.stringify(master.value))
const selectedTerminal = computed(() => (route.value.name === 'session' ? route.value.terminal : undefined))
const sessionRoute = computed(() => (route.value.name === 'session' ? route.value : null))
const paneRoute = computed(() => (route.value.name === 'workspace' && route.value.pane ? { machine: route.value.machine, pane: route.value.pane } : null))
const renderReady = computed(() => hub.ready || hub.bootError !== null)

const tabMachine = computed(() => {
  const r = master.value
  if (r.name === 'monitor' || r.name === 'workspace') return r.machine
  const last = prefs.local.lastMachine
  return last && hub.machines.some((m) => m.id === last) ? last : (hub.machines[0]?.id ?? null)
})
const activeTab = computed<TabId | null>(() => {
  switch (master.value.name) {
    case 'monitor':
      return 'monitor'
    case 'workspace':
      return 'workspace'
    case 'settings':
      return 'settings'
    default:
      return null
  }
})
const showTabs = computed(() => !fullTerminal.value && activeTab.value !== null && (activeTab.value === 'settings' ? tabMachine.value !== null : true))
const badge = computed(() => (tabMachine.value ? parseSessions(hub.viewOf(tabMachine.value).snapshot).filter((s) => isWaiting(s.rail)).length : 0))

function selectTab(tab: TabId): void {
  const machine = tabMachine.value
  if (tab === 'settings') navigate({ name: 'settings' }, { replace: true })
  else if (machine) navigate(tab === 'monitor' ? { name: 'monitor', machine } : { name: 'workspace', machine }, { replace: true })
}

watch(
  route,
  (r) => {
    if (r.name === 'notfound') navigate({ name: 'home' }, { replace: true })
    if ('machine' in r) prefs.setLastMachine(r.machine)
  },
  { immediate: true },
)
</script>

<template>
  <LockView v-if="lock.locked" />
  <div v-else class="shell" :data-ready="hub.ready ? '1' : '0'">
    <UpdateBar />
    <div class="notices">
      <div v-if="hub.bootError" class="notice" role="alert" data-testid="boot-error">
        <TriangleAlert :size="18" :stroke-width="2.2" aria-hidden="true" />
        <span class="notice-text">{{ t('boot.error') }}</span>
      </div>
      <div v-for="notice in hub.notices" :key="notice.id" class="notice" role="alert" data-testid="notice">
        <TriangleAlert :size="18" :stroke-width="2.2" aria-hidden="true" />
        <span class="notice-text">{{ t('notice.revoked', { name: notice.name }) }}</span>
        <button type="button" class="notice-close" :aria-label="t('notice.dismiss')" @click="hub.dismiss(notice.id)"><X :size="18" :stroke-width="2.2" aria-hidden="true" /></button>
      </div>
    </div>
    <div class="body">
      <div class="master">
        <main class="scroll" :class="{ fill: fullTerminal }">
          <div v-if="renderReady" :key="viewKey" class="view" :class="[routeDirection, { fill: fullTerminal }]">
            <HomeView v-if="master.name === 'home'" />
            <PairView v-else-if="master.name === 'pair'" />
            <MonitorView v-else-if="master.name === 'monitor'" :machine="master.machine" :selected="selectedTerminal" />
            <SessionView v-else-if="master.name === 'session'" :machine="master.machine" :terminal="master.terminal" />
            <TerminalView v-else-if="master.name === 'workspace' && master.pane" :machine="master.machine" :pane="master.pane" />
            <WorkspaceView v-else-if="master.name === 'workspace'" :machine="master.machine" :selected="paneRoute?.pane" />
            <SettingsView v-else-if="master.name === 'settings'" />
            <DebugView v-else-if="master.name === 'debug'" />
          </div>
        </main>
        <TabBar v-if="showTabs" :active="activeTab" :badge="badge" @select="selectTab" />
      </div>
      <aside v-if="wide" class="detail" data-testid="detail-pane">
        <SessionView v-if="sessionRoute" :key="sessionRoute.terminal" :machine="sessionRoute.machine" :terminal="sessionRoute.terminal" embedded />
        <TerminalView v-else-if="paneRoute" :key="paneRoute.pane" :machine="paneRoute.machine" :pane="paneRoute.pane" embedded />
        <p v-else class="placeholder">{{ t('machines.detailEmpty') }}</p>
      </aside>
    </div>
    <ConfirmHost />
    <RemoteConfirmHost />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding-top: env(safe-area-inset-top);
  padding-left: env(safe-area-inset-left);
  padding-right: env(safe-area-inset-right);
}

.notices {
  display: flex;
  flex-direction: column;
}

.notice {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--ccm-surface);
  border-bottom: 1px solid var(--ccm-line);
  color: var(--ccm-error);
  font-weight: 600;
}

.notice-text {
  flex: 1;
  color: var(--ccm-ink);
}

.notice-close {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.master {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.scroll.fill {
  overflow: hidden;
}

.view {
  min-height: 100%;
}

.view.fill {
  height: 100%;
}

.view.push {
  animation: push 300ms var(--ease-out);
}

.view.pop {
  animation: pop 210ms var(--ease-out);
}

@keyframes push {
  from {
    opacity: 0;
    transform: translateX(24px);
  }
}

@keyframes pop {
  from {
    opacity: 0;
    transform: translateX(-12px);
  }
}

@media (min-width: 768px) {
  .master {
    flex: 0 0 380px;
    border-right: 1px solid var(--ccm-line);
  }

  .detail {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .placeholder {
    display: grid;
    place-items: center;
    height: 100%;
    margin: 0;
    padding: 24px;
    color: var(--ccm-ink-faint);
    text-align: center;
  }
}
</style>
