<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { LoaderCircle, Trash2 } from 'lucide-vue-next'
import { wtCleanup } from '../../../core/lib/preview'
import { PtyConnection } from '../../workspace/lib/ptyClient'
import { useWorkspaceStore } from '../../workspace/stores/workspace'
import { cleanupWorktree, panesInside, type CleanupDeps, type CleanupError, type CleanupOutcome } from '../lib/worktreeFlow'

const props = defineProps<{ root: string; branch: string }>()
const emit = defineEmits<{ done: [result: 'removed' | 'kept' | 'failed'] }>()

const { t } = useI18n()
const workspace = useWorkspaceStore()

type Phase = 'checking' | 'ask' | 'running' | 'done' | 'error'
const phase = ref<Phase>('checking')
const path = ref('')
const cleanup = ref<string[]>([])
const outcome = ref<CleanupOutcome | null>(null)
const sheet = ref<HTMLElement | null>(null)
const keepButton = ref<HTMLButtonElement | null>(null)
let restore: HTMLElement | null = null

const paneCount = computed(() => (path.value ? panesInside(workspace.panes, path.value).length : 0))
const errorText = (code: CleanupError, detail?: string): string => t(`workspace.wt.cleanup.error.${code}`, { detail: detail ?? '' })

const deps: CleanupDeps = {
  check: (root, branch) => wtCleanup(root, branch, 'check'),
  run: (root, branch) => wtCleanup(root, branch, 'run'),
  panes: () => workspace.panes,
  closePanes: (keys) => workspace.requestClose(keys, () => false, { record: false }),
  remove: async (root, target) => {
    const reply = await PtyConnection.request({ t: 'wt.remove', cwd: root, path: target, deleteBranch: true }, 130_000)
    return reply as never
  },
  wait: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
}

async function check(): Promise<void> {
  const res = await wtCleanup(props.root, props.branch, 'check')
  if (res.ok) {
    path.value = res.path
    cleanup.value = res.cleanup ?? []
    phase.value = 'ask'
  } else {
    outcome.value = { ok: false, error: res.error, detail: res.detail, step: 'check' }
    phase.value = 'error'
  }
  await nextTick()
  keepButton.value?.focus()
}

async function remove(): Promise<void> {
  if (phase.value !== 'ask') return
  phase.value = 'running'
  const res = await cleanupWorktree(props.root, props.branch, deps)
  outcome.value = res
  phase.value = res.ok ? 'done' : 'error'
  await nextTick()
  keepButton.value?.focus()
}

function close(): void {
  if (phase.value === 'running') return
  emit('done', phase.value === 'done' ? 'removed' : phase.value === 'error' ? 'failed' : 'kept')
}

function trap(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    close()
    return
  }
  if (e.key !== 'Tab') return
  const items = [...(sheet.value?.querySelectorAll<HTMLElement>('button:not(:disabled)') ?? [])]
  if (!items.length) return
  const at = items.indexOf(document.activeElement as HTMLElement)
  e.preventDefault()
  items[(at + (e.shiftKey ? -1 : 1) + items.length) % items.length]!.focus()
}

const result = computed(() => {
  const o = outcome.value
  if (!o) return ''
  if (!o.ok) return errorText(o.error, o.detail)
  if (o.branch === 'kept') return t('workspace.wt.cleanup.doneKept', { branch: props.branch, detail: o.detail ?? '' })
  return t('workspace.wt.cleanup.done', { branch: props.branch })
})

onMounted(() => {
  restore = document.activeElement as HTMLElement | null
  void check()
})
onBeforeUnmount(() => {
  if (restore?.isConnected) restore.focus()
})
</script>

<template>
  <Teleport to="body">
    <Transition name="ccm-dialog" appear>
      <div class="ccm-scrim fixed inset-0 z-50 flex items-center justify-center p-4" data-wt-cleanup @click.self="close">
        <div
          ref="sheet"
          class="ccm-sheet flex w-full max-w-[460px] flex-col gap-3 p-5"
          role="alertdialog"
          aria-modal="true"
          aria-labelledby="wt-cleanup-title"
          aria-describedby="wt-cleanup-body"
          :aria-busy="phase === 'running' || phase === 'checking'"
          @keydown="trap"
        >
          <div class="flex items-start gap-3">
            <Trash2 :size="22" class="mt-0.5 shrink-0 text-ink-muted" aria-hidden="true" />
            <h2 id="wt-cleanup-title" class="min-w-0 break-words text-base font-semibold text-ink">{{ t('workspace.wt.cleanup.title', { branch }) }}</h2>
          </div>
          <div id="wt-cleanup-body" class="flex flex-col gap-2 text-sm text-ink-muted" role="status">
            <p v-if="phase === 'checking'" class="flex items-center gap-2">
              <LoaderCircle :size="14" class="ccm-spin" aria-hidden="true" />{{ t('workspace.wt.cleanup.checking') }}
            </p>
            <template v-else-if="phase === 'ask' || phase === 'running'">
              <p>{{ t('workspace.wt.cleanup.body') }}</p>
              <p v-if="paneCount" data-wt-cleanup-panes>{{ t('workspace.wt.cleanup.panes', { n: paneCount }, paneCount) }}</p>
              <p v-if="cleanup.length" class="flex flex-col gap-1">
                <span>{{ t('workspace.wt.cleanup.commands') }}</span>
                <code class="ccm-scroll max-h-24 overflow-auto whitespace-pre-wrap break-all rounded-inner bg-well px-2 py-1.5 font-mono text-2xs text-well-ink">{{ cleanup.join('\n') }}</code>
              </p>
              <p v-if="phase === 'running'" class="flex items-center gap-2 text-ink">
                <LoaderCircle :size="14" class="ccm-spin" aria-hidden="true" />{{ t('workspace.wt.cleanup.working') }}
              </p>
            </template>
            <p v-else :class="phase === 'error' ? 'text-st-error' : 'text-ink'" data-wt-cleanup-result :role="phase === 'error' ? 'alert' : 'status'">{{ result }}</p>
          </div>
          <div class="flex justify-end gap-2 pt-1">
            <template v-if="phase === 'done' || phase === 'error' || phase === 'checking'">
              <button ref="keepButton" type="button" class="ccm-btn" data-wt-cleanup-close @click="close">{{ t('workspace.wt.cleanup.close') }}</button>
            </template>
            <template v-else>
              <button ref="keepButton" type="button" class="ccm-btn" :disabled="phase === 'running'" data-wt-cleanup-keep @click="close">{{ t('workspace.wt.cleanup.keep') }}</button>
              <button type="button" class="ccm-btn ccm-btn-danger" :disabled="phase === 'running'" data-wt-cleanup-remove @click="remove">
                <LoaderCircle v-if="phase === 'running'" :size="13" class="ccm-spin" aria-hidden="true" />{{ t('workspace.wt.cleanup.remove') }}
              </button>
            </template>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
