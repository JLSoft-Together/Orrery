<script setup lang="ts">
import { Asterisk, Hexagon, SquareTerminal } from 'lucide-vue-next'
import { computed } from 'vue'
import type { Provider } from '../lib/monitorModel'

const props = withDefaults(defineProps<{ provider: Provider; size?: number }>(), { size: 36 })
const icon = computed(() => (props.provider === 'codex' ? Hexagon : props.provider === 'shell' ? SquareTerminal : Asterisk))
</script>

<template>
  <span class="tile" :class="`p-${provider}`" :data-provider="provider" aria-hidden="true" :style="{ width: `${size}px`, height: `${size}px` }">
    <component :is="icon" :size="Math.round(size * 0.5)" :stroke-width="2" />
  </span>
</template>

<style scoped>
.tile {
  display: grid;
  place-items: center;
  flex: none;
  border-radius: var(--radius-control);
}

.p-claude {
  color: var(--ccm-agent-claude);
  background: color-mix(in srgb, var(--ccm-agent-claude) 14%, var(--ccm-surface));
}

.p-codex {
  color: var(--ccm-agent-codex);
  background: color-mix(in srgb, var(--ccm-agent-codex) 14%, var(--ccm-surface));
}

.p-shell {
  color: var(--ccm-agent-shell);
  background: color-mix(in srgb, var(--ccm-agent-shell) 14%, var(--ccm-surface));
}
</style>
