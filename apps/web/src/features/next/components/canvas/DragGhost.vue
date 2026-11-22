<script setup lang="ts">
import { computed } from 'vue'
import Chip from '../ui/Chip.vue'

const props = defineProps<{
  x: number
  y: number
  agent: string
  agentTone: string
  title: string
  status?: string
  statusLabel?: string
  hint?: string
}>()

const style = computed(() => {
  const vw = typeof window === 'undefined' ? 0 : window.innerWidth
  const flip = props.x > vw / 2
  return { transform: `translate(${flip ? props.x - 14 : props.x + 14}px, ${props.y + 14}px)${flip ? ' translateX(-100%)' : ''}` }
})
</script>

<template>
  <Teleport to="body">
    <div class="mist-drag-ghost pointer-events-none fixed top-0 left-0 z-50" :style="style" aria-hidden="true" data-drag-ghost data-next-ghost>
      <span class="mist-drag-agent" :data-agent="agentTone">{{ agent }}</span>
      <span class="min-w-0 truncate">{{ title }}</span>
      <Chip v-if="status" :status="status" :label="statusLabel" />
      <span v-if="hint" class="mist-drag-hint">{{ hint }}</span>
    </div>
  </Teleport>
</template>
