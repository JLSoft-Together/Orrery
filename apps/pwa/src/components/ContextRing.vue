<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ pct: number; warn?: boolean; flag?: boolean; label: string }>()
const emit = defineEmits<{ open: [] }>()
const C = 2 * Math.PI * 12
const dash = computed(() => `${((C * Math.max(0, Math.min(100, props.pct))) / 100).toFixed(2)} ${C.toFixed(2)}`)
</script>

<template>
  <button type="button" class="ctx" :class="{ warn: warn || pct >= 80 }" :aria-label="label" data-testid="chat-ctx" :data-pct="pct" @click="emit('open')">
    <svg class="ring" viewBox="0 0 28 28" aria-hidden="true">
      <circle class="tr" cx="14" cy="14" r="12" />
      <circle class="fg" cx="14" cy="14" r="12" :stroke-dasharray="dash" />
    </svg>
    <span class="pct" aria-hidden="true">{{ pct }}</span>
    <span v-if="flag" class="flag" aria-hidden="true" />
  </button>
</template>

<style scoped>
.ctx {
  position: relative;
  display: inline-grid;
  place-items: center;
  flex: none;
  width: 44px;
  height: 44px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--ccm-ink-muted);
  font-variant-numeric: tabular-nums;
}

.ctx > * {
  grid-area: 1 / 1;
}

.ring {
  width: 30px;
  height: 30px;
  transform: rotate(-90deg);
}

.ring circle {
  fill: none;
  stroke-width: 2.6;
}

.tr {
  stroke: var(--ccm-line-strong);
}

.fg {
  stroke: var(--ccm-ink-muted);
  stroke-linecap: round;
}

.pct {
  font-size: 10.5px;
  font-weight: 700;
  color: var(--ccm-ink);
  letter-spacing: -0.02em;
}

.warn .fg {
  stroke: var(--ccm-warning);
}

.warn .pct {
  color: var(--ccm-warning);
}

.flag {
  place-self: start end;
  width: 9px;
  height: 9px;
  margin: 6px 6px 0 0;
  border-radius: 50%;
  background: var(--ccm-warning);
  box-shadow: 0 0 0 2px var(--ccm-canvas);
}
</style>
