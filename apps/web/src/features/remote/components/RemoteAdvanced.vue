<script setup lang="ts">
import { computed, onMounted, ref, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronDown, CircleAlert, CircleCheck, CircleOff, LoaderCircle, Settings2 } from 'lucide-vue-next'
import type { RemoteStatus } from '@ccm/shared'
import { SettingEmbed, SettingSection } from '@features/settings'
import ToggleSwitch from '../../../core/components/ToggleSwitch.vue'
import { hasPushSetup, hasRelaySetup } from '../composables/useRemoteSetupIpc'
import { useRemoteGuard } from '../composables/useRemoteGuard'
import { hasRemoteGuard } from '../lib/remoteGuard'
import { useRemoteStore } from '../stores/remote'
import RemoteAccessLog from './RemoteAccessLog.vue'
import RemoteGuardBlocked from './RemoteGuardBlocked.vue'
import RemoteGuardTrust from './RemoteGuardTrust.vue'
import RemoteInstallCard from './RemoteInstallCard.vue'
import RemoteLogList from './RemoteLogList.vue'
import RemotePushSetup from './RemotePushSetup.vue'
import RemoteRelaySetup from './RemoteRelaySetup.vue'

const { t } = useI18n()
const remote = useRemoteStore()
const guard = useRemoteGuard()
const open = ref(false)

const status = computed(() => remote.status)
const enabled = computed(() => !!status.value?.enabled)
const address = computed(() => (status.value?.addresses ?? []).map((a) => `${a}:${status.value?.port}`).join(', '))

type Tone = { icon: Component; cls: string; spin?: boolean }
const LISTEN_TONE: Record<'on' | 'off', Tone> = {
  on: { icon: CircleCheck, cls: 'text-success' },
  off: { icon: CircleAlert, cls: 'text-warning' },
}
const RELAY_TONE: Record<RemoteStatus['relay'], Tone> = {
  off: { icon: CircleOff, cls: 'text-ink-muted' },
  connecting: { icon: LoaderCircle, cls: 'text-ink-muted', spin: true },
  online: { icon: CircleCheck, cls: 'text-success' },
  error: { icon: CircleAlert, cls: 'text-warning' },
}
const listen = computed(() => LISTEN_TONE[status.value?.listening ? 'on' : 'off'])
const relay = computed(() => RELAY_TONE[status.value?.relay ?? 'off'])

const awake = computed({
  get: () => status.value?.keepAwake ?? true,
  set: (on: boolean) => void remote.setKeepAwake(on),
})

onMounted(() => {
  if (hasRemoteGuard) void guard.refresh()
})
</script>

<template>
  <div class="mt-4" data-remote-advanced>
    <button
      type="button"
      class="ccm-group flex w-full items-center gap-2.5 px-3 py-2.5 text-left"
      :aria-expanded="open"
      aria-controls="remote-advanced-body"
      data-remote-more
      @click="open = !open"
    >
      <Settings2 :size="15" class="shrink-0 text-ink-muted" aria-hidden="true" />
      <span class="min-w-0 flex-1 text-sm font-semibold">{{ t('v113Connect.advanced.title') }}</span>
      <span class="hidden min-w-0 truncate text-xs text-ink-muted sm:block">{{ t('v113Connect.advanced.tag') }}</span>
      <ChevronDown :size="15" class="shrink-0 text-ink-muted transition-transform duration-(--ccm-dur-base)" :class="open ? 'rotate-180' : ''" aria-hidden="true" />
    </button>

    <Transition name="ccm-fade">
      <div v-show="open" id="remote-advanced-body" class="mt-4" data-remote-advanced-body>
        <SettingSection id="remote-port">
          <div class="flex items-start gap-2 px-3 py-2 text-sm" data-remote-listener :data-state="status?.listening ? 'on' : 'off'">
            <component :is="listen.icon" :size="16" class="mt-0.5 shrink-0" :class="listen.cls" aria-hidden="true" />
            <div class="min-w-0 flex-1">
              <p>{{ status?.listening ? t('remoteV97.listener.on') : t('remoteV97.listener.off') }}</p>
              <p v-if="status?.listening && address" class="mt-0.5 font-mono text-2xs break-all text-ink-muted" data-remote-address>{{ address }}</p>
            </div>
          </div>
          <div class="flex items-center gap-2 border-t border-line px-3 py-2 text-sm" data-remote-relay :data-state="status?.relay">
            <component :is="relay.icon" :size="16" class="shrink-0" :class="[relay.cls, relay.spin ? 'animate-spin' : '']" aria-hidden="true" />
            <p class="min-w-0 flex-1">{{ t(`remoteV97.relay.${status?.relay ?? 'off'}`) }}</p>
          </div>
          <p class="border-t border-line px-3 py-2 text-2xs text-ink-faint" data-remote-firewall>{{ t('remoteV97.firewall', { port: status?.port ?? '' }) }}</p>
        </SettingSection>

        <SettingEmbed id="remoteInstall">
          <SettingSection id="remote-install" :title="t('v106Install.title')">
            <RemoteInstallCard />
          </SettingSection>
        </SettingEmbed>

        <SettingEmbed v-if="hasRelaySetup" id="remoteRelay">
          <SettingSection id="remote-relay" :title="t('remoteV97.relaySetup.title')">
            <RemoteRelaySetup />
          </SettingSection>
        </SettingEmbed>

        <SettingEmbed v-if="hasPushSetup" id="remotePush">
          <SettingSection id="remote-push" :title="t('remoteV97.pushSetup.title')">
            <RemotePushSetup />
          </SettingSection>
        </SettingEmbed>

        <SettingEmbed id="remoteAwake">
          <SettingSection id="remote-awake">
            <ToggleSwitch v-model="awake" class="px-3" :label="t('remoteV97.awake')" :hint="t('remoteV97.awakeHint')" :disabled="!enabled" data-remote-awake />
          </SettingSection>
        </SettingEmbed>

        <template v-if="hasRemoteGuard">
          <SettingEmbed id="remoteBlocked">
            <SettingSection id="remote-blocked" :title="t('remoteV98.guard.blockedTitle')">
              <RemoteGuardBlocked :blocked="remote.blocked" :busy="guard.busy.value" :error="guard.error.value" @run="guard.run" />
            </SettingSection>
          </SettingEmbed>

          <SettingEmbed id="remoteTrusted">
            <SettingSection id="remote-trusted" :title="t('remoteV98.guard.trustedTitle')">
              <RemoteGuardTrust :trusted="guard.state.value.trusted" :busy="guard.busy.value" @run="guard.run" />
            </SettingSection>
          </SettingEmbed>

          <SettingEmbed id="remoteAccess">
            <SettingSection id="remote-access" :title="t('remoteV98.guard.logTitle')">
              <RemoteAccessLog :log="guard.state.value.log" :busy="guard.busy.value" @refresh="guard.refresh" />
            </SettingSection>
          </SettingEmbed>
        </template>

        <SettingEmbed id="remoteLog">
          <SettingSection id="remote-log" :title="t('remoteV97.log.title')">
            <RemoteLogList />
          </SettingSection>
        </SettingEmbed>
      </div>
    </Transition>
  </div>
</template>
