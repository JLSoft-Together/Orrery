<script setup lang="ts">
import { computed } from 'vue'
import type { SessionGroupKey } from '../../lib/sessionGroups'

const props = defineProps<{ group: SessionGroupKey; subs: number }>()
const SUN = { needs: 'var(--color-warning)', running: 'var(--color-accent)', idle: 'var(--color-ink-faint)', ended: 'var(--color-line-strong)' } as const
const sun = computed(() => SUN[props.group])
</script>

<template>
  <svg width="28" height="28" viewBox="0 0 28 28" class="shrink-0" aria-hidden="true" data-orbit-mini>
    <circle cx="14" cy="14" r="12.5" fill="none" stroke="var(--color-line-strong)" stroke-width="1" stroke-dasharray="2 3" />
    <circle cx="14" cy="14" r="5" :fill="sun" data-orbit-sun />
    <circle v-if="subs > 0" cx="23.5" cy="8" r="3" fill="var(--color-accent)" data-orbit-planet />
  </svg>
</template>
