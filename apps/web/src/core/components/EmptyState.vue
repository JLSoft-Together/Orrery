<script setup lang="ts">
import type { Component } from 'vue'

withDefaults(defineProps<{ title: string; hint?: string; icon?: Component; compact?: boolean }>(), { compact: false })
</script>

<template>
  <div class="flex min-w-0 max-w-sm flex-col items-center text-center" :class="compact ? 'gap-1.5 px-3 py-4' : 'gap-2 px-4 py-6'" data-empty-state>
    <component :is="icon" v-if="icon" :size="compact ? 20 : 26" class="text-ink-faint" aria-hidden="true" />
    <p class="text-sm font-medium text-ink" :class="compact ? 'text-xs' : ''" data-empty-title>{{ title }}</p>
    <p v-if="hint" class="text-xs text-ink-muted" data-empty-hint>{{ hint }}</p>
    <div v-if="$slots.actions" class="mt-1 flex flex-wrap items-center justify-center gap-2" data-empty-actions>
      <slot name="actions" />
    </div>
  </div>
</template>
