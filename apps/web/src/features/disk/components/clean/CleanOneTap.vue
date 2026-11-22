<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { LoaderCircle, Sparkles, X } from 'lucide-vue-next'
import { formatSize } from '../../lib/diskMap'
import { CLEAN_PERMANENT_OK } from '../../lib/diskClean'

const props = defineProps<{
  state: 'idle' | 'scanning' | 'ready'
  groups: Array<{ id: string; count: number; size: number; on: boolean }>
  size: number
  days: number
  titles: Record<string, string>
  deleting: boolean
}>()
const emit = defineEmits<{ toggle: [id: string]; run: []; close: [] }>()

const { t, locale } = useI18n()
const picked = computed(() => props.groups.filter((g) => g.on).length)
</script>

<template>
  <section class="ccm-panel rounded-card border border-line px-3 py-2.5" :aria-label="t('diskClean.oneTap')" data-clean-onetap>
    <header class="flex flex-wrap items-start gap-2">
      <Sparkles :size="15" class="mt-0.5 shrink-0 text-accent-ink" aria-hidden="true" />
      <div class="min-w-0 flex-1 basis-60">
        <h3 class="text-sm font-semibold">{{ t('diskClean.oneTap') }}</h3>
        <p class="mt-0.5 text-2xs leading-relaxed text-ink-muted">{{ t('diskClean.oneTapHint', { days }) }}</p>
      </div>
      <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('diskClean.oneTapClose')" :title="t('diskClean.oneTapClose')" data-clean-onetap-close @click="emit('close')">
        <X :size="13" aria-hidden="true" />
      </button>
    </header>
    <p v-if="state === 'scanning'" class="mt-2 flex items-center gap-1.5 text-xs text-ink-muted" role="status" data-clean-onetap-scanning>
      <LoaderCircle :size="13" class="animate-spin" aria-hidden="true" />{{ t('diskClean.oneTapScanning') }}
    </p>
    <template v-else>
      <p v-if="!groups.length" class="mt-2 text-xs text-ink-muted" data-clean-onetap-empty>{{ t('diskClean.oneTapNothing') }}</p>
      <ul v-else class="mt-2 divide-y divide-line">
        <li v-for="g in groups" :key="g.id" class="flex min-w-0 items-center gap-2 py-1.5 text-xs" :data-clean-onetap-group="g.id">
          <input
            :id="`onetap-${g.id}`"
            type="checkbox"
            class="size-3.5 shrink-0 accent-[var(--ccm-accent)]"
            :checked="g.on"
            :disabled="deleting"
            data-clean-onetap-check
            @change="emit('toggle', g.id)"
          />
          <label :for="`onetap-${g.id}`" class="min-w-0 flex-1 truncate">{{ titles[g.id] ?? g.id }}</label>
          <span v-if="CLEAN_PERMANENT_OK.includes(g.id)" class="ccm-chip ccm-chip-warning shrink-0">{{ t('diskClean.oneTapPerm') }}</span>
          <span class="shrink-0 text-2xs text-ink-muted tabular-nums">{{ t('diskClean.items', { n: g.count }) }}</span>
          <span class="w-16 shrink-0 text-right font-medium tabular-nums">{{ formatSize(g.size, locale) }}</span>
        </li>
      </ul>
      <div v-if="groups.length" class="mt-2 flex justify-end">
        <button type="button" class="ccm-btn ccm-btn-primary" :disabled="deleting || !picked || !size" data-clean-onetap-run @click="emit('run')">
          <Sparkles :size="13" aria-hidden="true" />{{ t('diskClean.oneTapRun', { size: formatSize(size, locale) }) }}
        </button>
      </div>
    </template>
  </section>
</template>
