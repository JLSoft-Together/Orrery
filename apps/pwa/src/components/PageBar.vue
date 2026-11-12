<script setup lang="ts">
import { ChevronLeft } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { back } from '../router'

defineProps<{ title: string; backless?: boolean }>()
const { t } = useI18n()
</script>

<template>
  <header class="bar">
    <button v-if="!backless" type="button" class="back" :aria-label="t('common.back')" data-testid="back" @click="back()">
      <ChevronLeft :size="24" :stroke-width="2" aria-hidden="true" />
    </button>
    <h1 class="title" data-testid="page-title">{{ title }}</h1>
    <div class="right"><slot name="right" /></div>
  </header>
</template>

<style scoped>
.bar {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 56px;
  padding: 0 8px 0 4px;
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

.title {
  flex: 1;
  min-width: 0;
  margin: 0;
  padding-left: 8px;
  font-size: 1.0625rem;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bar:has(.back) .title {
  padding-left: 0;
}

.right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
}
</style>
