<script setup lang="ts">
import { ChevronLeft } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { back } from '../router'

defineProps<{ title: string; sub?: string; backless?: boolean }>()
const { t } = useI18n()
</script>

<template>
  <header class="bar glass-bar">
    <div class="row">
      <button v-if="!backless" type="button" class="back" :aria-label="t('common.back')" data-testid="back" @click="back()">
        <ChevronLeft :size="24" :stroke-width="2" aria-hidden="true" />
      </button>
      <div class="heading">
        <h1 class="title" data-testid="page-title">{{ title }}</h1>
        <p v-if="sub" class="sub" data-testid="page-sub">{{ sub }}</p>
      </div>
      <div class="right"><slot name="right" /></div>
    </div>
    <div v-if="$slots.below" class="below"><slot name="below" /></div>
  </header>
</template>

<style scoped>
.bar {
  position: sticky;
  top: 0;
  z-index: 5;
  margin: 0 -16px;
  padding: 0 24px 0 20px;
  border-bottom: 1px solid var(--ccm-glass-edge);
}

.row {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 56px;
}

.below {
  padding: 0 0 10px;
}

.back {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
  flex: none;
}

.heading {
  flex: 1;
  min-width: 0;
  padding-left: 8px;
}

.row:has(.back) .heading {
  padding-left: 0;
}

.title {
  margin: 0;
  font-size: 1.0625rem;
  font-weight: 700;
  line-height: 1.25;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sub {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
}
</style>
