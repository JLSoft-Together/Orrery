<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Smartphone } from 'lucide-vue-next'
import { SettingEmbed, SettingSection } from '@features/settings'
import { isWizardOpen } from '../lib/connectFlow'
import { activeDevices } from '../lib/remote'
import { useRemoteStore } from '../stores/remote'
import PhoneConnectWizard from './connect/PhoneConnectWizard.vue'
import { useConnectWizard } from './connect/useConnectWizard'
import RemoteAdvanced from './RemoteAdvanced.vue'
import RemoteDeviceRow from './RemoteDeviceRow.vue'
import RemoteHeroCard from './RemoteHeroCard.vue'
import RemotePushBell from './RemotePushBell.vue'

const { t } = useI18n()
const remote = useRemoteStore()
const wizard = useConnectWizard()

const askAll = ref(false)
const busy = ref(false)

const enabled = computed(() => !!remote.status?.enabled)
const devices = computed(() => activeDevices(remote.devices))
const open = computed(() => isWizardOpen(wizard.name.value))

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

watch(
  () => remote.pairWanted && enabled.value,
  (want) => {
    if (!want) return
    remote.pairWanted = false
    wizard.start()
    wizard.dispatch({ type: 'alreadyInstalled' })
  },
  { immediate: true },
)

onMounted(() => {
  void remote.start()
})
</script>

<template>
  <div data-settings-remote>
    <SettingEmbed id="remote">
      <SettingEmbed id="remoteAdd">
        <SettingEmbed id="remoteConnect">
          <SettingSection id="remote-main" :title="t('v113Connect.title')">
            <Transition name="ccm-fade" mode="out-in">
              <PhoneConnectWizard v-if="open" key="wizard" :wizard="wizard" />
              <RemoteHeroCard v-else key="hero" :wizard="wizard" />
            </Transition>
          </SettingSection>
        </SettingEmbed>
      </SettingEmbed>
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
          <RemotePushBell v-if="devices.length" :devices="devices" @toggle="(id, on) => void remote.setDevicePush(id, on)" />
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

    <RemoteAdvanced />
  </div>
</template>
