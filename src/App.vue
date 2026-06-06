<script setup lang="ts">
/**
 * 根组件 — 组合标题栏 + (侧边栏 + 路由内容) + 状态栏。
 * 路由用 currentPage 映射组件，无需引入 vue-router（页面固定四个）。
 *
 * 阶段 7：在 .main-area 内追加 lock-overlay（DESIGN 15.5 / 需求 3.9）。
 *   - 仅覆盖菜单 + 内容区，不覆盖标题栏与状态栏。
 *   - 半透明灰色，pointer-events 拦截点击。
 *   - 内部无任何文字 / 图标 / 按钮，运行文案由状态栏承载。
 *   - 由 appStore.isLocked 控制；阶段 12 起改由 runtime_status_changed 事件驱动。
 */
import { computed } from 'vue'
import { appStore } from './stores/appStore'
import AppTitleBar from './components/AppTitleBar.vue'
import AppSidebar from './components/AppSidebar.vue'
import AppStatusBar from './components/AppStatusBar.vue'
import HomePage from './pages/HomePage.vue'
import KeyboardPage from './pages/KeyboardPage.vue'
import MousePage from './pages/MousePage.vue'
import SettingsPage from './pages/SettingsPage.vue'
import type { AppPage } from './types/config'

const PAGE_COMPONENTS = {
  home: HomePage,
  keyboard: KeyboardPage,
  mouse: MousePage,
  settings: SettingsPage,
} satisfies Record<AppPage, unknown>

const currentPageComponent = computed(() => PAGE_COMPONENTS[appStore.currentPage])
</script>

<template>
  <div class="app-container">
    <AppTitleBar />
    <div class="main-area">
      <AppSidebar />
      <main class="content">
        <component :is="currentPageComponent" />
      </main>
      <!-- 运行期锁定蒙版：阶段 7 mock 切换，阶段 12 起由后端事件驱动 -->
      <div
        v-if="appStore.isLocked"
        class="lock-overlay"
        aria-hidden="true"
      ></div>
    </div>
    <AppStatusBar />
  </div>
</template>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  width: var(--window-width);
  height: var(--window-height);
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--window-radius);
  overflow: hidden;
}

.main-area {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
}

.content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

/**
 * lock-overlay — DESIGN 15.5
 * 绝对定位铺满 .main-area；不含任何子元素，纯视觉与点击拦截。
 * 半透明灰色取主背景为基底叠加 60% 不透明度，避免硬编码 RGBA。
 */
.lock-overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--bg-primary) 65%, transparent);
  pointer-events: auto;
  cursor: not-allowed;
  z-index: 10;
}
</style>
