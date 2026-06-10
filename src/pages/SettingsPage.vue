<script setup lang="ts">
/**
 * 设置页 — 需求 3.3.4 / DESIGN 15.6
 * 复用 KeyCaptureInput 渲染启动 / 停止热键；保存按钮对比快照，有变化才提示。
 * 阶段 12：保存时调用 update_hotkeys 注册 + 持久化。
 */
import { ref, onMounted, onBeforeUnmount, computed, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { appStore } from '../stores/appStore'
import KeyCaptureInput from '../components/KeyCaptureInput.vue'
import type { CapturedKey, HotkeyConfig, HotkeyUpdateResult } from '../types/config'

/** 已持久化快照（用于保存时对比是否真的变更） */
const persistedSnapshot = ref<HotkeyConfig>(cloneHotkeys(appStore.hotkeys))

/** v-model 绑定到 reactive store；阶段 6 暂用本地副本，避免捕获后立即影响快照对比 */
const startKey = ref<CapturedKey | null>(cloneKey(appStore.hotkeys.start))
const stopKey = ref<CapturedKey | null>(cloneKey(appStore.hotkeys.stop))

/** 保存提示文本：null 表示不显示；保存后短暂显示，失败/无变化时不显示 */
const saveMessage = ref<string | null>(null)
/** 提示文本是否为警告（橙色） */
const isWarning = ref(false)
let messageTimer: number | null = null

const isDirty = computed(() => {
  if (!startKey.value || !stopKey.value) return false
  return (
    startKey.value.keyLabel !== persistedSnapshot.value.start.keyLabel ||
    startKey.value.scanCode !== persistedSnapshot.value.start.scanCode ||
    stopKey.value.keyLabel !== persistedSnapshot.value.stop.keyLabel ||
    stopKey.value.scanCode !== persistedSnapshot.value.stop.scanCode
  )
})

function cloneKey(k: CapturedKey): CapturedKey {
  return { keyLabel: k.keyLabel, scanCode: k.scanCode }
}

function cloneHotkeys(h: HotkeyConfig): HotkeyConfig {
  return { start: cloneKey(h.start), stop: cloneKey(h.stop) }
}

function showMessage(text: string, warning: boolean = false): void {
  saveMessage.value = text
  isWarning.value = warning
  if (messageTimer !== null) {
    window.clearTimeout(messageTimer)
  }
  messageTimer = window.setTimeout(() => {
    saveMessage.value = null
    isWarning.value = false
    messageTimer = null
  }, 2000)
}

async function onSave(): Promise<void> {
  if (!startKey.value || !stopKey.value) return
  if (!isDirty.value) {
    // 无变化不提示（需求 3.3.4）
    return
  }

  try {
    // 阶段 12：调用 update_hotkeys 注册 + 持久化
    const result = await invoke<HotkeyUpdateResult>('update_hotkeys', {
      hotkeys: {
        start: cloneKey(startKey.value),
        stop: cloneKey(stopKey.value),
      },
    })

    // 无变化：不提示
    if (!result.changed) return

    // 注册失败
    if (!result.registered) {
      const msg = result.message || '热键注册失败'
      showMessage(msg, msg.includes('冲突'))
      return
    }

    // 持久化失败
    if (!result.persisted) {
      showMessage('持久化失败', false)
      return
    }

    // 注册 + 持久化都成功
    appStore.hotkeys = {
      start: cloneKey(startKey.value),
      stop: cloneKey(stopKey.value),
    }
    persistedSnapshot.value = cloneHotkeys(appStore.hotkeys)
    showMessage('已保存', false)
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : String(err)
    showMessage(`保存失败: ${errorMsg}`, false)
    console.error('Failed to update hotkeys:', err)
  }
}

// ===== 提示音录制（阶段 18） =====

type SoundTarget = 'start' | 'stop'

const MAX_REC_SECS = 5

/** 文件是否已存在 */
const startExists = ref(false)
const stopExists = ref(false)
/** 当前正在录制的目标；null 表示空闲 */
const recordingTarget = ref<SoundTarget | null>(null)
/** 录制已用时长（毫秒，前端计时仅作显示） */
const recElapsedMs = ref(0)
/** 麦克风是否可用（录制失败 no_input_device 时置 false 并禁用按钮） */
const micUnavailable = ref(false)
/** 录制区块提示文字 */
const recMessage = ref<string | null>(null)
const recIsWarning = ref(false)
let recMsgTimer: number | null = null

/** 波形环形缓冲（最近 N 个幅度采样，0~1） */
const WAVE_LEN = 150
const waveBuf = new Array<number>(WAVE_LEN).fill(0)
let waveWritePos = 0
const waveCanvasStart = ref<HTMLCanvasElement | null>(null)
const waveCanvasStop = ref<HTMLCanvasElement | null>(null)
let rafId: number | null = null
let recTimer: number | null = null

let unlistenAmp: UnlistenFn | null = null
let unlistenFinished: UnlistenFn | null = null
let unlistenError: UnlistenFn | null = null

// ===== 剪裁态（阶段 18.1）=====
const trimmingTarget = ref<SoundTarget | null>(null)
const trimmingSamples = ref<Int16Array | null>(null)
const trimmingSampleRate = ref(44100)
const trimmingDurationMs = ref(0)
const trimStart = ref(0)
const trimEnd = ref(0)
const draggingMarker = ref<'start' | 'end' | null>(null)
const trimCanvasStart = ref<HTMLCanvasElement | null>(null)
const trimCanvasStop = ref<HTMLCanvasElement | null>(null)

const trimRangeLabel = computed(() => {
  const s = (trimStart.value / 1000).toFixed(1)
  const e = (trimEnd.value / 1000).toFixed(1)
  const dur = ((trimEnd.value - trimStart.value) / 1000).toFixed(1)
  return `已选 ${s}s ~ ${e}s（${dur}秒）`
})

const recElapsedLabel = computed(() => {
  const s = Math.min(Math.floor(recElapsedMs.value / 1000), MAX_REC_SECS)
  return `0:0${s} / 0:0${MAX_REC_SECS}`
})

function showRecMessage(text: string, warning = false): void {
  recMessage.value = text
  recIsWarning.value = warning
  if (recMsgTimer !== null) window.clearTimeout(recMsgTimer)
  recMsgTimer = window.setTimeout(() => {
    recMessage.value = null
    recIsWarning.value = false
    recMsgTimer = null
  }, 2500)
}

async function refreshSoundStatus(): Promise<void> {
  try {
    const [s, t] = await invoke<[boolean, boolean]>('get_sound_status')
    startExists.value = s
    stopExists.value = t
  } catch (err) {
    console.error('Failed to get sound status:', err)
  }
}

async function onStartRecord(target: SoundTarget): Promise<void> {
  if (recordingTarget.value !== null) return
  try {
    await invoke('start_recording', { target })
    recordingTarget.value = target
    recElapsedMs.value = 0
    waveBuf.fill(0)
    waveWritePos = 0
    appStore.runtimeStatus = 'Recording'
    const startTs = Date.now()
    recTimer = window.setInterval(() => {
      recElapsedMs.value = Date.now() - startTs
      if (recElapsedMs.value >= MAX_REC_SECS * 1000) {
        // 后端到上限也会自停；此处主动停一次保证 UI 一致
        void onStopRecord()
      }
    }, 100)
    await nextTick()
    startWaveLoop()
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    if (msg.includes('no_input_device')) {
      micUnavailable.value = true
      showRecMessage('未检测到麦克风', true)
    } else {
      showRecMessage(`录制失败: ${msg}`, true)
    }
    console.error('start_recording failed:', err)
  }
}

async function onStopRecord(): Promise<void> {
  if (recordingTarget.value === null) return
  try {
    await invoke('stop_recording')
  } catch (err) {
    console.error('stop_recording failed:', err)
  }
  // 收尾统一在 recording_finished 事件处理
}

async function onCancelRecord(): Promise<void> {
  if (recordingTarget.value === null) return
  try {
    await invoke('cancel_recording')
  } catch (err) {
    console.error('cancel_recording failed:', err)
  }
}

async function onPreview(target: SoundTarget): Promise<void> {
  try {
    await invoke('preview_sound', { target })
  } catch (err) {
    console.error('preview_sound failed:', err)
  }
}

/** 重置录制本地状态（事件收尾或卸载时调用） */
function resetRecordingState(): void {
  recordingTarget.value = null
  if (recTimer !== null) {
    window.clearInterval(recTimer)
    recTimer = null
  }
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
}

function startWaveLoop(): void {
  if (rafId !== null) return
  const draw = () => {
    drawWave()
    rafId = requestAnimationFrame(draw)
  }
  rafId = requestAnimationFrame(draw)
}

function drawWave(): void {
  const canvas = recordingTarget.value === 'start' ? waveCanvasStart.value : waveCanvasStop.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  // 同步 canvas 分辨率到 CSS 尺寸（避免模糊 + 响应容器宽度）
  const cw = canvas.clientWidth
  if (canvas.width !== cw) canvas.width = cw
  const w = canvas.width
  const h = canvas.height
  ctx.clearRect(0, 0, w, h)
  const mid = h / 2
  const barW = w / WAVE_LEN
  const accent = getComputedStyle(canvas).getPropertyValue('--accent').trim()
  ctx.fillStyle = accent || '#FE7733'
  for (let i = 0; i < WAVE_LEN; i++) {
    // 从最旧到最新读取环形缓冲
    const idx = (waveWritePos + i) % WAVE_LEN
    const barH = Math.max(1, waveBuf[idx] * (h - 2))
    ctx.fillRect(i * barW, mid - barH / 2, Math.max(1, barW - 1), barH)
  }
}

// ===== 剪裁逻辑函数（阶段 18.1）=====
function enterTrimmingMode(target: SoundTarget, base64: string, sr: number, dur: number) {
  // base64 → Int16Array
  const bin = atob(base64)
  const buf = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i)
  const samples = new Int16Array(buf.buffer)

  trimmingTarget.value = target
  trimmingSamples.value = samples
  trimmingSampleRate.value = sr
  trimmingDurationMs.value = dur
  trimStart.value = 0
  trimEnd.value = dur
  appStore.runtimeStatus = 'Idle'

  nextTick(() => drawTrimWave())
}

function drawTrimWave() {
  const canvas = trimmingTarget.value === 'start' ? trimCanvasStart.value : trimCanvasStop.value
  if (!canvas || !trimmingSamples.value) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  const cw = canvas.clientWidth
  if (canvas.width !== cw) canvas.width = cw
  const w = canvas.width, h = canvas.height
  ctx.clearRect(0, 0, w, h)

  // 绘制全程波形（降采样）
  const samples = trimmingSamples.value
  ctx.strokeStyle = getComputedStyle(canvas).getPropertyValue('--accent').trim() || '#FE7733'
  ctx.lineWidth = 1
  ctx.beginPath()
  for (let x = 0; x < w; x++) {
    const i = Math.floor((x / w) * samples.length)
    const val = samples[i] / 32768
    const y = h / 2 - val * (h / 2 - 2)
    x === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)
  }
  ctx.stroke()

  // 绘制选区遮罩（start 左侧 + end 右侧半透明灰）
  ctx.fillStyle = 'rgba(0,0,0,0.5)'
  const startX = (trimStart.value / trimmingDurationMs.value) * w
  const endX = (trimEnd.value / trimmingDurationMs.value) * w
  ctx.fillRect(0, 0, startX, h)
  ctx.fillRect(endX, 0, w - endX, h)
}

async function onPreviewTrimmed() {
  if (!trimmingSamples.value) return
  const ctx = new AudioContext()
  const buf = ctx.createBuffer(1, trimmingSamples.value.length, trimmingSampleRate.value)
  const ch = buf.getChannelData(0)
  for (let i = 0; i < trimmingSamples.value.length; i++) {
    ch[i] = trimmingSamples.value[i] / 32768
  }
  const src = ctx.createBufferSource()
  src.buffer = buf
  src.connect(ctx.destination)
  const startS = trimStart.value / 1000
  const durS = (trimEnd.value - trimStart.value) / 1000
  src.start(0, startS, durS)
}

async function onConfirmTrim() {
  if (trimmingTarget.value === null) return
  try {
    await invoke('save_trimmed_audio', {
      target: trimmingTarget.value,
      startMs: trimStart.value,
      endMs: trimEnd.value,
    })
    exitTrimmingMode()
    await refreshSoundStatus()
    showRecMessage('已保存', false)
  } catch (err) {
    showRecMessage(`保存失败: ${err}`, true)
  }
}

function onCancelTrim() {
  exitTrimmingMode()
}

function exitTrimmingMode() {
  trimmingTarget.value = null
  trimmingSamples.value = null
  appStore.runtimeStatus = 'Idle'
}

function onMarkerMouseDown(marker: 'start' | 'end', e: MouseEvent) {
  e.preventDefault()
  draggingMarker.value = marker

  const onMove = (me: MouseEvent) => {
    if (!draggingMarker.value) return
    const canvas = trimmingTarget.value === 'start' ? trimCanvasStart.value : trimCanvasStop.value
    if (!canvas) return
    const rect = canvas.getBoundingClientRect()
    const x = me.clientX - rect.left
    const ratio = x / rect.width
    const ms = Math.max(0, Math.min(trimmingDurationMs.value, ratio * trimmingDurationMs.value))
    if (draggingMarker.value === 'start') {
      trimStart.value = Math.min(ms, trimEnd.value - 100)
    } else {
      trimEnd.value = Math.max(ms, trimStart.value + 100)
    }
    drawTrimWave()
  }
  const onUp = () => {
    draggingMarker.value = null
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

onMounted(async () => {
  await refreshSoundStatus()

  unlistenAmp = await listen<{ level: number }>('recording_amplitude', (e) => {
    waveBuf[waveWritePos] = Math.min(1, Math.max(0, e.payload.level))
    waveWritePos = (waveWritePos + 1) % WAVE_LEN
  })

  unlistenFinished = await listen<{
    cancelled: boolean
    samplesBase64?: string
    sampleRate?: number
    durationMs: number
  }>('recording_finished', (e) => {
    resetRecordingState()
    if (e.payload.cancelled) {
      appStore.runtimeStatus = 'Idle'
      return
    }
    // 录制完成，进入剪裁态
    if (e.payload.samplesBase64 && recordingTarget.value) {
      enterTrimmingMode(
        recordingTarget.value,
        e.payload.samplesBase64,
        e.payload.sampleRate || 44100,
        e.payload.durationMs
      )
    } else {
      appStore.runtimeStatus = 'Idle'
    }
  })

  unlistenError = await listen<{ error: string }>('recording_error', (e) => {
    resetRecordingState()
    appStore.runtimeStatus = 'Idle'
    const err = e.payload.error
    if (err.includes('no_input_device')) {
      micUnavailable.value = true
      showRecMessage('未检测到麦克风', true)
    } else {
      showRecMessage(`录制出错: ${err}`, true)
    }
  })
})

onBeforeUnmount(() => {
  // 离开设置页时若仍在录制，取消之
  if (recordingTarget.value !== null) {
    void invoke('cancel_recording').catch(() => {})
  }
  resetRecordingState()
  if (recMsgTimer !== null) window.clearTimeout(recMsgTimer)
  if (messageTimer !== null) window.clearTimeout(messageTimer)
  unlistenAmp?.()
  unlistenFinished?.()
  unlistenError?.()
})

onMounted(() => {
  // 设置页不是可触发模拟页 → runtimeStatus 维持 Idle（任务 4）
  appStore.runtimeStatus = 'Idle'
})
</script>

<template>
  <section class="settings-page">
    <header class="page-header">
      <h2 class="page-title">全局热键</h2>
      <p class="page-desc">在按键模拟 / 鼠标模拟页按下热键启动或停止模拟。</p>
    </header>

    <div class="form">
      <div class="form-row">
        <label class="form-label">启动热键</label>
        <KeyCaptureInput v-model="startKey" placeholder="点击捕获按键" />
      </div>

      <div class="form-row">
        <label class="form-label">停止热键</label>
        <KeyCaptureInput v-model="stopKey" placeholder="点击捕获按键" />
      </div>
    </div>

    <header class="page-header section-gap">
      <h2 class="page-title">提示音</h2>
      <p class="page-desc">录制人声覆盖 exe 同级的 按键开启 / 按键关闭 .wav（最长 5 秒）。</p>
    </header>

    <div class="form sound-form">
      <!-- 开启提示音 -->
      <div class="sound-item">
        <div class="sound-line">
          <span class="dot" :class="recordingTarget === 'start' ? 'dot-rec' : startExists ? 'dot-on' : 'dot-off'"></span>
          <span class="sound-label">开启提示音</span>
          <span class="sound-state">
            <template v-if="recordingTarget === 'start'">录制中 {{ recElapsedLabel }}</template>
            <template v-else-if="startExists">已录制</template>
            <template v-else>未录制</template>
          </span>
          <span class="sound-actions" v-if="recordingTarget !== 'start' && trimmingTarget !== 'start'">
            <button type="button" class="mini-btn" :disabled="!startExists || recordingTarget !== null || trimmingTarget !== null" @click="onPreview('start')">试听</button>
            <button type="button" class="mini-btn rec" :disabled="recordingTarget !== null || trimmingTarget !== null || micUnavailable" @click="onStartRecord('start')">录制</button>
          </span>
        </div>
        <!-- 录制中 -->
        <div v-if="recordingTarget === 'start'" class="rec-panel">
          <canvas ref="waveCanvasStart" class="wave" height="40"></canvas>
          <div class="rec-buttons">
            <button type="button" class="mini-btn" @click="onCancelRecord">取消</button>
            <button type="button" class="mini-btn primary" @click="onStopRecord">完成</button>
          </div>
        </div>
        <!-- 剪裁中 -->
        <div v-if="trimmingTarget === 'start'" class="trim-panel">
          <div class="trim-wave-wrap">
            <canvas ref="trimCanvasStart" class="wave-trim" height="60"></canvas>
            <div class="trim-marker start"
                 :style="{left: (trimStart / trimmingDurationMs * 100) + '%'}"
                 @mousedown="onMarkerMouseDown('start', $event)">
              <div class="marker-handle"></div>
            </div>
            <div class="trim-marker end"
                 :style="{left: (trimEnd / trimmingDurationMs * 100) + '%'}"
                 @mousedown="onMarkerMouseDown('end', $event)">
              <div class="marker-handle"></div>
            </div>
          </div>
          <span class="trim-info">{{ trimRangeLabel }}</span>
          <div class="trim-buttons">
            <button type="button" class="mini-btn" @click="onPreviewTrimmed">试听</button>
            <button type="button" class="mini-btn primary" @click="onConfirmTrim">确认</button>
            <button type="button" class="mini-btn" @click="onCancelTrim">取消</button>
          </div>
        </div>
      </div>

      <!-- 关闭提示音 -->
      <div class="sound-item">
        <div class="sound-line">
          <span class="dot" :class="recordingTarget === 'stop' ? 'dot-rec' : stopExists ? 'dot-on' : 'dot-off'"></span>
          <span class="sound-label">关闭提示音</span>
          <span class="sound-state">
            <template v-if="recordingTarget === 'stop'">录制中 {{ recElapsedLabel }}</template>
            <template v-else-if="stopExists">已录制</template>
            <template v-else>未录制</template>
          </span>
          <span class="sound-actions" v-if="recordingTarget !== 'stop' && trimmingTarget !== 'stop'">
            <button type="button" class="mini-btn" :disabled="!stopExists || recordingTarget !== null || trimmingTarget !== null" @click="onPreview('stop')">试听</button>
            <button type="button" class="mini-btn rec" :disabled="recordingTarget !== null || trimmingTarget !== null || micUnavailable" @click="onStartRecord('stop')">录制</button>
          </span>
        </div>
        <!-- 录制中 -->
        <div v-if="recordingTarget === 'stop'" class="rec-panel">
          <canvas ref="waveCanvasStop" class="wave" height="40"></canvas>
          <div class="rec-buttons">
            <button type="button" class="mini-btn" @click="onCancelRecord">取消</button>
            <button type="button" class="mini-btn primary" @click="onStopRecord">完成</button>
          </div>
        </div>
        <!-- 剪裁中 -->
        <div v-if="trimmingTarget === 'stop'" class="trim-panel">
          <div class="trim-wave-wrap">
            <canvas ref="trimCanvasStop" class="wave-trim" height="60"></canvas>
            <div class="trim-marker start"
                 :style="{left: (trimStart / trimmingDurationMs * 100) + '%'}"
                 @mousedown="onMarkerMouseDown('start', $event)">
              <div class="marker-handle"></div>
            </div>
            <div class="trim-marker end"
                 :style="{left: (trimEnd / trimmingDurationMs * 100) + '%'}"
                 @mousedown="onMarkerMouseDown('end', $event)">
              <div class="marker-handle"></div>
            </div>
          </div>
          <span class="trim-info">{{ trimRangeLabel }}</span>
          <div class="trim-buttons">
            <button type="button" class="mini-btn" @click="onPreviewTrimmed">试听</button>
            <button type="button" class="mini-btn primary" @click="onConfirmTrim">确认</button>
            <button type="button" class="mini-btn" @click="onCancelTrim">取消</button>
          </div>
        </div>
      </div>

      <span v-if="recMessage" class="save-msg sound-msg" :class="{ 'msg-warning': recIsWarning }">{{ recMessage }}</span>
    </div>

    <footer class="form-footer">
      <span v-if="saveMessage" class="save-msg" :class="{ 'msg-warning': isWarning }">{{ saveMessage }}</span>
      <button
        type="button"
        class="save-btn"
        :disabled="!isDirty"
        @click="onSave"
      >
        保存
      </button>
    </footer>
  </section>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 16px 18px;
  gap: 14px;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
}

/* 滚动条样式 — 同 KeyboardPage / MousePage */
.settings-page::-webkit-scrollbar {
  width: 8px;
}

.settings-page::-webkit-scrollbar-track {
  background: transparent;
}

.settings-page::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 4px;
}

.settings-page::-webkit-scrollbar-thumb:hover {
  background: var(--text-disabled);
}

.page-header {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.page-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.page-desc {
  font-size: 11px;
  color: var(--text-disabled);
  margin: 0;
  line-height: 1.4;
}

.form {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 18px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-subtle);
  border-radius: 7px;
}

.form-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.form-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
  flex-shrink: 0;
}

.form-footer {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  height: 32px;
  margin-top: auto;
}

.save-msg {
  font-size: 11px;
  color: var(--success);
  letter-spacing: 0.3px;
  animation: fade-in var(--transition-normal) var(--ease-default);
}

.save-msg.msg-warning {
  color: var(--warning);
}

@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(2px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.save-btn {
  height: 32px;
  min-width: 96px;
  padding: 0 24px;
  border-radius: 6px;
  background: var(--accent);
  color: var(--color-paper-white);
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.5px;
  transition:
    background var(--transition-fast) var(--ease-default),
    opacity var(--transition-fast) var(--ease-default);
}

.save-btn:hover:not(:disabled) {
  background: var(--accent-hover);
}

.save-btn:active:not(:disabled) {
  background: var(--accent-pressed);
}

.save-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ===== 提示音录制（阶段 18） ===== */
.section-gap {
  margin-top: 4px;
}

.sound-form {
  gap: 10px;
}

.sound-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sound-line {
  display: flex;
  align-items: center;
  gap: 10px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-on {
  background: var(--success);
}

.dot-off {
  background: var(--text-disabled);
}

.dot-rec {
  background: var(--danger);
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.sound-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
  flex-shrink: 0;
}

.sound-state {
  font-size: 11px;
  color: var(--text-secondary);
  flex: 1;
}

.sound-actions {
  display: flex;
  gap: 8px;
}

.mini-btn {
  height: 26px;
  padding: 0 12px;
  border-radius: 5px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  font-size: 11px;
  transition: background var(--transition-fast) var(--ease-default);
}

.mini-btn:hover:not(:disabled) {
  background: var(--border-color);
}

.mini-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.mini-btn.rec {
  border-color: var(--accent);
  color: var(--accent);
}

.mini-btn.rec:hover:not(:disabled) {
  background: var(--accent);
  color: var(--color-paper-white);
}

.mini-btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--color-paper-white);
}

.mini-btn.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}

.rec-panel {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-left: 18px;
  max-width: 100%;
  overflow: hidden;
}

.wave {
  flex: 1;
  min-width: 0;
  height: 40px;
  background: var(--bg-primary);
  border: 1px solid var(--border-subtle);
  border-radius: 5px;
}

.rec-buttons {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.sound-msg {
  align-self: flex-end;
}

/* ===== 剪裁样式（阶段 18.1）===== */
.trim-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-left: 18px;
}

.trim-wave-wrap {
  position: relative;
  width: 100%;
}

.wave-trim {
  width: 100%;
  height: 60px;
  background: var(--bg-primary);
  border: 1px solid var(--border-subtle);
  border-radius: 5px;
  display: block;
}

.trim-marker {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: var(--accent);
  cursor: ew-resize;
}

.trim-marker .marker-handle {
  position: absolute;
  top: -4px;
  left: 50%;
  transform: translateX(-50%);
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid var(--color-paper-white);
}

.trim-info {
  font-size: 11px;
  color: var(--text-secondary);
  padding-left: 0;
}

.trim-buttons {
  display: flex;
  gap: 8px;
}
</style>
