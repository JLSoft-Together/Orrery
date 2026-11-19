<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'

const props = defineProps<{ text: string; action: string; ms?: number }>()
const emit = defineEmits<{ undo: []; dismiss: [] }>()
let timer: ReturnType<typeof setTimeout> | null = null

onMounted(() => {
  timer = setTimeout(() => emit('dismiss'), props.ms ?? 8000)
})
onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <div class="bar" role="status" data-testid="undo-bar">
    <span class="text">{{ text }}</span>
    <button type="button" class="act" data-testid="undo-go" @click="emit('undo')">{{ action }}</button>
  </div>
</template>

<style scoped>
.bar {
  position: fixed;
  left: 16px;
  right: 16px;
  bottom: calc(140px + env(safe-area-inset-bottom));
  z-index: 6;
  display: flex;
  align-items: center;
  gap: 12px;
  max-width: 480px;
  margin: 0 auto;
  padding: 8px 8px 8px 16px;
  border-radius: 14px;
  background: rgba(0, 0, 0, 0.82);
  color: #ffffff;
}

.text {
  flex: 1;
  min-width: 0;
}

.act {
  min-height: 44px;
  padding: 0 16px;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: #ffffff;
  font-weight: 700;
  text-decoration: underline;
}
</style>
