<script setup lang="ts">
import { computed, ref, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { Ban, Cloud, Hand, ShieldCheck, Wifi } from 'lucide-vue-next'
import type { RemoteGuardBlock, RemoteGuardError, RemoteGuardOp, RemoteGuardSource } from '@ccm/shared'
import { GUARD_TTLS, looksLikeRule } from '../../../lib/remoteGuard'

const props = defineProps<{ blocked: RemoteGuardBlock[]; busy: boolean; error: RemoteGuardError | null }>()
const emit = defineEmits<{ run: [op: RemoteGuardOp] }>()
const { t, te, locale } = useI18n()

const askAll = ref(false)
const draft = ref('')
const ttl = ref<number | null>(86_400)
const touched = ref(false)

const SOURCE: Record<RemoteGuardSource, { icon: Component; chip: string }> = {
  lan: { icon: Wifi, chip: 'ccm-chip-warning' },
  relay: { icon: Cloud, chip: 'ccm-chip-warning' },
  manual: { icon: Hand, chip: 'ccm-chip-quiet' },
}
const TTL_KEY: Record<string, string> = { '3600': 'h1', '86400': 'd1', '604800': 'd7', '2592000': 'd30', null: 'forever' }

const valid = computed(() => looksLikeRule(draft.value))
const showInvalid = computed(() => touched.value && draft.value.trim() !== '' && !valid.value)
const shownError = computed(() => (showInvalid.value ? 'invalid' : props.error))

const when = (sec: number): string => new Date(sec * 1000).toLocaleString(locale.value, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
const whyText = (why: string): string => {
  const key = `remoteV98.guard.why.${why}`
  return te(key) ? t(key) : why
}

function block(): void {
  touched.value = true
  if (!valid.value) return
  emit('run', { op: 'block', cidr: draft.value.trim(), ttl: ttl.value })
  draft.value = ''
  touched.value = false
}

function unblockAll(): void {
  emit('run', { op: 'unblock-all' })
  askAll.value = false
}
</script>

<template>
  <div data-remote-guard-blocked>
    <p class="flex items-start gap-2 px-3 py-2 text-2xs text-ink-muted">
      <ShieldCheck :size="14" class="mt-px shrink-0 text-success" aria-hidden="true" />
      <span>{{ t('remoteV98.guard.selfHint') }}</span>
    </p>
    <TransitionGroup v-if="blocked.length" tag="ul" name="ccm-list" class="ccm-list-inflow relative divide-y divide-line border-t border-line" data-remote-guard-list>
      <li v-for="b in blocked" :key="`${b.source}:${b.key}`" class="flex items-center gap-3 px-3 py-2" :data-remote-guard-row="b.key" :data-source="b.source">
        <div class="min-w-0 flex-1">
          <p class="flex min-w-0 flex-wrap items-center gap-2">
            <span class="truncate font-mono text-xs font-medium" :title="b.key">{{ b.key }}</span>
            <span class="ccm-chip inline-flex h-5 items-center gap-1 px-1.5 text-2xs font-medium" :class="SOURCE[b.source].chip" data-remote-guard-source>
              <component :is="SOURCE[b.source].icon" :size="12" aria-hidden="true" />
              {{ t(`remoteV98.guard.source.${b.source}`) }}
            </span>
            <span class="min-w-0 truncate text-xs text-ink-muted">{{ whyText(b.why) }}</span>
          </p>
          <dl class="mt-0.5 flex flex-wrap gap-x-3 text-2xs text-ink-faint">
            <div v-if="b.source !== 'manual'" class="flex gap-1">
              <dt>{{ t('remoteV98.guard.cols.count') }}</dt>
              <dd class="tabular-nums">{{ b.count }}</dd>
            </div>
            <div class="flex gap-1">
              <dt>{{ t('remoteV98.guard.cols.since') }}</dt>
              <dd class="tabular-nums">{{ when(b.at) }}</dd>
            </div>
            <div class="flex gap-1">
              <dt>{{ t('remoteV98.guard.cols.until') }}</dt>
              <dd class="tabular-nums" data-remote-guard-until>{{ b.until === null ? t('remoteV98.guard.forever') : when(b.until) }}</dd>
            </div>
            <div v-if="b.level > 0">{{ t('remoteV98.guard.level', { n: b.level }) }}</div>
          </dl>
        </div>
        <button
          type="button"
          class="ccm-btn shrink-0"
          :disabled="busy"
          :aria-label="t('remoteV98.guard.unblockLabel', { ip: b.key })"
          data-remote-guard-unblock
          @click="emit('run', { op: 'unblock', key: b.key })"
        >
          {{ t('remoteV98.guard.unblock') }}
        </button>
      </li>
    </TransitionGroup>
    <div v-else class="flex items-center gap-3 border-t border-line px-3 py-3 text-sm text-ink-muted" role="status" data-remote-guard-empty>
      <ShieldCheck :size="18" class="shrink-0" aria-hidden="true" />
      <span>{{ t('remoteV98.guard.empty') }}</span>
    </div>

    <Transition name="ccm-fade" mode="out-in">
      <div v-if="blocked.length && !askAll" key="all" class="flex justify-end border-t border-line px-3 py-2">
        <button type="button" class="ccm-btn ccm-btn-danger" :disabled="busy" data-remote-guard-unblock-all @click="askAll = true">{{ t('remoteV98.guard.unblockAll') }}</button>
      </div>
      <div v-else-if="blocked.length" key="ask" class="flex flex-wrap items-center justify-end gap-2 border-t border-line px-3 py-2" role="group" :aria-label="t('remoteV98.guard.unblockAllAsk')">
        <span class="min-w-0 text-xs text-ink-muted">{{ t('remoteV98.guard.unblockAllAsk') }}</span>
        <button type="button" class="ccm-btn ccm-btn-danger" :disabled="busy" data-remote-guard-unblock-all-yes @click="unblockAll">{{ t('remoteV98.guard.unblockAllYes') }}</button>
        <button type="button" class="ccm-btn" :disabled="busy" @click="askAll = false">{{ t('remoteV98.guard.cancel') }}</button>
      </div>
    </Transition>

    <form class="border-t border-line px-3 py-2" data-remote-guard-add @submit.prevent="block">
      <div class="flex flex-wrap items-center gap-2">
        <label class="sr-only" for="remote-guard-cidr">{{ t('remoteV98.guard.add.field') }}</label>
        <input
          id="remote-guard-cidr"
          v-model="draft"
          type="text"
          inputmode="text"
          autocomplete="off"
          spellcheck="false"
          maxlength="64"
          class="ccm-field min-w-0 flex-1 font-mono text-xs"
          :placeholder="t('remoteV98.guard.add.placeholder')"
          :aria-invalid="showInvalid"
          aria-describedby="remote-guard-add-hint"
          data-remote-guard-cidr
          @blur="touched = true"
        />
        <label class="text-xs text-ink-muted" for="remote-guard-ttl">{{ t('remoteV98.guard.add.ttl') }}</label>
        <select id="remote-guard-ttl" v-model="ttl" class="ccm-field cursor-pointer text-xs" data-remote-guard-ttl>
          <option v-for="v in GUARD_TTLS" :key="String(v)" :value="v">{{ t(`remoteV98.guard.add.ttls.${TTL_KEY[String(v)]}`) }}</option>
        </select>
        <button type="submit" class="ccm-btn ccm-btn-danger shrink-0" :disabled="busy || !draft.trim()" data-remote-guard-block>
          <Ban :size="14" aria-hidden="true" />
          {{ t('remoteV98.guard.add.button') }}
        </button>
      </div>
      <p id="remote-guard-add-hint" class="mt-1 text-2xs" :class="shownError ? 'text-warning' : 'text-ink-faint'" :role="shownError ? 'alert' : undefined" data-remote-guard-hint>
        {{ shownError ? t(`remoteV98.guard.error.${shownError}`) : t('remoteV98.guard.add.hint') }}
      </p>
    </form>
  </div>
</template>
