<script setup lang="ts">
defineProps<{ title: string; testid?: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <div class="scrim" :data-testid="testid" @click.self="emit('close')">
    <div class="sheet" role="dialog" aria-modal="true" :aria-label="title">
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
  background: rgba(0, 0, 0, 0.45);
  animation: fade 160ms var(--ease-out);
}

.sheet {
  width: min(100%, 520px);
  max-height: 80%;
  overflow-y: auto;
  padding: 8px 16px calc(16px + env(safe-area-inset-bottom));
  border-radius: 20px 20px 0 0;
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
  animation: rise 260ms var(--ease-out);
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
