<script setup lang="ts">
import { Check, Clock, TriangleAlert } from 'lucide-vue-next'
import type { Notice } from '../lib/useAction'

defineProps<{ notice: Notice | null }>()
</script>

<template>
  <Transition name="note">
    <p v-if="notice" class="note" :class="notice.kind" role="status" data-testid="action-note" :data-kind="notice.kind">
      <Check v-if="notice.kind === 'ok'" :size="16" :stroke-width="2.4" aria-hidden="true" />
      <Clock v-else-if="notice.kind === 'wait'" :size="16" :stroke-width="2.4" aria-hidden="true" />
      <TriangleAlert v-else :size="16" :stroke-width="2.4" aria-hidden="true" />
      <span>{{ notice.text }}</span>
    </p>
  </Transition>
</template>

<style scoped>
.note {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 16px 8px;
  padding: 8px 12px;
  border-radius: var(--radius-control);
  background: var(--ccm-surface);
  border: 1px solid var(--ccm-line);
  font-size: 0.875rem;
  font-weight: 600;
  flex: none;
}

.note.ok {
  color: var(--ccm-done);
}

.note.wait {
  color: var(--ccm-waiting);
}

.note.error {
  color: var(--ccm-error);
}

.note-enter-active,
.note-leave-active {
  transition: opacity 180ms var(--ease-out), transform 180ms var(--ease-out);
}

.note-enter-from,
.note-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
