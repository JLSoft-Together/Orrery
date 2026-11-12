<script setup lang="ts">
import { ChevronDown, ChevronLeft, FolderTree, Link2Off, StopCircle, Eye } from 'lucide-vue-next'
import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import InputBar from '../components/InputBar.vue'
import KeyBar from '../components/KeyBar.vue'
import PaneSwitchSheet from '../components/PaneSwitchSheet.vue'
import PermissionBar from '../components/PermissionBar.vue'
import StatusChip from '../components/StatusChip.vue'
import TerminalSurface, { type SurfaceState } from '../components/TerminalSurface.vue'
import { parseSessions } from '../lib/monitorModel'
import { findPane, parseWorkspace } from '../lib/workspaceModel'
import { RemoteCommandError } from '../remote/types'
import { back, navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { keySequence, type KeyId } from '../terminal/keys'
import { StickyModifiers } from '../terminal/modifiers'
import ViewerPage from './ViewerPage.vue'

const props = defineProps<{ machine: string; pane: string; embedded?: boolean }>()
const { t } = useI18n()
const hub = useHubStore()
const mods = reactive(new StickyModifiers())
const surface = ref<InstanceType<typeof TerminalSurface> | null>(null)
const direct = ref(false)
const state = ref<SurfaceState>('attaching')
const detail = ref<string | null>(null)
const sheet = ref(false)
const viewer = ref(false)
const toast = ref<string | null>(null)
const waitedOut = ref(false)
let toastTimer: ReturnType<typeof setTimeout> | null = null
const waitTimer = setTimeout(() => {
  waitedOut.value = true
}, 20_000)

const known = computed(() => hub.machines.some((m) => m.id === props.machine))
const view = computed(() => hub.viewOf(props.machine))
const tabs = computed(() => parseWorkspace(view.value.workspace, parseSessions(view.value.snapshot)))
const found = computed(() => findPane(tabs.value, props.pane))
const online = computed(() => hub.linkOf(props.machine).status === 'ready')
const viewOnly = computed(() => hub.viewOnly[props.machine] === true)
const canType = computed(() => online.value && !viewOnly.value && state.value === 'live')
const gone = computed(() => found.value === null && (view.value.workspace !== null || waitedOut.value))
const overlay = computed(() => {
  if (found.value && !found.value.pane.canAttach) return { icon: StopCircle, text: t('term.notTerminal') }
  if (state.value === 'error') return { icon: Link2Off, text: detail.value === 'limit' ? t('term.attachLimit') : t(`err.${detail.value ?? 'internal'}`) }
  if (state.value === 'ended') return { icon: StopCircle, text: detail.value === null ? t('term.lost') : t('term.ended', { code: detail.value }) }
  return null
})

function say(text: string): void {
  toast.value = text
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => {
    toast.value = null
  }, 4000)
}

function errorText(error: unknown): string {
  return t(`err.${error instanceof RemoteCommandError ? error.code : 'internal'}`)
}

function onState(next: SurfaceState, info: string | null): void {
  state.value = next
  detail.value = info
}

function onKey(key: KeyId): void {
  const taken = mods.take()
  surface.value?.sendKey(keySequence({ key, ctrl: taken.ctrl, alt: taken.alt }))
}

async function answer(keys: string): Promise<void> {
  const pane = found.value?.pane
  if (!pane) return
  try {
    await hub.command(props.machine, 'ws.keys', { paneKey: pane.key, keys, since: pane.since ?? 0 })
  } catch (error) {
    say(errorText(error))
  }
}

function toggleDirect(): void {
  direct.value = !direct.value
}

function pick(key: string): void {
  sheet.value = false
  if (key !== props.pane) navigate({ name: 'workspace', machine: props.machine, pane: key }, { replace: true })
}

function toWorkspace(): void {
  navigate({ name: 'workspace', machine: props.machine }, { replace: true })
}

watch(
  () => props.pane,
  () => {
    mods.reset()
    direct.value = false
  },
)

onBeforeUnmount(() => {
  clearTimeout(waitTimer)
  if (toastTimer) clearTimeout(toastTimer)
})
</script>

<template>
  <section class="page" data-testid="terminal-view" :data-machine="machine" :data-pane="pane" :data-state="state">
    <template v-if="!known">
      <div class="gap">
        <Link2Off :size="32" :stroke-width="1.8" aria-hidden="true" />
        <h2 data-testid="deeplink-unpaired">{{ t('deeplink.unpaired') }}</h2>
        <button type="button" class="btn btn-primary" @click="navigate({ name: 'home' }, { replace: true })">{{ t('deeplink.toMachines') }}</button>
      </div>
    </template>
    <template v-else-if="gone">
      <div class="gap">
        <StopCircle :size="32" :stroke-width="1.8" aria-hidden="true" />
        <h2 data-testid="deeplink-gone">{{ t('deeplink.gone') }}</h2>
        <button type="button" class="btn btn-primary" @click="toWorkspace">{{ t('deeplink.toWorkspace') }}</button>
      </div>
    </template>
    <template v-else>
      <header class="head">
        <button v-if="!embedded" type="button" class="icon" :aria-label="t('common.back')" data-testid="back" @click="back()">
          <ChevronLeft :size="24" :stroke-width="2" aria-hidden="true" />
        </button>
        <button type="button" class="pick" :disabled="!found" data-testid="pane-pick" @click="sheet = true">
          <span v-if="found" class="dot" :class="`p-${found.pane.provider}`" aria-hidden="true" />
          <span class="pick-text">{{ found ? `${t(`provider.${found.pane.provider}`)} · ${found.tab.name}` : t('term.loading') }}</span>
          <ChevronDown :size="18" :stroke-width="2" aria-hidden="true" />
        </button>
        <StatusChip v-if="found" :rail="found.pane.rail" :lost="!online" />
        <button type="button" class="icon" :aria-label="t('viewer.open')" :disabled="!found?.pane.cwd" data-testid="open-viewer" @click="viewer = true">
          <FolderTree :size="22" :stroke-width="2" aria-hidden="true" />
        </button>
      </header>
      <p v-if="viewOnly" class="banner view-only" role="status" data-testid="view-only"><Eye :size="16" :stroke-width="2.2" aria-hidden="true" />{{ t('term.viewOnly') }}</p>
      <p v-else-if="!online" class="banner offline" role="status" data-testid="term-offline"><Link2Off :size="16" :stroke-width="2.2" aria-hidden="true" />{{ t('term.offline') }}</p>
      <PermissionBar v-if="found?.pane.asked" :text="found.pane.waitText" :enabled="online && !viewOnly" @keys="answer" />
      <div class="stage">
        <TerminalSurface v-if="found?.pane.canAttach" ref="surface" :key="found.pane.sessionId ?? ''" :machine="machine" :session-id="found.pane.sessionId" :mods="mods" :direct="direct" @state="onState" />
        <p v-if="overlay" class="overlay" role="status" data-testid="term-overlay"><component :is="overlay.icon" :size="18" :stroke-width="2.2" aria-hidden="true" />{{ overlay.text }}</p>
        <p v-if="toast" class="toast" role="status" data-testid="term-toast">{{ toast }}</p>
      </div>
      <KeyBar :ctrl="mods.ctrl" :alt="mods.alt" :disabled="!canType" @key="onKey" @ctrl="mods.toggleCtrl()" @alt="mods.toggleAlt()" />
      <InputBar :enabled="canType" :direct="direct" @direct="toggleDirect" @send="(text) => surface?.sendLine(text)" />
      <PaneSwitchSheet v-if="sheet && found" :title="t('term.panesIn', { tab: found.tab.name })" :panes="found.tab.panes" :current="pane" @pick="pick" @close="sheet = false" />
      <ViewerPage v-if="viewer && found?.pane.cwd" :machine="machine" :root="found.pane.cwd" :title="found.pane.title" @close="viewer = false" />
    </template>
  </section>
</template>

<style scoped>
.page {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 56px;
  padding: 0 8px 0 4px;
  flex: none;
}

.icon {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
  flex: none;
}

.icon:disabled {
  color: var(--ccm-ink-faint);
}

.pick {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  min-height: 44px;
  padding: 0 14px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  font-weight: 600;
}

.pick-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.dot {
  width: 9px;
  height: 9px;
  flex: none;
  border-radius: 50%;
  background: var(--ccm-agent-claude);
}

.dot.p-codex {
  background: var(--ccm-agent-codex);
}

.dot.p-shell {
  background: var(--ccm-agent-shell);
}

.banner {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  padding: 8px 16px;
  font-size: 0.875rem;
  font-weight: 600;
  flex: none;
}

.view-only {
  color: var(--ccm-waiting);
  background: color-mix(in srgb, var(--ccm-waiting) 10%, var(--ccm-surface));
}

.offline {
  color: var(--ccm-ink-muted);
  background: var(--ccm-raised);
}

.stage {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  background: var(--ccm-term-bg);
}

.overlay,
.toast {
  position: absolute;
  top: 12px;
  left: 12px;
  right: 12px;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  padding: 8px 12px;
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.78);
  color: #ffffff;
}

.toast {
  top: auto;
  bottom: 12px;
}

.gap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 64px 24px;
  text-align: center;
  color: var(--ccm-ink-muted);
}

.gap h2 {
  margin: 0;
  font-size: 1.125rem;
  color: var(--ccm-ink);
}
</style>
