<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Monitor, Smartphone } from 'lucide-vue-next'

defineProps<{ linked: boolean }>()
const { t } = useI18n()
</script>

<template>
  <div class="connect-link relative flex w-64 max-w-full items-center justify-between" :class="linked ? 'is-linked' : ''" role="img" :aria-label="linked ? t('v113Connect.orbit') : undefined" data-connect-link :data-linked="linked">
    <span class="grid size-10 place-items-center rounded-full bg-fill text-ink-muted"><Monitor :size="18" aria-hidden="true" /></span>
    <span class="connect-link-line absolute inset-x-12 top-1/2 h-0 border-t-2 border-dashed border-line-strong" aria-hidden="true" />
    <span class="connect-link-line-on absolute inset-x-12 top-1/2 h-0.5 -translate-y-1/2 origin-left bg-success" aria-hidden="true" />
    <span class="connect-link-planet absolute left-[42px] top-1/2 -mt-1.5 size-3 rounded-full bg-accent" aria-hidden="true" />
    <span class="grid size-10 place-items-center rounded-full" :class="linked ? 'bg-st-done-soft text-success' : 'bg-fill text-ink-muted'"><Smartphone :size="18" aria-hidden="true" /></span>
  </div>
</template>

<style scoped>
.connect-link-line-on {
  opacity: 0;
  scale: 0 1;
}
.connect-link-planet {
  opacity: 0;
}
.is-linked .connect-link-line-on {
  opacity: 1;
  scale: 1 1;
  transition:
    scale 600ms var(--ease-out-quint) 120ms,
    opacity 160ms var(--ease-out-quint);
}
.is-linked .connect-link-line {
  opacity: 0;
}
.is-linked .connect-link-planet {
  animation: connect-orbit 600ms var(--ease-out-quint) 1 both;
}
@keyframes connect-orbit {
  0% {
    opacity: 0;
    translate: 0 0;
  }
  15% {
    opacity: 1;
  }
  50% {
    translate: 80px -14px;
  }
  100% {
    opacity: 0;
    translate: 160px 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .is-linked .connect-link-planet {
    animation: none;
  }
  .is-linked .connect-link-line-on {
    transition: none;
  }
}
</style>
