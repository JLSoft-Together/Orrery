<script setup lang="ts">
import { ChevronLeft, File, Folder } from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAsync } from '../lib/useAsync'
import { viewerData, type Commander } from '../lib/viewerApi'
import { parseDir, parseFile, relativePath, samePath, sizeText } from '../lib/viewerModel'
import AsyncState from './AsyncState.vue'
import CodeLines from './CodeLines.vue'

const props = defineProps<{ command: Commander; root: string; open?: string | null }>()
const emit = defineEmits<{ opened: [path: string | null] }>()
const { t } = useI18n()
const dir = ref(props.root)
const file = ref<string | null>(props.open ?? null)

const list = useAsync(async () => parseDir(await viewerData(props.command, 'fs.list', { root: props.root, dir: dir.value })), false)
const content = useAsync(async () => parseFile(await viewerData(props.command, 'fs.read', { path: file.value })), false)

const atRoot = computed(() => samePath(dir.value, props.root))
const crumb = computed(() => (file.value ? relativePath(props.root, file.value) : relativePath(props.root, dir.value) || '.'))
const lineCount = computed(() => (content.data.value?.text ? content.data.value.text.replace(/\r\n/g, '\n').split('\n').length : 0))

watch(
  dir,
  () => {
    void list.reload()
  },
  { immediate: true },
)

watch(
  file,
  (path) => {
    emit('opened', path)
    content.data.value = null
    if (path) void content.reload()
  },
  { immediate: true },
)

function up(): void {
  if (file.value) {
    file.value = null
    return
  }
  const trimmed = dir.value.replace(/[\\/]+$/, '')
  const cut = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  if (cut > 0) dir.value = trimmed.slice(0, cut)
}
</script>

<template>
  <div class="files" data-testid="viewer-files">
    <div class="path">
      <button v-if="file || !atRoot" type="button" class="up" :aria-label="t('common.back')" data-testid="files-up" @click="up"><ChevronLeft :size="20" :stroke-width="2" aria-hidden="true" /></button>
      <span class="crumb mono" data-testid="files-path">{{ crumb }}</span>
    </div>
    <template v-if="file">
      <AsyncState :blank="content.data.value === null" :error="content.error.value" :empty="content.data.value !== null && content.data.value.kind !== 'text'" :empty-text="t(`viewer.kind.${content.data.value?.kind ?? 'missing'}`)" @retry="content.reload">
        <div class="scroll"><CodeLines v-if="content.data.value?.text !== null && content.data.value" :text="content.data.value.text ?? ''" /></div>
        <footer v-if="content.data.value" class="foot mono" data-testid="file-foot">{{ t('viewer.fileFoot', { n: lineCount, size: sizeText(content.data.value.size) }) }}</footer>
      </AsyncState>
    </template>
    <AsyncState v-else :blank="list.data.value === null" :error="list.error.value" :empty="list.data.value !== null && list.data.value.length === 0" :empty-text="t('viewer.emptyDir')" @retry="list.reload">
      <ul class="list" data-testid="files-list">
        <li v-for="entry in list.data.value ?? []" :key="entry.path">
          <button type="button" class="entry" :data-testid="`entry-${entry.name}`" @click="entry.directory ? (dir = entry.path) : (file = entry.path)">
            <component :is="entry.directory ? Folder : File" :size="20" :stroke-width="2" aria-hidden="true" />
            <span class="name">{{ entry.name }}</span>
          </button>
        </li>
      </ul>
    </AsyncState>
  </div>
</template>

<style scoped>
.files {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
}

.path {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 36px;
  padding: 0 16px;
  flex: none;
}

.up {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  margin-left: -10px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
}

.crumb {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--ccm-term-bg);
}

.foot {
  flex: none;
  padding: 10px 16px calc(10px + env(safe-area-inset-bottom));
  border-top: 1px solid var(--ccm-line);
  font-size: 0.8125rem;
  color: var(--ccm-ink-muted);
}

.list {
  flex: 1;
  margin: 0;
  padding: 0 8px;
  overflow-y: auto;
  list-style: none;
}

.entry {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 44px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  text-align: left;
  color: var(--ccm-ink);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
