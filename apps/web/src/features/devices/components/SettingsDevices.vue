<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { MonitorSmartphone, Plus, Smartphone, Wifi } from 'lucide-vue-next'
import { SettingEmbed } from '@features/settings'
import EmptyState from '../../../core/components/EmptyState.vue'
import { useUiStore } from '../../../core/stores/ui'
import { useSettingsDevices } from '../composables/useSettingsDevices'
import { installedVersion } from '../lib/deviceActions'
import { hasDevices } from '../lib/devices'
import type { DeviceRow, RowFilter } from '../lib/mergeDevices'
import DeviceCaptureDialog from './DeviceCaptureDialog.vue'
import SettingsDeviceRow from './SettingsDeviceRow.vue'

const { t } = useI18n()
const ui = useUiStore()
const d = useSettingsDevices()

const FILTERS: RowFilter[] = ['all', 'remote', 'adb', 'emulator']
const captureFor = ref<DeviceRow | null>(null)

const total = computed(() => d.rows.value.length)

const versionOf = (row: DeviceRow): string => installedVersion(row, row.adb ? d.pkg.value[row.adb.id] : undefined)

function installedOf(row: DeviceRow): boolean | null {
  const pkg = row.adb ? d.pkg.value[row.adb.id] : undefined
  if (pkg && pkg.ok) return pkg.installed
  return row.remote ? true : null
}

function connectPhone(): void {
  ui.settingsPage = 'remote'
}

function pairAdb(): void {
  ui.settingsOpen = false
  ui.view = 'devices'
}
</script>

<template>
  <div data-settings-devices>
    <SettingEmbed id="devicesList">
      <div class="flex flex-wrap items-start gap-3">
        <div class="min-w-0 flex-1 basis-52">
          <h3 class="text-sm font-semibold text-ink">{{ t('v113Devices.title') }}</h3>
          <p class="mt-0.5 text-xs text-ink-muted" data-devices-count>{{ t('v113Devices.subtitle', { n: total }, total) }}</p>
        </div>
        <button type="button" class="ccm-btn ccm-btn-primary" data-devices-connect @click="connectPhone">
          <Plus :size="14" aria-hidden="true" />
          {{ t('v113Devices.connect') }}
        </button>
      </div>

      <SettingEmbed id="devicesAdb">
        <Transition name="ccm-fade" mode="out-in">
          <div v-if="total" key="list">
            <div class="ccm-seg mb-2 inline-flex max-w-full gap-0.5 overflow-x-auto rounded-control p-0.5" role="group" :aria-label="t('v113Devices.filterLabel')" data-devices-filter>
              <button
                v-for="f in FILTERS"
                :key="f"
                type="button"
                :aria-pressed="d.filter.value === f"
                :data-devices-filter-value="f"
                class="inline-flex h-6 shrink-0 cursor-pointer items-center gap-1.5 rounded-inner px-2 text-2xs font-medium whitespace-nowrap transition-colors duration-(--ccm-dur-fast) ease-out-quint"
                :class="d.filter.value === f ? 'ccm-seg-thumb ccm-seg-on' : 'text-ink-muted hover:text-ink'"
                @click="d.filter.value = f"
              >
                {{ t(`v113Devices.filter.${f}`) }}
                <span class="tabular text-ink-faint">{{ d.counts.value[f] }}</span>
              </button>
            </div>
            <div class="ccm-group overflow-hidden">
              <TransitionGroup v-if="d.shown.value.length" tag="div" name="ccm-list" class="ccm-list-inflow relative divide-y divide-line" :aria-label="t('v113Devices.listLabel')" data-devices-list>
                <SettingsDeviceRow
                  v-for="row in d.shown.value"
                  :key="row.key"
                  :row="row"
                  :open="d.openKey.value === row.key"
                  :action="d.actionOf(row)"
                  :busy="d.busy.value[row.key]"
                  :version="versionOf(row)"
                  :installed="installedOf(row)"
                  :desktop-version="d.desktopVersion.value"
                  :latency="row.remote ? d.latency.value[row.remote.deviceId] : undefined"
                  @toggle="d.toggle(row.key)"
                  @run="(a) => d.run(row, a)"
                  @test="d.sendTest(row)"
                  @open-app="d.openApp(row)"
                  @capture="captureFor = row"
                  @push="(on) => d.pushTo(row, on)"
                  @rename="(name) => d.rename(row, name)"
                  @revoke="d.revoke(row)"
                  @undo="d.undoRevoke(row)"
                  @merge="(yes) => row.suggestion && d.mergeWith(row.suggestion.key, yes)"
                  @unmerge="d.unmerge(row)"
                />
              </TransitionGroup>
              <p v-else class="flex items-center gap-2 px-3 py-4 text-sm text-ink-muted" role="status" data-devices-none>
                <Smartphone :size="16" class="shrink-0" aria-hidden="true" />
                {{ t('v113Devices.noneInGroup') }}
              </p>
            </div>
          </div>
          <div v-else key="empty" class="ccm-group flex justify-center overflow-hidden" data-devices-empty>
            <EmptyState :title="t('v113Devices.empty.title')" :hint="t('v113Devices.empty.hint')" :icon="MonitorSmartphone">
              <template #actions>
                <button v-if="hasDevices" type="button" class="ccm-btn" data-devices-pair-adb @click="pairAdb">
                  <Wifi :size="13" aria-hidden="true" />
                  {{ t('v113Devices.empty.pairAdb') }}
                </button>
                <button type="button" class="ccm-btn ccm-btn-primary" data-devices-empty-connect @click="connectPhone">
                  <Smartphone :size="13" aria-hidden="true" />
                  {{ t('v113Devices.empty.connect') }}
                </button>
              </template>
            </EmptyState>
          </div>
        </Transition>
      </SettingEmbed>
    </SettingEmbed>

    <DeviceCaptureDialog :open="!!captureFor" :device-id="captureFor?.adb?.id ?? null" :device-name="captureFor?.name ?? ''" @close="captureFor = null" />
  </div>
</template>
