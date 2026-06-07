/**
 * 全局状态管理 — DESIGN 9.1
 *
 * 使用 Vue 3 reactive 作为轻量 store。阶段 2 仅含页面路由、运行状态与锁定标记；
 * config / driverStatus / 各动作列表等字段在阶段 3-8 逐步补全。
 */

import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppPage, RuntimeStatus, KeyboardAction, MouseAction, HotkeyConfig } from '../types/config'

export const appStore = reactive({
  /** 当前激活页面，默认首页 */
  currentPage: 'home' as AppPage,
  /** 运行状态机，默认待机 */
  runtimeStatus: 'Idle' as RuntimeStatus,
  /** 运行期锁定蒙版开关（阶段 7 接入） */
  isLocked: false,
  /**
   * 按键动作列表 — 应用默认初始值，与后端 default_config() 一致。
   * onMounted 调用 load_config 后会被持久化数据覆盖；
   * load_config 极少情况失败时此值作为合理的展示兜底。
   */
  keyboardActions: [
    {
      id: 'default-keyboard-1',
      selected: true,
      keyLabel: 'F',
      scanCode: 33,
      intervalMs: 20,
    },
  ] as KeyboardAction[],
  /**
   * 鼠标动作列表 — 应用默认初始值，与后端 default_config() 一致。
   */
  mouseActions: [
    {
      id: 'default-mouse-1',
      x: null,
      y: null,
      intervalMs: 20,
    },
  ] as MouseAction[],
  /**
   * 全局热键配置 — 应用默认初始值（F12 / F12），与后端 default_config() 一致。
   */
  hotkeys: {
    start: { keyLabel: 'F12', scanCode: 88 },
    stop: { keyLabel: 'F12', scanCode: 88 },
  } as HotkeyConfig,
})

/** 切换当前页面（阶段 12：调用后端 set_current_page） */
export function setPage(page: AppPage): void {
  appStore.currentPage = page
  invoke('set_current_page', { page }).catch((err) => {
    console.error('Failed to set current page:', err)
  })
}
