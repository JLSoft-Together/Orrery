<script setup lang="ts">
import { Cloud, LoaderCircle, MoonStar, RefreshCw, ShieldOff, TriangleAlert, Wifi, WifiOff } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { LinkState } from '../remote/types'

const props = defineProps<{ link: LinkState; rtt?: number }>()
const { t } = useI18n()

const meta = computed(() => {
  const { status, route } = props.link
  if (status === 'ready' && route === 'lan') {
    const ms = props.rtt === undefined ? '' : ` · ${props.rtt} ms`
    return { icon: Wifi, tone: 'done', text: `${t('link.lan')}${ms}` }
  }
  if (status === 'ready') return { icon: Cloud, tone: 'info', text: t('link.relay') }
  if (status === 'discovering') return { icon: LoaderCircle, tone: 'working', text: t('link.discovering'), spin: true }
  if (status === 'handshaking') return { icon: LoaderCircle, tone: 'working', text: t('link.handshaking'), spin: true }
  if (status === 'reconnecting') return { icon: RefreshCw, tone: 'warning', text: t('link.reconnecting') }
  if (status === 'revoked') return { icon: ShieldOff, tone: 'error', text: t('link.revoked') }
  if (status === 'protoTooOld') return { icon: TriangleAlert, tone: 'error', text: t('link.protoTooOld') }
  if (status === 'offline') return { icon: WifiOff, tone: 'idle', text: t('link.offline') }
  return { icon: MoonStar, tone: 'idle', text: t('link.idle') }
})
</script>

<template>
  <span class="chip" :class="`tone-${meta.tone}`" data-testid="link-chip" :data-status="link.status" :data-route="link.route ?? ''">
    <component :is="meta.icon" :size="14" :stroke-width="2.2" class="chip-icon" :class="{ spin: meta.spin }" aria-hidden="true" />
    <span>{{ meta.text }}</span>
  </span>
</template>

<style scoped>
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: var(--radius-chip);
  border: 1px solid currentColor;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
}

.tone-done {
  color: var(--ccm-done);
}

.tone-info {
  color: var(--ccm-info);
}

.tone-working {
  color: var(--ccm-working);
}

.tone-warning {
  color: var(--ccm-warning);
}

.tone-idle {
  color: var(--ccm-idle);
}

.tone-error {
  color: var(--ccm-error);
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
