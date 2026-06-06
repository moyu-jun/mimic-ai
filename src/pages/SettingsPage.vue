<script setup lang="ts">
/**
 * 设置页 — 需求 3.3.4 / DESIGN 15.6
 * 复用 KeyCaptureInput 渲染启动 / 停止热键；保存按钮对比快照，有变化才提示。
 * 阶段 6 mock：保存仅显示「已保存（mock）」，阶段 12 接 update_hotkeys 真实命令。
 */
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import { appStore } from '../stores/appStore'
import KeyCaptureInput from '../components/KeyCaptureInput.vue'
import type { CapturedKey, HotkeyConfig } from '../types/config'

/** 已持久化快照（用于保存时对比是否真的变更） */
const persistedSnapshot = ref<HotkeyConfig>(cloneHotkeys(appStore.hotkeys))

/** v-model 绑定到 reactive store；阶段 6 暂用本地副本，避免捕获后立即影响快照对比 */
const startKey = ref<CapturedKey | null>(cloneKey(appStore.hotkeys.start))
const stopKey = ref<CapturedKey | null>(cloneKey(appStore.hotkeys.stop))

/** 保存提示文本（mock）：null 表示不显示；保存后短暂显示，失败/无变化时不显示 */
const saveMessage = ref<string | null>(null)
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

function showMessage(text: string): void {
  saveMessage.value = text
  if (messageTimer !== null) {
    window.clearTimeout(messageTimer)
  }
  messageTimer = window.setTimeout(() => {
    saveMessage.value = null
    messageTimer = null
  }, 2000)
}

function onSave(): void {
  if (!startKey.value || !stopKey.value) return
  if (!isDirty.value) {
    // 无变化不提示（需求 3.3.4）
    return
  }

  // 阶段 6 mock：直接写入 store + 更新快照；阶段 12 接 update_hotkeys 真实命令
  appStore.hotkeys = {
    start: cloneKey(startKey.value),
    stop: cloneKey(stopKey.value),
  }
  persistedSnapshot.value = cloneHotkeys(appStore.hotkeys)
  showMessage('已保存（mock）')
}

onMounted(() => {
  // 设置页不是可触发模拟页 → runtimeStatus 维持 Idle（任务 4）
  appStore.runtimeStatus = 'Idle'
})

onBeforeUnmount(() => {
  if (messageTimer !== null) {
    window.clearTimeout(messageTimer)
  }
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

    <footer class="form-footer">
      <span v-if="saveMessage" class="save-msg">{{ saveMessage }}</span>
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
  overflow: hidden;
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
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 16px;
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
}

.save-msg {
  font-size: 11px;
  color: var(--success);
  letter-spacing: 0.3px;
  animation: fade-in var(--transition-normal) var(--ease-default);
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
</style>
