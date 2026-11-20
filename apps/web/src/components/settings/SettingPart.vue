<script setup lang="ts">
defineProps<{ id: string; title: string }>()
</script>

<template>
  <div class="mt-8 first:mt-0" :data-settings-part="id">
    <h3 :id="`part-${id}`" class="mb-3 border-b border-line pb-1.5 text-sm font-semibold text-ink">{{ title }}</h3>
    <slot />
  </div>
</template>
