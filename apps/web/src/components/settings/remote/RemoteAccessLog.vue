<script setup lang="ts">
import { type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Ban, CircleAlert, CircleCheck, CircleSlash, History, RefreshCw } from 'lucide-vue-next'
import type { RemoteGuardLogLine, RemoteGuardResult } from '@ccm/shared'

defineProps<{ log: RemoteGuardLogLine[]; busy: boolean }>()
const emit = defineEmits<{ refresh: [] }>()
const { t, te, locale } = useI18n()

const RESULT: Record<RemoteGuardResult, { icon: Component; cls: string }> = {
  ok: { icon: CircleCheck, cls: 'text-success' },
  fail: { icon: CircleAlert, cls: 'text-warning' },
  blocked: { icon: Ban, cls: 'text-warning' },
  denied: { icon: CircleSlash, cls: 'text-ink-muted' },
}

const time = (sec: number): string => new Date(sec * 1000).toLocaleString(locale.value, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit' })
const whyText = (why: string): string => {
  const key = `remoteV98.guard.why.${why}`
  return te(key) ? t(key) : why
}
</script>

<template>
  <div data-remote-access-log>
    <div class="flex items-center gap-2 px-3 py-2">
      <p class="min-w-0 flex-1 text-2xs text-ink-muted">{{ t('remoteV98.guard.log.hint') }}</p>
      <button type="button" class="ccm-btn shrink-0" :disabled="busy" data-remote-access-refresh @click="emit('refresh')">
        <RefreshCw :size="13" aria-hidden="true" />
        {{ t('remoteV98.guard.log.refresh') }}
      </button>
    </div>
    <ol v-if="log.length" class="ccm-scroll max-h-72 divide-y divide-line overflow-x-hidden overflow-y-auto border-t border-line" data-remote-access-lines>
      <li v-for="(line, i) in log" :key="`${line.at}:${i}`" class="flex items-baseline gap-2 px-3 py-1.5 text-xs" :data-result="line.result">
        <time class="shrink-0 font-mono text-2xs text-ink-faint tabular-nums" :datetime="new Date(line.at * 1000).toISOString()">{{ time(line.at) }}</time>
        <span class="inline-flex shrink-0 items-center gap-1 font-medium" :class="RESULT[line.result]?.cls ?? 'text-ink-muted'">
          <component :is="RESULT[line.result]?.icon ?? CircleAlert" :size="12" aria-hidden="true" />
          {{ t(`remoteV98.guard.log.result.${line.result}`) }}
        </span>
        <span class="shrink-0 font-mono text-2xs" :title="line.ip">{{ line.ip }}<template v-if="line.country"> [{{ line.country }}]</template></span>
        <span class="shrink-0 text-2xs text-ink-faint">{{ t(`remoteV98.guard.log.via.${line.via}`) }}</span>
        <span class="min-w-0 flex-1 truncate text-ink-muted" :title="line.device ? `${line.device} · ${whyText(line.why)}` : whyText(line.why)">
          <template v-if="line.device">{{ line.device }} · </template>{{ whyText(line.why) }}
        </span>
      </li>
    </ol>
    <div v-else class="flex items-center gap-3 border-t border-line px-3 py-3 text-sm text-ink-muted" role="status" data-remote-access-empty>
      <History :size="18" class="shrink-0" aria-hidden="true" />
      <span>{{ t('remoteV98.guard.log.empty') }}</span>
    </div>
  </div>
</template>
