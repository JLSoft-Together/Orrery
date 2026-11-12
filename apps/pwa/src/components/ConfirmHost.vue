<script setup lang="ts">
import { answerConfirm, pendingConfirm } from '../lib/confirm'
</script>

<template>
  <div v-if="pendingConfirm" class="scrim" data-testid="confirm" @click.self="answerConfirm(false)">
    <div class="sheet" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
      <h2 id="confirm-title" class="title">{{ pendingConfirm.title }}</h2>
      <p v-if="pendingConfirm.body" class="body">{{ pendingConfirm.body }}</p>
      <div class="buttons">
        <button type="button" class="btn" data-testid="confirm-cancel" @click="answerConfirm(false)">{{ pendingConfirm.cancel }}</button>
        <button type="button" class="btn" :class="pendingConfirm.danger ? 'btn-danger' : 'btn-primary'" data-testid="confirm-ok" @click="answerConfirm(true)">{{ pendingConfirm.ok }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  align-items: end;
  justify-items: center;
  padding: 16px 16px calc(16px + env(safe-area-inset-bottom));
  background: rgba(0, 0, 0, 0.45);
  animation: fade 160ms var(--ease-out);
}

.sheet {
  width: min(100%, 420px);
  padding: 20px;
  border-radius: var(--radius-card);
  border: 1px solid var(--ccm-line);
  background: var(--ccm-surface);
  animation: rise 260ms var(--ease-out);
}

.title {
  margin: 0 0 8px;
  font-size: 1.0625rem;
}

.body {
  margin: 0 0 16px;
  color: var(--ccm-ink-muted);
}

.buttons {
  display: flex;
  gap: 8px;
}

.buttons .btn {
  flex: 1;
}

@keyframes fade {
  from {
    opacity: 0;
  }
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(24px);
  }
}
</style>
