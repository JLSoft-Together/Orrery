<script setup lang="ts">
import { Check, Laptop, Pencil, Trash2 } from 'lucide-vue-next'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { askConfirm } from '../lib/confirm'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'

const { t } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()
const editing = ref<string | null>(null)
const draft = ref('')

function startEdit(id: string, current: string): void {
  editing.value = id
  draft.value = current
}

async function save(id: string): Promise<void> {
  await hub.rename(id, draft.value)
  editing.value = null
}

async function forget(id: string, name: string): Promise<void> {
  const ok = await askConfirm({ title: t('settings.forgetTitle', { name }), body: t('settings.forgetBody'), ok: t('machines.forget'), cancel: t('common.cancel'), danger: true })
  if (!ok) return
  await hub.forget(id)
  await prefs.dropMachine(id)
}
</script>

<template>
  <div class="card list" data-testid="settings-machines">
    <p v-if="hub.machines.length === 0" class="none">{{ t('settings.noMachines') }}</p>
    <div v-for="m in hub.machines" :key="m.id" class="row" :data-testid="`settings-machine-${m.id}`">
      <span class="avatar" aria-hidden="true"><Laptop :size="18" :stroke-width="2" /></span>
      <template v-if="editing === m.id">
        <input v-model="draft" class="input" maxlength="60" :aria-label="t('settings.rename')" data-testid="rename-input" @keydown.enter="save(m.id)" />
        <button type="button" class="icon" :aria-label="t('settings.save')" data-testid="rename-save" @click="save(m.id)"><Check :size="20" :stroke-width="2.2" aria-hidden="true" /></button>
      </template>
      <template v-else>
        <span class="name" data-testid="settings-machine-name">{{ m.alias ?? m.name }}</span>
        <button type="button" class="icon" :aria-label="t('settings.rename')" data-testid="rename-start" @click="startEdit(m.id, m.alias ?? m.name)"><Pencil :size="18" :stroke-width="2" aria-hidden="true" /></button>
        <button type="button" class="icon danger" :aria-label="t('machines.forget')" data-testid="machine-forget" @click="forget(m.id, m.alias ?? m.name)"><Trash2 :size="18" :stroke-width="2" aria-hidden="true" /></button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.list {
  overflow: hidden;
}

.none {
  margin: 0;
  padding: 14px;
  color: var(--ccm-ink-muted);
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 56px;
  padding: 6px 8px 6px 14px;
  border-top: 1px solid var(--ccm-line);
}

.row:first-child {
  border-top: 0;
}

.avatar {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  margin-right: 4px;
  border-radius: var(--radius-control);
  background: var(--ccm-raised);
  color: var(--ccm-ink-muted);
  flex: none;
}

.name {
  flex: 1;
  min-width: 0;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input {
  flex: 1;
  min-width: 0;
  min-height: 44px;
  padding: 0 10px;
  border-radius: var(--radius-control);
  border: 1px solid var(--ccm-line-strong);
  background: var(--ccm-control);
}

.icon {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
  flex: none;
}

.icon.danger {
  color: var(--ccm-error);
}
</style>
