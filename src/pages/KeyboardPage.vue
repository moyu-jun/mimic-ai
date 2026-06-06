<script setup lang="ts">
/**
 * 按键模拟页 — 需求 3.3.2 / DESIGN 15.6
 * 列表：勾选 / 键位 / 间隔 / 删除；顶部：捕获框 + 添加按钮。
 * 阶段 4 数据全部 mock 前端，阶段 8 起接 load_config / save_config。
 */
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { appStore } from '../stores/appStore'
import KeyCaptureInput from '../components/KeyCaptureInput.vue'
import type { CapturedKey, KeyboardAction } from '../types/config'

const DEFAULT_INTERVAL_MS = 20

const capturedKey = ref<CapturedKey | null>(null)

function addAction(): void {
  if (!capturedKey.value) return

  // 已存在相同键位则拒绝添加，重置输入框
  const exists = appStore.keyboardActions.some(
    a => a.scanCode === capturedKey.value!.scanCode
  )
  if (exists) {
    capturedKey.value = null
    return
  }

  const newAction: KeyboardAction = {
    id: `kb-${Date.now()}`,
    selected: false,
    keyLabel: capturedKey.value.keyLabel,
    scanCode: capturedKey.value.scanCode,
    intervalMs: DEFAULT_INTERVAL_MS,
  }

  appStore.keyboardActions.push(newAction)
  capturedKey.value = null
}

function deleteAction(id: string): void {
  const idx = appStore.keyboardActions.findIndex(a => a.id === id)
  if (idx !== -1) {
    appStore.keyboardActions.splice(idx, 1)
  }
}

function toggleSelected(id: string): void {
  const action = appStore.keyboardActions.find(a => a.id === id)
  if (action) {
    action.selected = !action.selected
  }
}

function onIntervalInput(action: KeyboardAction, e: Event): void {
  const target = e.target as HTMLInputElement
  const sanitized = target.value.replace(/[^0-9]/g, '')

  const num = parseInt(sanitized, 10)
  if (!isNaN(num) && num > 0) {
    action.intervalMs = num
    target.value = sanitized
  } else {
    // Reject zero and empty - reset to DEFAULT
    action.intervalMs = DEFAULT_INTERVAL_MS
    target.value = String(DEFAULT_INTERVAL_MS)
  }
}

onMounted(() => {
  // 阶段 4 mock：进入按键页时切换到 ReadyKeyboard（TASKS 任务 6）
  appStore.runtimeStatus = 'ReadyKeyboard'
})

onBeforeUnmount(() => {
  // 离开时回到 Idle（阶段 12 会由 set_current_page 统一管理）
  appStore.runtimeStatus = 'Idle'
})
</script>

<template>
  <section class="keyboard-page">
    <header class="top-bar">
      <KeyCaptureInput v-model="capturedKey" placeholder="点击捕获按键" />
      <button
        type="button"
        class="add-btn"
        :disabled="!capturedKey"
        @click="addAction"
      >
        添加
      </button>
    </header>

    <div class="list-container">
      <div v-if="!appStore.keyboardActions.length" class="empty-hint">
        暂无按键动作
      </div>
      <div v-else class="list-scroll">
        <div
          v-for="action in appStore.keyboardActions"
          :key="action.id"
          class="list-row"
          :class="{ unselected: !action.selected }"
        >
          <label class="checkbox-wrapper">
            <input
              type="checkbox"
              class="checkbox"
              :checked="action.selected"
              :aria-label="`选择 ${action.keyLabel} 按键动作`"
              @change="toggleSelected(action.id)"
            />
          </label>
          <span class="key-info">{{ action.keyLabel }}</span>
          <input
            type="text"
            class="interval-input"
            :value="action.intervalMs"
            @input="onIntervalInput(action, $event)"
          />
          <span class="unit">ms</span>
          <button
            type="button"
            class="delete-btn"
            aria-label="删除"
            @click="deleteAction(action.id)"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
              <path
                d="M3 3 L11 11 M11 3 L3 11"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.keyboard-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 14px 16px;
  gap: 12px;
  overflow: hidden;
}

.top-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.add-btn {
  height: 30px;
  padding: 0 16px;
  border-radius: 6px;
  background: var(--accent);
  color: var(--color-paper-white);
  font-size: 12px;
  font-weight: 500;
  transition:
    background var(--transition-fast) var(--ease-default),
    opacity var(--transition-fast) var(--ease-default);
}

.add-btn:hover:not(:disabled) {
  background: var(--accent-hover);
}

.add-btn:active:not(:disabled) {
  background: var(--accent-pressed);
}

.add-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.list-container {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.empty-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  font-size: 12px;
  color: var(--text-disabled);
}

.list-scroll {
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow-y: auto;
  scrollbar-gutter: stable;
}

.list-row {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 36px;
  min-height: 36px;
  flex-shrink: 0;
  padding: 0 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-subtle);
  border-radius: 7px;
  transition: opacity var(--transition-fast) var(--ease-default);
}

.list-row.unselected {
  opacity: 0.5;
}

.checkbox-wrapper {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.checkbox {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: var(--accent);
}

.key-info {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  font-weight: 500;
}

.interval-input {
  width: 60px;
  height: 24px;
  padding: 0 8px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 5px;
  font-size: 12px;
  color: var(--text-primary);
  text-align: center;
  transition: border-color var(--transition-fast) var(--ease-default);
}

.interval-input:focus {
  outline: none;
  border-color: var(--accent);
}

.unit {
  font-size: 11px;
  color: var(--text-disabled);
}

.delete-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 5px;
  color: var(--text-secondary);
  transition:
    background var(--transition-fast) var(--ease-default),
    color var(--transition-fast) var(--ease-default);
}

.delete-btn:hover {
  background: var(--bg-elevated);
  color: var(--danger);
}

.delete-btn:active {
  background: var(--bg-primary);
}

/* 滚动条样式 */
.list-scroll::-webkit-scrollbar {
  width: 8px;
}

.list-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.list-scroll::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 4px;
}

.list-scroll::-webkit-scrollbar-thumb:hover {
  background: var(--text-disabled);
}
</style>
