<script setup lang="ts">
import { CameraOff } from 'lucide-vue-next'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { decodeFrame } from '../lib/qrDecode'

const emit = defineEmits<{ scan: [text: string]; manual: [] }>()
const { t } = useI18n()
const video = ref<HTMLVideoElement | null>(null)
const state = ref<'starting' | 'live' | 'denied' | 'unsupported'>('starting')
let stream: MediaStream | null = null
let frame = 0
let timer: ReturnType<typeof setTimeout> | null = null
let stopped = false
const canvas = document.createElement('canvas')

function tick(): void {
  if (stopped) return
  const el = video.value
  if (el && el.videoWidth > 0) {
    const scale = Math.min(1, 640 / el.videoWidth)
    canvas.width = Math.round(el.videoWidth * scale)
    canvas.height = Math.round(el.videoHeight * scale)
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (ctx) {
      ctx.drawImage(el, 0, 0, canvas.width, canvas.height)
      const image = ctx.getImageData(0, 0, canvas.width, canvas.height)
      const text = decodeFrame(image.data, image.width, image.height)
      if (text) {
        emit('scan', text)
        return
      }
    }
  }
  timer = setTimeout(() => {
    frame = requestAnimationFrame(tick)
  }, 120)
}

async function start(): Promise<void> {
  if (!navigator.mediaDevices?.getUserMedia) {
    state.value = 'unsupported'
    return
  }
  try {
    stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: { ideal: 'environment' } }, audio: false })
  } catch {
    state.value = 'denied'
    return
  }
  if (stopped) {
    stop()
    return
  }
  const el = video.value
  if (!el) return
  el.srcObject = stream
  try {
    await el.play()
  } catch {
    state.value = 'unsupported'
    return
  }
  state.value = 'live'
  tick()
}

function stop(): void {
  stopped = true
  if (timer) clearTimeout(timer)
  cancelAnimationFrame(frame)
  for (const track of stream?.getTracks() ?? []) track.stop()
  stream = null
}

onMounted(() => void start())
onBeforeUnmount(stop)
</script>

<template>
  <div class="scanner" :data-state="state" data-testid="qr-scanner">
    <video ref="video" class="video" muted playsinline autoplay :class="{ hidden: state !== 'live' }" />
    <div v-if="state === 'live' || state === 'starting'" class="frame" aria-hidden="true">
      <span class="corner tl" />
      <span class="corner tr" />
      <span class="corner bl" />
      <span class="corner br" />
    </div>
    <div v-if="state === 'denied' || state === 'unsupported'" class="fallback" data-testid="qr-fallback">
      <CameraOff :size="28" :stroke-width="1.8" aria-hidden="true" />
      <p class="fb-title">{{ t(state === 'denied' ? 'pair.cameraDenied' : 'pair.cameraUnsupported') }}</p>
      <p class="fb-body">{{ t('pair.cameraHint') }}</p>
      <button type="button" class="btn" data-testid="qr-manual" @click="emit('manual')">{{ t('pair.useCode') }}</button>
    </div>
  </div>
</template>

<style scoped>
.scanner {
  position: relative;
  aspect-ratio: 1;
  width: 100%;
  overflow: hidden;
  border-radius: var(--radius-card);
  background: var(--ccm-scanner);
  display: grid;
  place-items: center;
}

.video {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.video.hidden {
  visibility: hidden;
}

.frame {
  position: absolute;
  inset: 12%;
}

.corner {
  position: absolute;
  width: 36px;
  height: 36px;
  border: 0 solid var(--ccm-accent);
}

.tl {
  top: 0;
  left: 0;
  border-top-width: 5px;
  border-left-width: 5px;
  border-top-left-radius: var(--radius-card);
}

.tr {
  top: 0;
  right: 0;
  border-top-width: 5px;
  border-right-width: 5px;
  border-top-right-radius: var(--radius-card);
}

.bl {
  bottom: 0;
  left: 0;
  border-bottom-width: 5px;
  border-left-width: 5px;
  border-bottom-left-radius: var(--radius-card);
}

.br {
  bottom: 0;
  right: 0;
  border-bottom-width: 5px;
  border-right-width: 5px;
  border-bottom-right-radius: var(--radius-card);
}

.fallback {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px;
  text-align: center;
  color: #ececf1;
}

.fb-title {
  margin: 0;
  font-weight: 700;
}

.fb-body {
  margin: 0;
  font-size: 13px;
  color: #a6a6b0;
}
</style>
