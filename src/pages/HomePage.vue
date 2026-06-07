<script setup lang="ts">
/**
 * 首页 — 状态仪表盘（DESIGN 15.4 / 需求 3.3.1）
 * 阶段 9：onMounted 调用 get_init_warning，有写盘失败时显示小字提示。
 * 阶段 10：管理员权限状态接 get_admin_status；未授权时提供「以管理员身份重启」按钮，
 *         点击触发 request_admin_restart（UAC 提示 + 自身退出）。
 *   - 驱动状态：阶段 11 接 check_driver_status / install_driver
 *   - 热键概览：由 load_config 提供（阶段 8 已接入）
 *
 * 阶段 7：临时增加「模拟运行（mock）」切换按钮，用于验证锁定蒙版视觉。
 *   - 阶段 12 真热键接入后整体移除该按钮（但保留 App.vue 内的 lock-overlay 逻辑）。
 */
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { appStore } from '../stores/appStore'
import type { DriverStatus } from '../types/config'

const APP_VERSION = '0.1.0'
const APP_TAGLINE = 'Windows 按键与鼠标模拟工具'

// 启动时配置写盘失败的警告（极小概率，默认为空）
const configWarning = ref<string | null>(null)

// 阶段 10：管理员权限状态（来自后端 get_admin_status）
// 默认 true 避免首次渲染闪过橙色警告；onMounted 立即覆盖为真实值。
const isAdmin = ref(true)
const isRestarting = ref(false)
const restartError = ref<string | null>(null)

// 阶段 11：驱动状态（来自后端 check_driver_status）
const driverStatus = ref<DriverStatus>('NotInstalled')
const isInstalling = ref(false)
const installError = ref<string | null>(null)

async function onRestartAsAdmin(): Promise<void> {
  if (isRestarting.value) return
  isRestarting.value = true
  restartError.value = null
  try {
    await invoke('request_admin_restart')
    // 后端会在 200ms 内退出当前进程，前端无需做更多事情
  } catch (err) {
    // 用户拒绝 UAC 或调度失败
    isRestarting.value = false
    restartError.value = String(err).includes('declined')
      ? '已取消提权'
      : '提权重启失败，请稍后重试'
  }
}

async function onInstallDriver(): Promise<void> {
  if (isInstalling.value) return
  isInstalling.value = true
  installError.value = null
  try {
    await invoke('install_interception_driver')
    // 安装器已启动，重新检测驱动状态
    const status = await invoke<string>('check_driver_status')
    driverStatus.value = JSON.parse(status) as DriverStatus
  } catch (err) {
    installError.value = String(err).includes('declined')
      ? '已取消安装'
      : String(err).includes('not found')
        ? '安装程序不存在，请检查 drivers 目录'
        : '安装失败，请稍后重试'
    log_error('[HomePage] install driver failed:', err)
  } finally {
    isInstalling.value = false
  }
}

function log_error(msg: string, err: unknown): void {
  console.error(msg, err)
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

onMounted(async () => {
  // 并行触发，任一失败不阻塞另一项
  const warningPromise = invoke<string | null>('get_init_warning').catch(() => null)
  const adminPromise = invoke<boolean>('get_admin_status').catch(() => true)
  const driverPromise = invoke<string>('check_driver_status').catch(() => '"NotInstalled"')
  const [warning, admin, driverRaw] = await Promise.all([warningPromise, adminPromise, driverPromise])
  configWarning.value = warning
  isAdmin.value = admin
  try {
    driverStatus.value = JSON.parse(driverRaw) as DriverStatus
  } catch {
    driverStatus.value = 'NotInstalled'
  }
})
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
        {{ isAdmin ? '管理员权限已授予' : '管理员权限受限,部分功能不可用' }}
      </span>
      <button
        v-if="!isAdmin"
        type="button"
        class="restart-btn"
        :disabled="isRestarting"
        @click="onRestartAsAdmin"
      >
        {{ isRestarting ? '正在重启...' : '以管理员身份重启' }}
      </button>
    </div>
    <p v-if="restartError" class="restart-error">{{ restartError }}</p>

    <!-- 驱动状态卡片 -->
    <div class="card driver-card">
      <div class="driver-info">
        <span
          class="driver-dot"
          :class="{
            'dot-ready': driverStatus === 'Ready',
            'dot-warn': driverStatus === 'InstalledNeedReboot',
            'dot-error': driverStatus === 'Error',
          }"
        ></span>
        <span class="driver-text">
          {{ driverStatus === 'Ready' ? 'Interception 驱动已加载'
           : driverStatus === 'InstalledNeedReboot' ? '驱动已安装，需重启系统'
           : driverStatus === 'Error' ? '驱动错误'
           : '驱动未安装' }}
        </span>
      </div>
      <button
        v-if="driverStatus === 'NotInstalled'"
        type="button"
        class="install-btn"
        :disabled="isInstalling"
        @click="onInstallDriver"
      >
        {{ isInstalling ? '正在安装...' : '安装驱动' }}
      </button>
    </div>
    <p v-if="installError" class="install-error">{{ installError }}</p>

    <!-- 当前热键概览 -->
    <div class="card hotkey-card">
      <span class="hotkey-label">当前热键</span>
      <span class="hotkey-value">
        启动：<b>{{ appStore.hotkeys.start.keyLabel }}</b>
        <span class="sep">|</span>
        停止：<b>{{ appStore.hotkeys.stop.keyLabel }}</b>
      </span>
    </div>

    <!-- 配置写盘失败警告（极小概率，默认不显示） -->
    <p v-if="configWarning" class="config-warning">
      ⚠ 配置文件无法写入，本次使用默认配置运行，修改不会保存。
    </p>

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

/* 阶段 10：以管理员身份重启按钮 — 仅未授权时出现 */
.restart-btn {
  margin-left: auto;
  padding: 3px 10px;
  border-radius: 6px;
  background: var(--warning);
  color: var(--bg-primary);
  font-size: 11px;
  font-weight: 600;
  flex-shrink: 0;
  transition:
    background var(--transition-fast) var(--ease-default),
    transform var(--transition-fast) var(--ease-default);
}

.restart-btn:hover:not(:disabled) {
  background: var(--accent-hover);
}

.restart-btn:active:not(:disabled) {
  background: var(--accent-pressed);
  transform: translateY(1px);
}

.restart-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.restart-error {
  margin: -4px 0 0;
  font-size: 11px;
  color: var(--danger);
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

.driver-dot.dot-ready {
  background: var(--success);
}

.driver-dot.dot-warn {
  background: var(--warning);
}

.driver-dot.dot-error {
  background: var(--danger);
}

.install-error {
  margin: -4px 0 0;
  font-size: 11px;
  color: var(--danger);
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

/* 配置写盘失败警告 — 正常情况不可见 */
.config-warning {
  margin: 0;
  font-size: 11px;
  color: var(--warning);
  opacity: 0.85;
}
</style>
