<script setup lang="ts">
import { ChevronLeft } from 'lucide-vue-next'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import ViewerChanges from '../components/ViewerChanges.vue'
import ViewerFiles from '../components/ViewerFiles.vue'
import ViewerLog from '../components/ViewerLog.vue'
import ViewerRun from '../components/ViewerRun.vue'
import type { JsonMap } from '../remote/types'
import { useHubStore } from '../stores/hub'

type Segment = 'files' | 'diff' | 'git' | 'run'

const props = defineProps<{ machine: string; root: string; title: string }>()
const emit = defineEmits<{ close: [] }>()
const { t } = useI18n()
const hub = useHubStore()
const segment = ref<Segment>('files')
const refresh = ref(0)
const segments: Segment[] = ['files', 'diff', 'git', 'run']
let off: (() => void) | null = null

const command = (type: string, fields?: JsonMap, timeoutMs?: number): Promise<JsonMap> => hub.command(props.machine, type, fields, timeoutMs)
const subscribe = (handler: (event: JsonMap) => void): (() => void) => hub.events(props.machine, handler)

onMounted(() => {
  off = subscribe((event) => {
    if (event.t === 'git.changed') refresh.value++
  })
  void command('git.watch', { root: props.root }).catch(() => null)
})

onBeforeUnmount(() => {
  off?.()
  void command('git.unwatch', { root: props.root }).catch(() => null)
})
</script>

<template>
  <section class="viewer" data-testid="viewer" :data-segment="segment">
    <header class="head">
      <button type="button" class="icon" :aria-label="t('common.back')" data-testid="viewer-close" @click="emit('close')"><ChevronLeft :size="24" :stroke-width="2" aria-hidden="true" /></button>
      <h2 class="title">{{ title }}</h2>
    </header>
    <div class="seg" role="tablist">
      <button v-for="s in segments" :key="s" type="button" role="tab" class="seg-btn" :class="{ on: segment === s }" :aria-selected="segment === s" :data-testid="`viewer-seg-${s}`" @click="segment = s">{{ t(`viewer.seg.${s}`) }}</button>
    </div>
    <ViewerFiles v-if="segment === 'files'" :command="command" :root="root" />
    <ViewerChanges v-else-if="segment === 'diff'" :command="command" :root="root" :refresh="refresh" />
    <ViewerLog v-else-if="segment === 'git'" :command="command" :root="root" :refresh="refresh" />
    <ViewerRun v-else :command="command" :root="root" :refresh="refresh" :subscribe="subscribe" />
  </section>
</template>

<style scoped>
.viewer {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  background: var(--ccm-canvas);
  animation: enter 260ms var(--ease-out);
}

.head {
  display: flex;
  align-items: center;
  gap: 4px;
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
}

.title {
  flex: 1;
  min-width: 0;
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 1.0625rem;
}

.seg {
  display: flex;
  gap: 4px;
  margin: 0 16px 8px;
  padding: 3px;
  border-radius: var(--radius-control);
  background: var(--ccm-raised);
  flex: none;
}

.seg-btn {
  flex: 1;
  min-height: 40px;
  border: 0;
  border-radius: calc(var(--radius-control) - 2px);
  background: transparent;
  color: var(--ccm-ink-muted);
  font-weight: 600;
}

.seg-btn.on {
  background: var(--ccm-surface);
  color: var(--ccm-ink);
  box-shadow: 0 0 0 1px var(--ccm-line);
}

@keyframes enter {
  from {
    opacity: 0;
    transform: translateX(24px);
  }
}
</style>
