<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { History } from 'lucide-vue-next'
import { useRemoteStore } from '../../../stores/remote'

const { t, locale } = useI18n()
const remote = useRemoteStore()

const time = (at: number): string => new Date(at).toLocaleTimeString(locale.value, { hour: '2-digit', minute: '2-digit', second: '2-digit' })
</script>

<template>
  <TransitionGroup v-if="remote.log.length" tag="ol" name="ccm-list" class="ccm-list-inflow ccm-scroll relative max-h-64 divide-y divide-line overflow-x-hidden overflow-y-auto" data-remote-log>
    <li v-for="line in remote.log" :key="line.id" class="flex items-baseline gap-2 px-3 py-1.5 text-xs">
      <time class="shrink-0 font-mono text-2xs text-ink-faint tabular-nums" :datetime="new Date(line.at).toISOString()">{{ time(line.at) }}</time>
      <span class="shrink-0 font-medium">{{ line.device }}</span>
      <span class="min-w-0 flex-1 truncate text-ink-muted" :title="line.text">{{ line.text }}</span>
    </li>
  </TransitionGroup>
  <div v-else class="flex items-center gap-3 px-3 py-4 text-sm text-ink-muted" role="status" data-remote-log-empty>
    <History :size="18" class="shrink-0" aria-hidden="true" />
    <span>{{ t('remoteV97.log.empty') }}</span>
  </div>
</template>
