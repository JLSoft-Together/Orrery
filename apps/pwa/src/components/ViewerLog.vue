<script setup lang="ts">
import { GitCommitHorizontal } from 'lucide-vue-next'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAsync } from '../lib/useAsync'
import { viewerData, type Commander } from '../lib/viewerApi'
import { parseLog, type LogCommit } from '../lib/viewerModel'
import AsyncState from './AsyncState.vue'

const props = defineProps<{ command: Commander; root: string; refresh: number }>()
const { t, locale } = useI18n()
const extra = ref<LogCommit[]>([])
const more = ref(false)
const busy = ref(false)

function query(skip: number): Record<string, unknown> {
  return { root: props.root, query: { branch: null, notRef: null, author: null, since: null, until: null, text: null, regex: false, matchCase: false, paths: [], skip } }
}

const first = useAsync(async () => {
  const page = parseLog(await viewerData(props.command, 'log.page', query(0)))
  extra.value = []
  more.value = page.more
  return page.commits
})

async function loadMore(): Promise<void> {
  busy.value = true
  try {
    const skip = (first.data.value?.length ?? 0) + extra.value.length
    const page = parseLog(await viewerData(props.command, 'log.page', query(skip)))
    extra.value = [...extra.value, ...page.commits]
    more.value = page.more
  } catch {
    more.value = false
  } finally {
    busy.value = false
  }
}

function when(seconds: number): string {
  return seconds ? new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(seconds * 1000) : ''
}

watch(
  () => props.refresh,
  () => void first.reload(),
)
</script>

<template>
  <div class="log" data-testid="viewer-log">
    <AsyncState :blank="first.data.value === null" :error="first.error.value" :empty="first.data.value !== null && first.data.value.length === 0" :empty-text="t('viewer.noCommits')" @retry="first.reload">
      <ul class="list">
        <li v-for="c in [...(first.data.value ?? []), ...extra]" :key="c.hash" class="commit" :data-testid="`commit-${c.short}`">
          <GitCommitHorizontal :size="20" :stroke-width="2" class="ico" aria-hidden="true" />
          <span class="body">
            <span class="subject">{{ c.subject }}</span>
            <span class="meta mono">{{ c.short }} · {{ c.author }} · {{ when(c.time) }}</span>
            <span v-if="c.refs.length" class="refs"><span v-for="r in c.refs" :key="r" class="ref mono">{{ r }}</span></span>
          </span>
        </li>
      </ul>
      <button v-if="more" type="button" class="btn more" :disabled="busy" data-testid="log-more" @click="loadMore">{{ t('viewer.more') }}</button>
    </AsyncState>
  </div>
</template>

<style scoped>
.log {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  overflow-y: auto;
}

.list {
  margin: 0;
  padding: 0 16px;
  list-style: none;
}

.commit {
  display: flex;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--ccm-line);
}

.ico {
  flex: none;
  margin-top: 2px;
  color: var(--ccm-ink-faint);
}

.body {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.subject {
  font-weight: 600;
  overflow-wrap: anywhere;
}

.meta {
  font-size: 0.75rem;
  color: var(--ccm-ink-muted);
  overflow-wrap: anywhere;
}

.refs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.ref {
  padding: 1px 8px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  font-size: 0.6875rem;
  color: var(--ccm-ink-muted);
}

.more {
  margin: 12px 16px;
}
</style>
