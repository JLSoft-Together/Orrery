<script setup lang="ts">
defineProps<{ modelValue: boolean; label: string; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <button type="button" role="switch" class="switch" :class="{ on: modelValue }" :aria-checked="modelValue" :aria-label="label" :disabled="disabled" @click="emit('update:modelValue', !modelValue)">
    <span class="thumb" />
  </button>
</template>

<style scoped>
.switch {
  position: relative;
  flex: none;
  width: 52px;
  height: 32px;
  padding: 0;
  border: 0;
  border-radius: 16px;
  background: var(--ccm-line-strong);
  transition: background-color 200ms var(--ease-out);
}

.switch::after {
  content: '';
  position: absolute;
  inset: -6px -0px;
}

.switch.on {
  background: var(--ccm-accent);
}

.switch:disabled {
  opacity: 0.45;
}

.thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: #ffffff;
  transition: transform 200ms var(--ease-out);
}

.on .thumb {
  transform: translateX(20px);
}
</style>
