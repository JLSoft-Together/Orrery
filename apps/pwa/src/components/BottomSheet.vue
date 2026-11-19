<script setup lang="ts">
defineProps<{ title: string; testid?: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <div class="scrim" :data-testid="testid" @click.self="emit('close')">
    <div class="sheet glass-sheet" role="dialog" aria-modal="true" :aria-label="title">
      <span class="grab" aria-hidden="true" />
      <h2 class="title">{{ title }}</h2>
      <slot />
    </div>
  </div>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  align-items: end;
  justify-items: center;
  background: var(--ccm-scrim);
  animation: fade var(--ccm-dur-fast) var(--ease-out);
}

.sheet {
  width: min(calc(100% - 16px), 520px);
  max-height: 80%;
  margin-bottom: calc(8px + env(safe-area-inset-bottom));
  overflow-y: auto;
  padding: 8px 16px 20px;
  border-radius: 28px;
  animation: rise var(--ccm-dur-slow) var(--ease-out);
}

.grab {
  display: block;
  width: 36px;
  height: 4px;
  margin: 0 auto 12px;
  border-radius: 2px;
  background: var(--ccm-line-strong);
}

.title {
  margin: 0 0 12px;
  font-size: 1.0625rem;
}

@keyframes fade {
  from {
    opacity: 0;
  }
}

@keyframes rise {
  from {
    transform: translateY(32px);
    opacity: 0;
  }
}
</style>
