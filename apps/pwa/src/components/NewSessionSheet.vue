<script setup lang="ts">
import { Folder } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RemoteCommandError } from '../remote/types'
import { useHubStore } from '../stores/hub'
import { baseName } from '../lib/workspaceModel'
import AgentGlyph from './AgentGlyph.vue'
import BottomSheet from './BottomSheet.vue'

type Preset = 'claude' | 'codex' | 'shell'

const props = defineProps<{ machine: string; places: string[] }>()
const emit = defineEmits<{ close: []; opened: [paneKey: string | null, preset: Preset] }>()
const { t } = useI18n()
const hub = useHubStore()
const busy = ref(false)
const fail = ref<string | null>(null)
const place = ref<string | null>(props.places[0] ?? null)
const options = computed(() => props.places.slice(0, 8))

async function open(preset: Preset): Promise<void> {
  if (!place.value) return
  busy.value = true
  fail.value = null
  try {
    const reply = await hub.command(props.machine, 'ws.open', { cwd: place.value, preset })
    emit('opened', typeof reply.paneKey === 'string' ? reply.paneKey : null, preset)
  } catch (error) {
    fail.value = t(`err.${error instanceof RemoteCommandError ? error.code : 'internal'}`)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <BottomSheet :title="t('chat.newSession')" testid="new-session-sheet" @close="emit('close')">
    <div class="opts">
      <button v-for="p in (['claude', 'codex', 'shell'] as const)" :key="p" type="button" class="opt" :disabled="busy || !place" :data-testid="`new-session-${p}`" @click="open(p)">
        <AgentGlyph :provider="p" avatar />
        <span>{{ t(`provider.${p}`) }}</span>
      </button>
    </div>
    <div v-if="options.length" class="where" role="radiogroup" :aria-label="t('chat.where')">
      <span class="where-label">{{ t('chat.where') }}</span>
      <button v-for="cwd in options" :key="cwd" type="button" class="pchip" role="radio" :aria-checked="place === cwd" :title="cwd" :data-testid="`new-place-${baseName(cwd)}`" @click="place = cwd">
        <Folder :size="16" :stroke-width="2" aria-hidden="true" />{{ baseName(cwd) }}
      </button>
    </div>
    <p v-if="fail" class="error" role="alert">{{ fail }}</p>
  </BottomSheet>
</template>

<style scoped>
.opts {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.opt {
  display: flex;
  align-items: center;
  gap: 14px;
  min-height: 60px;
  padding: 8px 12px;
  border: 0;
  border-radius: 22px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
  color: var(--ccm-ink);
  font-size: 1rem;
  font-weight: 600;
  text-align: left;
}

.opt:disabled {
  opacity: 0.5;
}

.where {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 14px;
}

.where-label {
  margin-right: 4px;
  font-size: 0.875rem;
  color: var(--ccm-ink-muted);
}

.pchip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 40px;
  max-width: 100%;
  padding: 0 14px;
  border: 0;
  border-radius: 20px;
  background: var(--ccm-raised);
  box-shadow: inset 0 0 0 1px var(--ccm-line);
  color: var(--ccm-ink);
  font-size: 0.875rem;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pchip[aria-checked='true'] {
  background: var(--ccm-ink);
  color: var(--ccm-canvas);
  box-shadow: none;
}

.error {
  margin: 10px 0 0;
  color: var(--ccm-error);
  font-weight: 600;
}
</style>
