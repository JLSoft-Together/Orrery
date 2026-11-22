<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, GitCommitHorizontal, LoaderCircle, X } from 'lucide-vue-next'
import { isMac } from '../../lib/platform'
import { gitCommit, gitCommitCancel, gitUnstage, type ChangesStatus, type SecretBlock } from '../../lib/preview'
import { stagedCount } from '../../lib/changesView'
import { useSettingsStore } from '../../stores/settings'
import SecretGuardDialog from '../../features/secretGuard/SecretGuardDialog.vue'
import { isEmptyBlock, isSecretBlock, laterNotice, withoutPath } from '../../features/secretGuard/secretGuard'

const props = defineProps<{ status: Extract<ChangesStatus, { ok: true }> }>()
const emit = defineEmits<{ committed: [subject: string] }>()

const { t } = useI18n()
// Drafts survive switching between repos (not reloads): one per repo root.
const drafts = new Map<string, string>()
const message = ref(drafts.get(props.status.root) ?? '')
const amend = ref(false)
const busy = ref(false)
const notice = ref<{ ok: boolean; text: string } | null>(null)
const settings = useSettingsStore()
const block = shallowRef<SecretBlock | null>(null)
const scanning = ref(false)
const unstaging = ref<string | null>(null)
const guardNote = ref<string | null>(null)

watch(
  () => props.status.root,
  (root, prev) => {
    if (prev) drafts.set(prev, message.value)
    message.value = drafts.get(root) ?? ''
    amend.value = false
    notice.value = null
    block.value = null
  },
)

const staged = computed(() => stagedCount(props.status.entries))
// HEAD that may still be local; the main process checks the remotes again before amending.
const mayAmend = computed(() => !props.status.upstream || props.status.ahead > 0)
watch(mayAmend, (v) => {
  if (!v) amend.value = false
})
const canCommit = computed(() => !busy.value && !!message.value.trim() && (staged.value > 0 || amend.value))
const keys = isMac ? '⌘↩' : 'Ctrl+Enter'

async function submit(force = false): Promise<void> {
  if (!canCommit.value) return
  busy.value = true
  notice.value = null
  const scan = settings.secretGuard && !force
  const stopNotice = scan ? laterNotice(() => (scanning.value = true)) : () => undefined
  const root = props.status.root
  const res = await gitCommit(root, message.value, amend.value, { scan })
  stopNotice()
  scanning.value = false
  busy.value = false
  if (isSecretBlock(res)) {
    if (props.status.root !== root) return
    guardNote.value = null
    block.value = res
    return
  }
  if (res.ok) {
    message.value = ''
    drafts.delete(props.status.root)
    amend.value = false
    notice.value = { ok: true, text: t('workspace.commit.done', { hash: res.hash.slice(0, 7), subject: res.subject }) }
    emit('committed', res.subject)
  } else if (res.error !== 'cancelled') {
    notice.value = { ok: false, text: t(`workspace.commit.error.${res.error}`, { detail: res.detail ?? '' }) }
  }
}

function proceed(): void {
  block.value = null
  void submit(true)
}

async function unstage(path: string): Promise<void> {
  const root = props.status.root
  unstaging.value = path
  const res = await gitUnstage(root, [path])
  unstaging.value = null
  if (!block.value || props.status.root !== root) return
  if (!res.ok) {
    guardNote.value = t('secretGuard.unstageFailed', { path })
    return
  }
  const next = withoutPath(block.value, path)
  if (isEmptyBlock(next)) {
    block.value = null
    notice.value = { ok: true, text: t('secretGuard.allRemoved') }
    return
  }
  block.value = next
  guardNote.value = t('secretGuard.unstaged', { path })
}

function onKey(e: KeyboardEvent): void {
  if (e.key === 'Enter' && (isMac ? e.metaKey : e.ctrlKey)) {
    e.preventDefault()
    void submit()
  }
}
</script>

<template>
  <div class="flex shrink-0 flex-col gap-1 px-2 pb-1.5">
    <textarea
      v-model="message"
      rows="2"
      spellcheck="true"
      class="ccm-scroll max-h-40 min-h-12 w-full resize-y rounded-inner border border-line bg-control px-2 py-1 text-xs text-ink outline-none placeholder:text-ink-faint focus-visible:ring-2 focus-visible:ring-focus"
      :placeholder="t('workspace.commit.placeholder', { branch: status.branch ?? '', keys })"
      :aria-label="t('workspace.commit.message')"
      :disabled="busy"
      @keydown="onKey"
    />
    <div class="flex min-w-0 items-center gap-1.5">
      <button
        type="button"
        class="ccm-btn ccm-btn-primary h-6 min-w-0 flex-1 justify-center gap-1 px-2 text-2xs"
        :disabled="!canCommit"
        :title="`${t('workspace.commit.action')} (${keys})`"
        data-commit-submit
        @click="submit()"
      >
        <LoaderCircle v-if="busy" :size="12" class="ccm-spin" aria-hidden="true" />
        <GitCommitHorizontal v-else :size="13" aria-hidden="true" />
        <span class="truncate">{{ scanning ? t('secretGuard.scanning') : amend ? t('workspace.commit.amendAction') : t('workspace.commit.actionN', { n: staged }, staged) }}</span>
      </button>
      <button v-if="busy" type="button" class="ccm-btn h-6 px-2 text-2xs" @click="gitCommitCancel(status.root)">
        {{ t('workspace.commit.cancel') }}
      </button>
      <label v-else-if="mayAmend" class="flex shrink-0 cursor-pointer items-center gap-1 text-2xs text-ink-muted" :title="t('workspace.commit.amendHint')">
        <input v-model="amend" type="checkbox" class="accent-(--ccm-accent)" />
        {{ t('workspace.commit.amend') }}
      </label>
    </div>
    <p v-if="!staged && !amend && message.trim()" class="text-2xs text-ink-faint">{{ t('workspace.commit.nothingStaged') }}</p>
    <SecretGuardDialog :block="block" mode="commit" :unstaging="unstaging" :note="guardNote" @unstage="unstage" @proceed="proceed" @cancel="block = null" />
    <p v-if="notice" data-commit-notice class="flex items-start gap-1 text-2xs" :class="notice.ok ? 'text-ink-muted' : 'text-st-error'" role="status">
      <Check v-if="notice.ok" :size="12" class="mt-px shrink-0 text-success" aria-hidden="true" />
      <span class="min-w-0 flex-1 break-words">{{ notice.text }}</span>
      <button type="button" class="shrink-0 cursor-pointer text-ink-faint hover:text-ink" :aria-label="t('workspace.explorer.status.dismiss')" @click="notice = null">
        <X :size="11" aria-hidden="true" />
      </button>
    </p>
  </div>
</template>
