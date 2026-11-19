<script setup lang="ts">
import { FolderInput, Minimize2, Pencil, Undo2, X } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { rememberClose } from '../lib/closeUndo'
import { askConfirm } from '../lib/confirm'
import { canCompact, closePane, compactPane, errorCode, moveTargets, movePane, renameTab, reopenPane, TAB_NAME_MAX } from '../lib/paneActions'
import type { WsPane, WsTab } from '../lib/workspaceModel'
import { useHubStore } from '../stores/hub'
import BottomSheet from './BottomSheet.vue'

const props = defineProps<{ machine: string; tab: WsTab; tabs: WsTab[]; pane: WsPane; closed: number }>()
const emit = defineEmits<{ close: []; closedPane: [] }>()
const { t } = useI18n()
const hub = useHubStore()
const mode = ref<'menu' | 'rename' | 'move'>('menu')
const name = ref(props.tab.customName ?? '')
const busy = ref(false)
const fail = ref<string | null>(null)

const others = computed(() => moveTargets(props.tabs, props.tab.key))
const title = computed(() => (mode.value === 'rename' ? t('paneActions.rename') : mode.value === 'move' ? t('paneActions.move') : props.pane.title))
const run = (type: string, fields?: Record<string, unknown>) => hub.command(props.machine, type, fields as never)

async function act(task: () => Promise<void>): Promise<void> {
  if (busy.value) return
  busy.value = true
  fail.value = null
  try {
    await task()
  } catch (error) {
    fail.value = t(`err.${errorCode(error)}`)
  } finally {
    busy.value = false
  }
}

function saveName(): Promise<void> {
  return act(async () => {
    await renameTab(run, props.tab.key, name.value)
    emit('close')
  })
}

function moveTo(key: string): Promise<void> {
  return act(async () => {
    await movePane(run, props.pane.key, key)
    emit('close')
  })
}

function reopen(): Promise<void> {
  return act(async () => {
    await reopenPane(run)
    emit('close')
  })
}

function compact(): Promise<void> {
  return act(async () => {
    const ok = await askConfirm({ title: t('paneActions.compactTitle'), body: t('paneActions.compactBody'), ok: t('paneActions.compact'), cancel: t('common.cancel') })
    if (!ok) return
    await compactPane(run, props.pane.key)
    emit('close')
  })
}

function close(): Promise<void> {
  return act(async () => {
    const ok = await askConfirm({ title: t('paneActions.closeTitle', { name: props.pane.title }), body: t('paneActions.closeBody'), ok: t('paneActions.close'), cancel: t('common.cancel'), danger: true })
    if (!ok) return
    const result = await closePane(run, props.pane.key)
    if (result.closed) emit('closedPane')
    else if (result.confirm) rememberClose(props.machine, result.confirm)
    emit('close')
  })
}
</script>

<template>
  <BottomSheet :title="title" testid="pane-actions" @close="emit('close')">
    <ul v-if="mode === 'menu'" class="list">
      <li>
        <button type="button" class="row" :disabled="busy" data-testid="act-rename" @click="((mode = 'rename'), (fail = null))"><Pencil :size="20" :stroke-width="2" aria-hidden="true" />{{ t('paneActions.rename') }}</button>
      </li>
      <li>
        <button type="button" class="row" :disabled="busy || others.length === 0" data-testid="act-move" @click="((mode = 'move'), (fail = null))"><FolderInput :size="20" :stroke-width="2" aria-hidden="true" />{{ t('paneActions.move') }}</button>
      </li>
      <li v-if="canCompact(pane)">
        <button type="button" class="row" :disabled="busy" data-testid="act-compact" @click="compact"><Minimize2 :size="20" :stroke-width="2" aria-hidden="true" />{{ t('paneActions.compact') }}</button>
      </li>
      <li>
        <button type="button" class="row" :disabled="busy || closed === 0" data-testid="act-reopen" @click="reopen"><Undo2 :size="20" :stroke-width="2" aria-hidden="true" />{{ t('paneActions.reopen', { n: closed }) }}</button>
      </li>
      <li>
        <button type="button" class="row danger" :disabled="busy" data-testid="act-close" @click="close"><X :size="20" :stroke-width="2.2" aria-hidden="true" />{{ t('paneActions.close') }}</button>
      </li>
    </ul>

    <form v-else-if="mode === 'rename'" class="form" @submit.prevent="saveName">
      <input v-model="name" class="field" :maxlength="TAB_NAME_MAX" :placeholder="t('paneActions.renameHint')" autocomplete="off" data-testid="rename-input" />
      <button type="submit" class="btn btn-primary" :disabled="busy" data-testid="rename-save">{{ t('paneActions.save') }}</button>
    </form>

    <ul v-else class="list">
      <li v-for="x in others" :key="x.key">
        <button type="button" class="row" :disabled="busy" :data-testid="`move-${x.key}`" @click="moveTo(x.key)"><FolderInput :size="20" :stroke-width="2" aria-hidden="true" />{{ x.name }}</button>
      </li>
    </ul>

    <p v-if="fail" class="error" role="alert" data-testid="act-error">{{ fail }}</p>
  </BottomSheet>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 52px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--radius-control);
  background: transparent;
  color: var(--ccm-ink);
  font-weight: 600;
  text-align: left;
}

.row:disabled {
  color: var(--ccm-ink-faint);
}

.row.danger {
  color: var(--ccm-error);
}

.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field {
  width: 100%;
  min-height: 48px;
  padding: 0 14px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
  color: var(--ccm-ink);
  font-size: 1rem;
}

.error {
  margin: 8px 0 0;
  color: var(--ccm-error);
  font-weight: 600;
}
</style>
