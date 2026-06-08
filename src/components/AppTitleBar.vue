<script setup lang="ts">
/**
 * 自定义标题栏 — DESIGN 15.3 / 需求 3.1
 * 左：应用图标 + Mimic；中：当前页面名；右：最小化 / 关闭。
 * 整条作为 data-tauri-drag-region 拖拽区，按钮区单独阻止拖拽。
 */
import { computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { appStore } from '../stores/appStore'
import { PAGE_LABELS } from '../lib/pages'

const currentPageLabel = computed(() => PAGE_LABELS[appStore.currentPage])

const appWindow = getCurrentWindow()

function minimizeWindow(): void {
  appWindow.minimize()
}

function closeWindow(): void {
  appWindow.close()
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <span class="brand">
      <svg class="app-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <rect width="24" height="24" rx="4" fill="url(#gradient)"/>
        <path d="M8 10L12 14L16 10" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        <defs>
          <linearGradient id="gradient" x1="0" y1="0" x2="24" y2="24">
            <stop offset="0%" stop-color="#FE7733"/>
            <stop offset="100%" stop-color="#B1FA63"/>
          </linearGradient>
        </defs>
      </svg>
      <span class="app-name">Mimic</span>
    </span>
    <span class="current-page">{{ currentPageLabel }}</span>
    <div class="window-controls">
      <button
        class="ctrl-btn"
        type="button"
        aria-label="最小化"
        @click="minimizeWindow"
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <line x1="1" y1="5" x2="9" y2="5" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
      <button
        class="ctrl-btn ctrl-close"
        type="button"
        aria-label="关闭"
        @click="closeWindow"
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1" />
          <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  height: var(--titlebar-height);
  padding: 0 8px 0 12px;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-self: start;
  pointer-events: none;
}

.app-icon {
  width: 18px;
  height: 18px;
}

.app-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.current-page {
  justify-self: center;
  font-size: 12px;
  color: var(--text-secondary);
  pointer-events: none;
}

.window-controls {
  display: flex;
  align-items: center;
  gap: 4px;
  justify-self: end;
}

.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 24px;
  border-radius: 6px;
  color: var(--text-secondary);
  transition:
    background var(--transition-fast) var(--ease-default),
    color var(--transition-fast) var(--ease-default);
}

.ctrl-btn:hover {
  background: var(--bg-elevated);
  color: var(--text-primary);
}

.ctrl-btn:active {
  background: var(--bg-secondary);
}

.ctrl-close:hover {
  background: var(--danger);
  color: var(--color-paper-white);
}
</style>
