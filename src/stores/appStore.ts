/**
 * 全局状态管理 — DESIGN 9.1
 *
 * 使用 Vue 3 reactive 作为轻量 store。阶段 2 仅含页面路由、运行状态与锁定标记；
 * config / driverStatus / 各动作列表等字段在阶段 3-8 逐步补全。
 */

import { reactive } from 'vue'
import type { AppPage, RuntimeStatus, KeyboardAction, MouseAction, HotkeyConfig } from '../types/config'

export const appStore = reactive({
  /** 当前激活页面，默认首页 */
  currentPage: 'home' as AppPage,
  /** 运行状态机，默认待机 */
  runtimeStatus: 'Idle' as RuntimeStatus,
  /** 运行期锁定蒙版开关（阶段 7 接入） */
  isLocked: false,
  /** 按键动作列表（阶段 4 mock，阶段 8 起由 load_config 提供） */
  keyboardActions: [
    {
      id: 'mock-kb-1',
      selected: true,
      keyLabel: 'F',
      scanCode: 33,
      intervalMs: 20,
    },
  ] as KeyboardAction[],
  /** 鼠标动作列表（阶段 5 mock，阶段 8 起由 load_config 提供） */
  mouseActions: [
    {
      id: 'mock-mouse-1',
      x: null,
      y: null,
      intervalMs: 20,
    },
  ] as MouseAction[],
  /** 全局热键配置（阶段 6 mock，阶段 12 起由 update_hotkeys 真实注册） */
  hotkeys: {
    start: { keyLabel: 'F12', scanCode: 88 },
    stop: { keyLabel: 'F12', scanCode: 88 },
  } as HotkeyConfig,
})

/** 切换当前页面（后续阶段会在此处追加 set_current_page 后端调用） */
export function setPage(page: AppPage): void {
  appStore.currentPage = page
}
