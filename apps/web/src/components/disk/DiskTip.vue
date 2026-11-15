<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ x: number; y: number; w: number; h: number; title: string; lines: string[] }>()

const style = computed(() => {
  const flip = props.x > props.w - 220
  const down = props.y < props.h - 90
  return {
    left: `${flip ? props.x - 12 : props.x + 14}px`,
    top: `${down ? props.y + 14 : props.y - 12}px`,
    transform: `translate(${flip ? '-100%' : '0'}, ${down ? '0' : '-100%'})`,
  }
})
</script>

<template>
  <div class="pointer-events-none absolute z-10 max-w-[18rem] rounded-inner bg-raised px-2.5 py-1.5 text-xs shadow-lg ring-1 ring-line" :style="style" role="tooltip" data-disk-tip>
    <p class="truncate font-medium">{{ title }}</p>
    <p v-for="(l, i) in lines" :key="i" class="truncate text-2xs text-ink-muted">{{ l }}</p>
  </div>
</template>
