<script setup lang="ts">
import { Minus, Plus, Undo2 } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import type { ChangeEntry } from '../lib/viewerModel'

defineProps<{ entry: ChangeEntry; staged: boolean; editable: boolean; busy: boolean }>()
const emit = defineEmits<{ open: []; toggle: []; discard: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="row" :data-testid="`row-${staged ? 'staged' : 'changes'}-${entry.path}`">
    <button type="button" class="entry" :data-testid="staged ? `staged-${entry.path}` : `change-${entry.path}`" @click="emit('open')">
      <span class="mark mono" :class="`m-${entry.mark === '?' ? 'u' : entry.mark}`">{{ entry.mark }}</span>
      <span class="name mono">{{ entry.path }}</span>
      <span v-if="entry.added !== null" class="plus mono">+{{ entry.added }}</span>
      <span v-if="entry.removed !== null" class="minus mono">−{{ entry.removed }}</span>
    </button>
    <template v-if="editable">
      <button v-if="!staged" type="button" class="tool" :disabled="busy" :aria-label="t('git.discard')" :data-testid="`discard-${entry.path}`" @click="emit('discard')"><Undo2 :size="18" :stroke-width="2" aria-hidden="true" /></button>
      <button type="button" class="tool" :disabled="busy" :aria-label="staged ? t('git.unstage') : t('git.stage')" :data-testid="`${staged ? 'unstage' : 'stage'}-${entry.path}`" @click="emit('toggle')">
        <Minus v-if="staged" :size="18" :stroke-width="2.4" aria-hidden="true" />
        <Plus v-else :size="18" :stroke-width="2.4" aria-hidden="true" />
      </button>
    </template>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
}

.entry {
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  gap: 10px;
  min-height: 44px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.tool {
  display: grid;
  place-items: center;
  flex: none;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink-muted);
}

.mark {
  width: 18px;
  flex: none;
  font-weight: 700;
  text-align: center;
  color: var(--ccm-warning);
}

.m-A,
.m-u {
  color: var(--ccm-done);
}

.m-D,
.m-\! {
  color: var(--ccm-error);
}

.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
}

.plus {
  color: var(--ccm-done);
  font-size: 0.8125rem;
}

.minus {
  color: var(--ccm-error);
  font-size: 0.8125rem;
}
</style>
