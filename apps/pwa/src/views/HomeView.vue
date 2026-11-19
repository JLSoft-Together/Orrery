<script setup lang="ts">
import { Ban, Laptop, Plus, QrCode, SlidersHorizontal, Trash2 } from 'lucide-vue-next'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import LinkChip from '../components/LinkChip.vue'
import MachineCard from '../components/MachineCard.vue'
import OrbitArt from '../components/OrbitArt.vue'
import { askConfirm } from '../lib/confirm'
import { useNow } from '../lib/useNow'
import { groupMachines } from '../lib/machineGroups'
import { navigate } from '../router'
import { useHubStore } from '../stores/hub'
import { usePrefsStore } from '../stores/prefs'

const { t, locale } = useI18n()
const hub = useHubStore()
const prefs = usePrefsStore()
const now = useNow()

const groups = computed(() => groupMachines(hub.machines, new Set(hub.machines.filter((m) => hub.linkOf(m.id).status === 'ready').map((m) => m.id))))
const connected = computed(() => groups.value.primary.filter((m) => hub.linkOf(m.id).status === 'ready'))
const others = computed(() => groups.value.primary.filter((m) => hub.linkOf(m.id).status !== 'ready'))
const total = computed(() => hub.machines.length)

function date(ms: number): string {
  return new Intl.DateTimeFormat(locale.value, { day: '2-digit', month: '2-digit' }).format(ms)
}

function openMachine(id: string): void {
  navigate({ name: 'monitor', machine: id })
}

async function cleanOld(): Promise<void> {
  const old = groups.value.duplicates
  const ok = await askConfirm({ title: t('home.cleanTitle'), body: t('home.cleanBody', { n: old.length }), ok: t('machines.forget'), cancel: t('common.cancel'), danger: true })
  if (!ok) return
  for (const machine of old) {
    await hub.forget(machine.id)
    await prefs.dropMachine(machine.id)
  }
}
</script>

<template>
  <section class="page" data-testid="machines-view">
    <header class="top glass-bar">
      <span class="logo" aria-hidden="true">O</span>
      <h1 class="title">{{ t('machines.title') }}</h1>
      <button type="button" class="icon-btn" :aria-label="t('tabs.settings')" data-testid="home-settings" @click="navigate({ name: 'settings' })">
        <SlidersHorizontal :size="22" :stroke-width="2" aria-hidden="true" />
      </button>
      <button v-if="total > 0" type="button" class="pair-btn" :aria-label="t('home.pair')" data-testid="pair-fab" @click="navigate({ name: 'pair' })">
        <Plus :size="22" :stroke-width="2.4" aria-hidden="true" />
      </button>
    </header>

    <div v-if="total === 0" class="empty" data-testid="machines-empty">
      <OrbitArt />
      <h2>{{ t('machines.emptyTitle') }}</h2>
      <p>{{ t('machines.emptyBody') }}</p>
      <button type="button" class="btn btn-primary cta" data-testid="empty-pair" @click="navigate({ name: 'pair' })"><QrCode :size="20" :stroke-width="2" aria-hidden="true" />{{ t('home.pairNew') }}</button>
      <button type="button" class="link" data-testid="empty-manual" @click="navigate({ name: 'pair' })">{{ t('pair.manual') }}</button>
    </div>

    <template v-else>
      <h2 v-if="connected.length" class="label group">{{ t('machines.connected', { n: connected.length }) }}</h2>
      <TransitionGroup name="mlist" tag="div" class="stack">
        <MachineCard v-for="m in connected" :key="m.id" :machine="m" :link="hub.linkOf(m.id)" :view="hub.viewOf(m.id)" :rtt="hub.rtt[m.id]" :bell="prefs.bell(m.id)" :now="now" @open="openMachine(m.id)" @policy="hub.setPolicy(m.id, $event)" @bell="prefs.setBell(m.id, $event)" />
      </TransitionGroup>
      <h2 v-if="others.length" class="label group">{{ t('machines.others', { n: others.length }) }}</h2>
      <TransitionGroup name="mlist" tag="div" class="stack">
        <MachineCard v-for="m in others" :key="m.id" :machine="m" :link="hub.linkOf(m.id)" :view="hub.viewOf(m.id)" :rtt="hub.rtt[m.id]" :bell="prefs.bell(m.id)" :now="now" @open="openMachine(m.id)" @policy="hub.setPolicy(m.id, $event)" @bell="prefs.setBell(m.id, $event)" />
      </TransitionGroup>
      <template v-if="groups.duplicates.length">
        <h2 class="label group">{{ t('home.duplicates', { n: groups.duplicates.length }) }}</h2>
        <div class="card dups" data-testid="machines-duplicates">
          <div v-for="m in groups.duplicates" :key="m.id" class="dup">
            <span class="avatar" aria-hidden="true"><Laptop :size="18" :stroke-width="2" /></span>
            <span class="dup-text">
              <span class="dup-name">{{ m.alias ?? m.name }}</span>
              <span class="dup-sub">{{ t('home.pairedOn', { date: date(m.pairedAt) }) }}</span>
            </span>
            <span v-if="m.revoked" class="revoked"><Ban :size="14" :stroke-width="2.2" aria-hidden="true" />{{ t('link.revoked') }}</span>
            <LinkChip v-else :link="hub.linkOf(m.id)" :rtt="hub.rtt[m.id]" />
          </div>
          <button type="button" class="clean" data-testid="clean-old" @click="cleanOld"><Trash2 :size="20" :stroke-width="2" aria-hidden="true" />{{ t('home.cleanOld', { n: groups.duplicates.length }) }}</button>
        </div>
      </template>
    </template>
  </section>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 16px 16px;
  min-height: 100%;
}

.top {
  position: sticky;
  top: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 64px;
  margin: 0 -16px;
  padding: 0 16px;
  border-bottom: 1px solid var(--ccm-glass-edge);
}

.logo {
  display: grid;
  place-items: center;
  flex: none;
  width: 32px;
  height: 32px;
  border-radius: 10px;
  background-color: var(--ccm-accent);
  background-image: var(--ccm-accent-grad);
  color: var(--ccm-on-accent);
  font-weight: 800;
}

.title {
  flex: 1;
  min-width: 0;
  margin: 0 0 0 4px;
  font-size: 1.375rem;
  font-weight: 800;
  letter-spacing: -0.01em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-btn {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-ink);
}

.pair-btn {
  display: grid;
  place-items: center;
  flex: none;
  width: 44px;
  height: 44px;
  border: 0;
  border-radius: 50%;
  background-color: var(--ccm-accent);
  background-image: var(--ccm-accent-grad);
  box-shadow: var(--ccm-shadow-accent);
  color: var(--ccm-on-accent);
  transition: transform var(--ccm-dur-press) var(--ccm-ease-standard);
}

.pair-btn:active {
  transform: scale(var(--ccm-motion-scale, 0.96));
}

.stack {
  display: flex;
  flex-direction: column;
  gap: 12px;
  position: relative;
}

.group {
  margin: 8px 0 0 4px;
  font-size: 0.8125rem;
  font-weight: 700;
  color: var(--ccm-ink-muted);
}

.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px 8px 64px;
  text-align: center;
}

.empty h2 {
  margin: 8px 0 0;
  font-size: 1.125rem;
}

.empty p {
  margin: 0;
  max-width: 28ch;
  color: var(--ccm-ink-muted);
}

.cta {
  margin-top: 8px;
}

.link {
  min-height: 44px;
  border: 0;
  background: transparent;
  color: var(--ccm-accent-ink);
  font-weight: 700;
}

.dups {
  padding: 0 14px;
}

.dup {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  padding: 8px 0;
  border-bottom: 1px solid var(--ccm-line);
}

.avatar {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-control);
  background: var(--ccm-raised);
  color: var(--ccm-ink-muted);
  flex: none;
}

.dup-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.dup-name {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dup-sub {
  font-size: 0.75rem;
  color: var(--ccm-ink-muted);
}

.revoked {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: var(--radius-chip);
  border: 1px solid var(--ccm-line-strong);
  color: var(--ccm-idle);
  font-size: 0.8125rem;
  font-weight: 600;
}

.clean {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 48px;
  border: 0;
  background: transparent;
  color: var(--ccm-accent-ink);
  font-weight: 700;
}

.mlist-enter-active,
.mlist-leave-active {
  transition:
    opacity var(--ccm-dur-slow) var(--ccm-ease-enter),
    transform var(--ccm-dur-slow) var(--ccm-ease-enter);
}

.mlist-move {
  transition: transform var(--ccm-dur-slow) var(--ccm-ease-enter);
}

.mlist-enter-from,
.mlist-leave-to {
  opacity: 0;
  transform: translateY(8px) scale(0.98);
}

.mlist-leave-active {
  position: absolute;
  left: 0;
  right: 0;
}
</style>
