<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { hasDiskMap } from '../../lib/diskMap'
import { useUiGenStore } from '../../stores/uiGen'
import SettingPart from './SettingPart.vue'
import SettingsAccess from './SettingsAccess.vue'
import SettingsData from './SettingsData.vue'
import SettingsDisk from './SettingsDisk.vue'

const { t } = useI18n()
const uiGen = useUiGenStore()
</script>

<template>
  <div>
    <SettingPart id="data" :title="t('v107gon.settings.parts.data')"><SettingsData /></SettingPart>
    <SettingPart v-if="uiGen.isNext && hasDiskMap" id="disk" :title="t('v107gon.settings.parts.disk')"><SettingsDisk /></SettingPart>
    <SettingPart id="access" :title="t('v107gon.settings.parts.access')"><SettingsAccess /></SettingPart>
  </div>
</template>
