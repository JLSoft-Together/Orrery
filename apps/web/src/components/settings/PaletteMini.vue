<script setup lang="ts">
import type { NextPreview } from '../../next/palettes'

defineProps<{ preview: NextPreview; stars?: boolean }>()
</script>

<template>
  <div
    class="relative h-[84px] w-full overflow-hidden rounded-inner"
    :style="{ background: `radial-gradient(60% 90% at 15% 0%, ${preview.glowA}, transparent 70%), radial-gradient(60% 90% at 95% 100%, ${preview.glowB}, transparent 70%), ${preview.canvas}` }"
    data-palette-mini
    aria-hidden="true"
  >
    <div class="absolute inset-y-1.5 left-1.5 w-[22%] rounded-[5px]" :style="{ background: preview.glass, outline: `1px solid ${preview.glass}` }">
      <span class="absolute top-1.5 left-1.5 h-[3px] w-[60%] rounded-full" :style="{ background: preview.ink, opacity: 0.55 }" />
      <span class="absolute top-3.5 left-1.5 h-[3px] w-[45%] rounded-full" :style="{ background: preview.ink, opacity: 0.3 }" />
      <span class="absolute top-[22px] left-1.5 h-[3px] w-[52%] rounded-full" :style="{ background: preview.waiting }" />
      <span class="absolute top-[30px] left-1.5 h-[3px] w-[38%] rounded-full" :style="{ background: preview.accent }" />
    </div>
    <div class="absolute inset-y-1.5 left-[27%] w-[33%] rounded-[5px]" :style="{ background: preview.well }">
      <span class="absolute top-1.5 left-1.5 h-[3px] w-[55%] rounded-full" :style="{ background: preview.accent }" />
      <span class="absolute top-3.5 left-1.5 h-[3px] w-[75%] rounded-full" :style="{ background: preview.ink, opacity: 0.4 }" />
      <span class="absolute top-[22px] left-1.5 h-[3px] w-[40%] rounded-full" :style="{ background: preview.ink, opacity: 0.25 }" />
    </div>
    <div class="absolute inset-y-1.5 right-1.5 left-[62%] rounded-[5px]" :style="{ background: preview.glass, outline: `1px solid ${preview.glass}` }">
      <span class="absolute top-1.5 left-1.5 h-[4px] w-[30%] rounded-full" :style="{ background: preview.accent }" />
      <span class="absolute top-4 left-1.5 h-[3px] w-[70%] rounded-full" :style="{ background: preview.ink, opacity: 0.4 }" />
      <span class="absolute top-[26px] left-1.5 h-[3px] w-[55%] rounded-full" :style="{ background: preview.ink, opacity: 0.25 }" />
    </div>
    <template v-if="stars">
      <span class="absolute top-2 left-[40%] size-[2px] rounded-full" :style="{ background: preview.ink }" />
      <span class="absolute right-[10%] bottom-3 size-[2px] rounded-full" :style="{ background: preview.ink }" />
      <span class="absolute top-[60%] left-[8%] size-[2px] rounded-full" :style="{ background: preview.ink }" />
    </template>
  </div>
</template>
