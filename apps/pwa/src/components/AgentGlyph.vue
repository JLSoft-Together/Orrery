<script setup lang="ts">
import { Asterisk, Hexagon, SquareTerminal } from 'lucide-vue-next'
import { computed } from 'vue'
import type { Provider } from '../lib/monitorModel'

const props = withDefaults(defineProps<{ provider: Provider; avatar?: boolean; size?: number }>(), { avatar: false, size: 18 })
const icon = computed(() => (props.provider === 'codex' ? Hexagon : props.provider === 'shell' ? SquareTerminal : Asterisk))
</script>

<template>
  <span class="glyph" :class="[`p-${provider}`, { av: avatar }]" :data-provider="provider" aria-hidden="true">
    <component :is="icon" :size="avatar ? size + 1 : size" :stroke-width="2.2" />
  </span>
</template>

<style scoped>
.glyph {
  --tone: var(--ccm-agent-claude);
  display: inline-grid;
  place-items: center;
  flex: none;
  color: var(--tone);
}

.glyph.p-codex {
  --tone: var(--ccm-agent-codex);
}

.glyph.p-shell {
  --tone: var(--ccm-agent-shell);
}

.glyph.av {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--tone) 13%, transparent);
}
</style>
