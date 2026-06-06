<script setup lang="ts">
/**
 * 首页 — 状态仪表盘（DESIGN 15.4 / 需求 3.3.1）
 * 阶段 3：数据全部为前端 mock，不绑定后端。
 *   - 管理员权限：阶段 10 接 get_admin_status
 *   - 驱动状态：阶段 11 接 check_driver_status / install_driver
 *   - 热键概览：阶段 8 起由 load_config 提供
 *
 * 阶段 7：临时增加「模拟运行（mock）」切换按钮，用于验证锁定蒙版视觉。
 *   - 点击在 Idle ↔ RunningKeyboard 间切换 runtimeStatus + isLocked。
 *   - 按钮使用 position:fixed + 高 z-index，确保锁定后仍能点击切回（否则验证流程无法闭环）。
 *   - 阶段 12 真热键接入后整体移除该按钮（但保留 App.vue 内的 lock-overlay 逻辑）。
 */
import { computed } from 'vue'
import { appStore } from '../stores/appStore'

const APP_VERSION = '0.1.0'
const APP_TAGLINE = 'Windows 按键与鼠标模拟工具'

// === mock 数据（后续阶段替换为真实命令）===
const isAdmin = true
const startHotkey = 'F12'
const stopHotkey = 'F12'

function onInstallDriver(): void {
  // 阶段 11 接 install_driver；当前为占位，点击无副作用。
  console.log('[mock] install driver clicked')
}

// === 阶段 7 mock：模拟运行切换 ===
const isMockRunning = computed(() => appStore.runtimeStatus === 'RunningKeyboard')

function toggleMockRun(): void {
  if (isMockRunning.value) {
    appStore.runtimeStatus = 'Idle'
    appStore.isLocked = false
  } else {
    appStore.runtimeStatus = 'RunningKeyboard'
    appStore.isLocked = true
  }
}
</script>

<template>
  <section class="home">
    <!-- 顶部：应用名 + 版本 + 简介 -->
    <header class="hero">
      <div class="hero-title">
        <span class="app-name">Mimic</span>
        <span class="app-version">v{{ APP_VERSION }}</span>
      </div>
      <p class="tagline">{{ APP_TAGLINE }}</p>
    </header>

    <!-- 管理员权限状态 -->
    <div
      class="status-line"
      :class="isAdmin ? 'ok' : 'warn'"
    >
      <span class="status-icon">{{ isAdmin ? '✓' : '!' }}</span>
      <span class="status-text">
        {{ isAdmin ? '管理员权限已授予' : '管理员权限受限，部分功能不可用' }}
      </span>
    </div>

    <!-- 驱动状态卡片 -->
    <div class="card driver-card">
      <div class="driver-info">
        <span class="driver-dot"></span>
        <span class="driver-text">驱动未安装</span>
      </div>
      <button type="button" class="install-btn" @click="onInstallDriver">
        安装驱动
      </button>
    </div>

    <!-- 当前热键概览 -->
    <div class="card hotkey-card">
      <span class="hotkey-label">当前热键</span>
      <span class="hotkey-value">
        启动：<b>{{ startHotkey }}</b>
        <span class="sep">|</span>
        停止：<b>{{ stopHotkey }}</b>
      </span>
    </div>

    <!-- 阶段 7 临时按钮：mock 切换 RunningKeyboard，验证锁定蒙版（阶段 12 移除） -->
    <button
      type="button"
      class="mock-run-btn"
      :class="{ running: isMockRunning }"
      @click="toggleMockRun"
    >
      {{ isMockRunning ? '停止模拟（mock）' : '模拟运行（mock）' }}
    </button>
  </section>
</template>

<style scoped>
.home {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  padding: 16px 18px;
  overflow: hidden;
}

.hero {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.hero-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.app-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
}

.app-version {
  font-size: 12px;
  color: var(--text-disabled);
}

.tagline {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
}

/* 权限状态行 */
.status-line {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 12px;
}

.status-line.ok {
  background: color-mix(in srgb, var(--success) 14%, transparent);
  color: var(--success);
}

.status-line.warn {
  background: color-mix(in srgb, var(--warning) 16%, transparent);
  color: var(--warning);
}

.status-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: currentColor;
  color: var(--bg-primary);
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
}

/* 卡片通用 */
.card {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
}

.driver-card {
  justify-content: space-between;
}

.driver-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.driver-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-disabled);
}

.driver-text {
  font-size: 13px;
  color: var(--text-secondary);
}

.install-btn {
  padding: 5px 14px;
  border-radius: 6px;
  background: var(--accent);
  color: var(--color-paper-white);
  font-size: 12px;
  font-weight: 500;
  transition:
    background var(--transition-fast) var(--ease-default),
    transform var(--transition-fast) var(--ease-default);
}

.install-btn:hover {
  background: var(--accent-hover);
}

.install-btn:active {
  background: var(--accent-pressed);
  transform: translateY(1px);
}

.install-btn:focus-visible {
  outline: 1px solid var(--accent);
  outline-offset: 2px;
}

/* 热键概览 */
.hotkey-card {
  justify-content: space-between;
}

.hotkey-label {
  font-size: 13px;
  color: var(--text-secondary);
}

.hotkey-value {
  font-size: 13px;
  color: var(--text-primary);
}

.hotkey-value b {
  font-weight: 600;
}

.hotkey-value .sep {
  margin: 0 6px;
  color: var(--text-disabled);
}

/**
 * 阶段 7 临时按钮 — 锁定后仍需可点击切回 Idle，
 * 因此用 position:fixed 浮在蒙版之上（z-index > .lock-overlay 的 10）。
 * 阶段 12 真热键接入后整体移除。
 */
.mock-run-btn {
  position: fixed;
  right: 16px;
  bottom: calc(var(--statusbar-height) + 12px);
  z-index: 50;
  padding: 6px 14px;
  border-radius: 6px;
  background: var(--accent);
  color: var(--color-paper-white);
  font-size: 12px;
  font-weight: 500;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  transition:
    background var(--transition-fast) var(--ease-default),
    transform var(--transition-fast) var(--ease-default);
}

.mock-run-btn:hover {
  background: var(--accent-hover);
}

.mock-run-btn:active {
  transform: translateY(1px);
}

.mock-run-btn.running {
  background: var(--danger);
}

.mock-run-btn.running:hover {
  background: color-mix(in srgb, var(--danger) 85%, white);
}
</style>
