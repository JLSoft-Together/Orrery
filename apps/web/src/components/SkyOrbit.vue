<script setup lang="ts">
defineProps<{ id: string; data: { rx: number; ry: number } }>()
</script>

<template>
  <svg class="pointer-events-none block" :width="data.rx * 2" :height="data.ry * 2" aria-hidden="true" data-sky-orbit>
    <ellipse :cx="data.rx" :cy="data.ry" :rx="data.rx - 1" :ry="data.ry - 1" fill="none" stroke="var(--color-line-strong)" stroke-width="1" stroke-dasharray="4 5" />
  </svg>
</template>
