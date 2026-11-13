<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronsLeftRight, FolderPlus, PanelTopOpen, Plus, X } from 'lucide-vue-next'
import { GROUP_NAME_MAX, type Box } from '../../../lib/splitLayout'
import { boxStyle } from './geometry'

export interface GroupFrame {
  project: string
  name: string
  letter: string
  tone: number
  count: number
  active: boolean
  folded: boolean
  vertical: boolean
  box: Box
}

defineProps<{ frames: GroupFrame[] }>()
const emit = defineEmits<{
  add: [project: string]
  addProject: [project: string]
  toTab: [project: string]
  close: [project: string]
  focus: [project: string]
  unfold: [project: string]
  rename: [project: string, name: string]
  menu: [frame: GroupFrame, event: MouseEvent, el: HTMLElement]
  grab: [event: PointerEvent, frame: GroupFrame, el: HTMLElement]
}>()
const { t } = useI18n()

const renaming = ref<string | null>(null)
const draft = ref('')
const input = ref<HTMLInputElement[] | null>(null)

async function startRename(f: GroupFrame): Promise<void> {
  renaming.value = f.project
  draft.value = f.name
  await nextTick()
  input.value?.[0]?.focus()
  input.value?.[0]?.select()
}

function endRename(save: boolean): void {
  const project = renaming.value
  renaming.value = null
  if (save && project !== null) emit('rename', project, draft.value)
}

defineExpose({ startRename })

function onDown(e: PointerEvent, frame: GroupFrame): void {
  if (e.button !== 0 || (e.target as HTMLElement).closest('button, input')) return
  emit('grab', e, frame, e.currentTarget as HTMLElement)
}
</script>

<template>
  <section
    v-for="f in frames"
    :key="f.project"
    class="ccm-group-frame pointer-events-none absolute"
    :style="boxStyle(f.box)"
    :data-group-frame="f.project"
    :data-project-tone="f.tone"
    :data-active="f.active ? '' : undefined"
    :data-folded="f.folded ? '' : undefined"
    :aria-label="t('groupsV99.label', { name: f.name })"
  >
    <button
      v-if="f.folded"
      type="button"
      class="pointer-events-auto flex size-full cursor-pointer items-center gap-1.5 rounded-card p-1.5 text-ink hover:bg-fill focus-visible:bg-fill"
      :class="f.vertical ? 'flex-col' : 'flex-row'"
      :title="`${f.project}\n${t('groupsV99.unfold', { name: f.name })}`"
      :aria-label="t('groupsV99.unfold', { name: f.name })"
      data-group-unfold
      @click="emit('unfold', f.project)"
      @contextmenu.prevent="emit('menu', f, $event, $event.currentTarget as HTMLElement)"
    >
      <span class="ccm-group-mono" aria-hidden="true">{{ f.letter }}</span>
      <span class="min-h-0 min-w-0 truncate text-xs font-semibold" :class="f.vertical ? 'ccm-group-vertical' : ''">{{ f.name }}</span>
      <span class="shrink-0 text-2xs text-ink-muted tabular-nums">{{ f.count }}</span>
      <ChevronsLeftRight :size="13" class="mt-auto shrink-0 text-ink-faint" aria-hidden="true" />
    </button>
    <header
      v-else
      class="pointer-events-auto flex h-[30px] min-w-0 cursor-grab items-center gap-1.5 pr-1 pl-1.5 select-none active:cursor-grabbing"
      :title="`${f.project}\n${t('groupsV99.dragHint')}`"
      data-group-head
      @pointerdown="onDown($event, f)"
      @dblclick="emit('focus', f.project)"
      @contextmenu.prevent="emit('menu', f, $event, $event.currentTarget as HTMLElement)"
    >
      <span class="ccm-group-chip inline-flex h-5 min-w-0 shrink items-center gap-1.5 rounded-control pr-2 pl-0.5" data-group-chip>
        <span class="ccm-group-mono" aria-hidden="true">{{ f.letter }}</span>
        <input
          v-if="renaming === f.project"
          ref="input"
          v-model="draft"
          class="w-40 min-w-0 rounded-inner border border-line bg-canvas px-1 text-xs font-semibold text-ink outline-none focus-visible:ring-2 focus-visible:ring-focus"
          :maxlength="GROUP_NAME_MAX"
          :aria-label="t('groupsV99.nameLabel')"
          data-group-rename
          @keydown.enter.prevent="endRename(true)"
          @keydown.esc.prevent.stop="endRename(false)"
          @blur="endRename(true)"
        />
        <span v-else class="truncate text-xs font-semibold text-ink" data-group-name>{{ f.name }}</span>
      </span>
      <span class="shrink-0 text-2xs text-ink-muted tabular-nums">{{ t('groupsV99.panes', { n: f.count }, f.count) }}</span>
      <span class="min-w-0 flex-1" />
      <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('groupsV99.newSession', { name: f.name })" :title="t('groupsV99.newSession', { name: f.name })" data-group-add @click="emit('add', f.project)">
        <Plus :size="14" aria-hidden="true" />
      </button>
      <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('groupsV99.addProject', { name: f.name })" :title="t('groupsV99.addProject', { name: f.name })" data-group-add-project @click="emit('addProject', f.project)">
        <FolderPlus :size="14" aria-hidden="true" />
      </button>
      <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('groupsV99.toTab', { name: f.name })" :title="t('groupsV99.toTab', { name: f.name })" data-group-to-tab @click="emit('toTab', f.project)">
        <PanelTopOpen :size="14" aria-hidden="true" />
      </button>
      <button type="button" class="ccm-btn-plain shrink-0" :aria-label="t('groupsV99.close', { name: f.name })" :title="t('groupsV99.close', { name: f.name })" data-group-close @click="emit('close', f.project)">
        <X :size="14" aria-hidden="true" />
      </button>
    </header>
  </section>
</template>

<style scoped>
.ccm-group-frame {
  --tone: var(--ccm-accent);
  border-radius: var(--radius-card);
  box-shadow: inset 0 0 0 1px var(--ccm-chrome-line);
  transition: box-shadow var(--ccm-dur-fast, 120ms) ease-out;
}
.ccm-group-frame[data-active] {
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tone) 70%, var(--ccm-chrome-line));
}
.ccm-group-frame[data-folded] {
  background: var(--ccm-material);
  box-shadow: inset 0 0 0 1px var(--ccm-chrome-line), inset 3px 0 0 0 var(--tone);
}
.ccm-group-frame[data-project-tone='0'] { --tone: var(--ccm-agent-claude); }
.ccm-group-frame[data-project-tone='1'] { --tone: var(--ccm-agent-codex); }
.ccm-group-frame[data-project-tone='2'] { --tone: var(--ccm-agent-browser); }
.ccm-group-frame[data-project-tone='3'] { --tone: var(--ccm-agent-file); }
.ccm-group-frame[data-project-tone='4'] { --tone: var(--ccm-success); }
.ccm-group-frame[data-project-tone='5'] { --tone: var(--ccm-peer); }
.ccm-group-frame[data-project-tone='6'] { --tone: var(--ccm-warning); }
.ccm-group-frame[data-project-tone='7'] { --tone: var(--ccm-accent); }
.ccm-group-chip {
  background: color-mix(in srgb, var(--tone) 16%, var(--ccm-canvas, transparent));
}
.ccm-group-mono {
  display: inline-flex;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-inner);
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
  color: var(--ccm-mono-ink);
  background: color-mix(in srgb, var(--tone) 62%, var(--ccm-mono-mix));
}
.ccm-group-vertical {
  writing-mode: vertical-rl;
  max-height: 100%;
}
</style>
