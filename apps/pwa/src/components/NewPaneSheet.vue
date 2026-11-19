<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RemoteCommandError } from '../remote/types'
import { useHubStore } from '../stores/hub'
import AgentTile from './AgentTile.vue'
import BottomSheet from './BottomSheet.vue'

const props = defineProps<{ machine: string; cwd: string }>()
const emit = defineEmits<{ close: []; opened: [paneKey: string | null] }>()
const { t } = useI18n()
const hub = useHubStore()
const busy = ref(false)
const fail = ref<string | null>(null)

async function openPane(preset: 'claude' | 'codex' | 'shell'): Promise<void> {
  busy.value = true
  fail.value = null
  try {
    const reply = await hub.command(props.machine, 'ws.open', { cwd: props.cwd, preset })
    emit('opened', typeof reply.paneKey === 'string' ? reply.paneKey : null)
    emit('close')
  } catch (error) {
    fail.value = t(`err.${error instanceof RemoteCommandError ? error.code : 'internal'}`)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <BottomSheet :title="t('workspace.newPane')" testid="new-sheet" @close="emit('close')">
    <div class="presets">
      <button v-for="p in (['claude', 'codex', 'shell'] as const)" :key="p" type="button" class="preset" :disabled="busy" :data-testid="`new-${p}`" @click="openPane(p)">
        <AgentTile :provider="p" :size="36" /><span>{{ t(`provider.${p}`) }}</span>
      </button>
    </div>
    <p v-if="fail" class="error" role="alert">{{ fail }}</p>
  </BottomSheet>
</template>

<style scoped>
.presets {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.preset {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  padding: 10px 12px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-control);
  font-weight: 600;
  text-align: left;
}

.error {
  margin: 8px 0 0;
  color: var(--ccm-error);
  font-weight: 600;
}
</style>
