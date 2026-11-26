<script setup lang="ts">
withDefaults(defineProps<{ active?: boolean; label?: string }>(), { active: false, label: undefined })
</script>

<template>
  <header
    class="flex h-(--ccm-density-head-h) min-w-0 shrink-0 items-center gap-2 pr-2 pl-2.5 text-xs"
    :class="active ? 'bg-(--ccm-pane-head-tint)' : ''"
    :role="label ? 'group' : undefined"
    :aria-label="label"
    data-next-pane-head
    :data-active="active || undefined"
  >
    <div class="flex min-w-0 shrink items-center gap-2">
      <slot name="left" />
    </div>
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <slot />
    </div>
    <div class="flex shrink-0 items-center gap-0.5">
      <slot name="right" />
    </div>
  </header>
</template>
