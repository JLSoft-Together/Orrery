<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Plus, ShieldCheck, X } from 'lucide-vue-next'
import { REMOTE_GUARD_RULES_MAX, type RemoteGuardOp } from '@ccm/shared'
import { looksLikeRule, splitRules } from '../../../lib/remoteGuard'

const props = defineProps<{ trusted: string[]; busy: boolean }>()
const emit = defineEmits<{ run: [op: RemoteGuardOp] }>()
const { t } = useI18n()

const draft = ref('')
const items = computed(() => splitRules(draft.value))
const valid = computed(() => items.value.length > 0 && items.value.every(looksLikeRule) && props.trusted.length + items.value.length <= REMOTE_GUARD_RULES_MAX)
const showInvalid = computed(() => draft.value.trim() !== '' && !valid.value)

function add(): void {
  if (!valid.value) return
  emit('run', { op: 'trust', list: [...props.trusted, ...items.value] })
  draft.value = ''
}

function remove(rule: string): void {
  emit('run', { op: 'trust', list: props.trusted.filter((r) => r !== rule) })
}
</script>

<template>
  <div data-remote-guard-trust>
    <p class="px-3 py-2 text-2xs text-ink-muted">{{ t('remoteV98.guard.trusted.hint') }}</p>
    <ul v-if="trusted.length" class="divide-y divide-line border-t border-line" data-remote-guard-trust-list>
      <li v-for="rule in trusted" :key="rule" class="flex items-center gap-2 px-3 py-1.5" :data-remote-guard-trusted="rule">
        <ShieldCheck :size="14" class="shrink-0 text-success" aria-hidden="true" />
        <span class="min-w-0 flex-1 truncate font-mono text-xs" :title="rule">{{ rule }}</span>
        <button type="button" class="ccm-btn-plain shrink-0" :disabled="busy" :aria-label="t('remoteV98.guard.trusted.removeLabel', { ip: rule })" :title="t('remoteV98.guard.trusted.remove')" @click="remove(rule)">
          <X :size="14" aria-hidden="true" />
        </button>
      </li>
    </ul>
    <p v-else class="border-t border-line px-3 py-2 text-xs text-ink-muted" role="status" data-remote-guard-trust-empty>{{ t('remoteV98.guard.trusted.empty') }}</p>
    <form class="flex flex-wrap items-center gap-2 border-t border-line px-3 py-2" @submit.prevent="add">
      <label class="sr-only" for="remote-guard-trust">{{ t('remoteV98.guard.trusted.field') }}</label>
      <input
        id="remote-guard-trust"
        v-model="draft"
        type="text"
        autocomplete="off"
        spellcheck="false"
        maxlength="512"
        class="ccm-field min-w-0 flex-1 font-mono text-xs"
        :placeholder="t('remoteV98.guard.add.placeholder')"
        :aria-invalid="showInvalid"
        data-remote-guard-trust-input
      />
      <button type="submit" class="ccm-btn shrink-0" :disabled="busy || !valid" data-remote-guard-trust-add>
        <Plus :size="14" aria-hidden="true" />
        {{ t('remoteV98.guard.trusted.add') }}
      </button>
      <p v-if="showInvalid" class="w-full text-2xs text-warning" role="alert">{{ t('remoteV98.guard.error.invalid') }}</p>
    </form>
  </div>
</template>
