<script setup lang="ts">
/**
 * 根组件 — 组合标题栏 + (侧边栏 + 路由内容) + 状态栏。
 * 路由用 currentPage 映射组件，无需引入 vue-router（页面固定四个）。
 * 阶段 7 会在 .main-area 内追加 lock-overlay。
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
  display: flex;
  flex: 1;
  min-height: 0;
}

.content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}
</style>
