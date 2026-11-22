<script setup lang="ts">
import { hasAgentData } from '../lib/agentData'
import { hasBackup } from '../lib/backup'
import { hasDiskMap } from '@features/disk'
import { useUiGenStore } from '../../next/stores/uiGen'
import { useUiStore } from '../../../core/stores/ui'
import AgentDataSettings from './AgentDataSettings.vue'
import BackupSettings from './BackupSettings.vue'
import SettingEmbed from './SettingEmbed.vue'
import { SettingsDiskMap } from '@features/disk'

const ui = useUiStore()
const uiGen = useUiGenStore()
</script>

<template>
  <div>
    <SettingEmbed v-if="hasAgentData" id="agentData"><AgentDataSettings :open="ui.settingsOpen" /></SettingEmbed>
    <SettingEmbed v-if="hasBackup" id="backup"><BackupSettings /></SettingEmbed>
    <SettingEmbed v-if="hasDiskMap && !uiGen.isNext" id="diskMap"><SettingsDiskMap /></SettingEmbed>
  </div>
</template>
