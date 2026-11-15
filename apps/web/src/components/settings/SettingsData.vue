<script setup lang="ts">
import { hasAgentData } from '../../lib/agentData'
import { hasBackup } from '../../lib/backup'
import { hasDiskMap } from '../../lib/diskMap'
import { useUiStore } from '../../stores/ui'
import AgentDataSettings from '../AgentDataSettings.vue'
import BackupSettings from '../BackupSettings.vue'
import SettingEmbed from './SettingEmbed.vue'
import SettingsDiskMap from './SettingsDiskMap.vue'

const ui = useUiStore()
</script>

<template>
  <div>
    <SettingEmbed v-if="hasAgentData" id="agentData"><AgentDataSettings :open="ui.settingsOpen" /></SettingEmbed>
    <SettingEmbed v-if="hasBackup" id="backup"><BackupSettings /></SettingEmbed>
    <SettingEmbed v-if="hasDiskMap" id="diskMap"><SettingsDiskMap /></SettingEmbed>
  </div>
</template>
