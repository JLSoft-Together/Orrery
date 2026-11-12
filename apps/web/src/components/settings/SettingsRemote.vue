<script setup lang="ts">
import { computed, onMounted, ref, type Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { CircleAlert, CircleCheck, CircleOff, LoaderCircle, Plus, Smartphone, SmartphoneNfc } from 'lucide-vue-next'
import type { RemoteStatus } from '@ccm/shared'
import { activeDevices } from '../../lib/remote'
import { hasPushSetup, hasRelaySetup } from '../../composables/useRemoteSetupIpc'
import { useRemoteStore } from '../../stores/remote'
import ToggleSwitch from '../ToggleSwitch.vue'
import RemoteDeviceRow from './remote/RemoteDeviceRow.vue'
import RemoteLogList from './remote/RemoteLogList.vue'
import RemotePairPanel from './remote/RemotePairPanel.vue'
import RemotePushSetup from './remote/RemotePushSetup.vue'
import RemoteRelaySetup from './remote/RemoteRelaySetup.vue'
import SettingEmbed from './SettingEmbed.vue'
import SettingSection from './SettingSection.vue'

const { t } = useI18n()
const remote = useRemoteStore()

const pairing = ref(false)
const askAll = ref(false)
const busy = ref(false)

const status = computed(() => remote.status)
const enabled = computed(() => !!status.value?.enabled)
const devices = computed(() => activeDevices(remote.devices))
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

const toggle = computed({
  get: () => enabled.value,
  set: (on: boolean) => {
    if (!on) pairing.value = false
    void remote.setEnabled(on)
  },
})
const awake = computed({
  get: () => status.value?.keepAwake ?? true,
  set: (on: boolean) => void remote.setKeepAwake(on),
})

async function revoke(deviceId: string): Promise<void> {
  await remote.revoke(deviceId)
}

async function renameDevice(deviceId: string, name: string): Promise<void> {
  await remote.renameDevice(deviceId, name)
}

async function revokeAll(): Promise<void> {
  busy.value = true
  await remote.revokeAll()
  busy.value = false
  askAll.value = false
}

onMounted(() => void remote.start())
</script>

<template>
  <div data-settings-remote>
    <SettingEmbed id="remote">
      <SettingSection id="remote-main" :title="t('remoteV97.section.main')">
        <ToggleSwitch v-model="toggle" class="px-3" :label="t('remoteV97.enable')" :hint="t('remoteV97.enableHint')" data-remote-enable />
        <Transition name="ccm-fade">
          <div v-if="enabled" class="border-t border-line">
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
          </div>
        </Transition>
      </SettingSection>
    </SettingEmbed>

    <SettingEmbed id="remoteAdd">
      <SettingSection id="remote-add" :title="t('remoteV97.add.title')">
        <Transition name="ccm-fade" mode="out-in">
          <RemotePairPanel v-if="pairing && enabled" key="pair" @close="pairing = false" />
          <div v-else key="start" class="flex items-center gap-3 px-3 py-2.5">
            <SmartphoneNfc :size="18" class="shrink-0 text-ink-muted" aria-hidden="true" />
            <p class="min-w-0 flex-1 text-xs text-ink-muted">{{ enabled ? t('remoteV97.add.hint') : t('remoteV97.add.needsOn') }}</p>
            <button type="button" class="ccm-btn ccm-btn-primary shrink-0" :disabled="!enabled" data-remote-add @click="pairing = true">
              <Plus :size="14" aria-hidden="true" />
              {{ t('remoteV97.add.button') }}
            </button>
          </div>
        </Transition>
      </SettingSection>
    </SettingEmbed>

    <SettingEmbed id="remoteDevices">
      <SettingSection id="remote-devices" :title="t('remoteV97.devices.title')">
        <TransitionGroup v-if="devices.length" tag="div" name="ccm-list" class="ccm-list-inflow relative divide-y divide-line" data-remote-devices>
          <RemoteDeviceRow v-for="d in devices" :key="d.deviceId" :device="d" @revoke="revoke" @rename="renameDevice" />
        </TransitionGroup>
        <div v-else class="flex items-center gap-3 px-3 py-4 text-sm text-ink-muted" role="status" data-remote-empty>
          <Smartphone :size="18" class="shrink-0" aria-hidden="true" />
          <span>{{ t('remoteV97.devices.empty') }}</span>
        </div>
        <template #after>
          <Transition name="ccm-fade" mode="out-in">
            <div v-if="devices.length && !askAll" key="all" class="mt-2 flex justify-end px-3">
              <button type="button" class="ccm-btn ccm-btn-danger" data-remote-revoke-all @click="askAll = true">{{ t('remoteV97.devices.revokeAll') }}</button>
            </div>
            <div v-else-if="devices.length" key="ask" class="mt-2 flex flex-wrap items-center justify-end gap-2 px-3" role="group" :aria-label="t('remoteV97.devices.revokeAllAsk')">
              <span class="min-w-0 text-xs text-ink-muted">{{ t('remoteV97.devices.revokeAllAsk') }}</span>
              <button type="button" class="ccm-btn ccm-btn-danger" :disabled="busy" data-remote-revoke-all-yes @click="revokeAll">{{ t('remoteV97.devices.revokeAllYes') }}</button>
              <button type="button" class="ccm-btn" :disabled="busy" @click="askAll = false">{{ t('remoteV97.cancel') }}</button>
            </div>
          </Transition>
        </template>
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

    <SettingEmbed id="remoteLog">
      <SettingSection id="remote-log" :title="t('remoteV97.log.title')">
        <RemoteLogList />
      </SettingSection>
    </SettingEmbed>
  </div>
</template>
