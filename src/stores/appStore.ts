/**
 * 全局状态管理 — DESIGN 9.1
 *
 * 使用 Vue 3 reactive 作为轻量 store。阶段 2 仅含页面路由、运行状态与锁定标记；
 * config / driverStatus / 各动作列表等字段在阶段 3-8 逐步补全。
 */

import { reactive } from 'vue'
import type { AppPage, RuntimeStatus } from '../types/config'

export const appStore = reactive({
  /** 当前激活页面，默认首页 */
  currentPage: 'home' as AppPage,
  /** 运行状态机，默认待机 */
  runtimeStatus: 'Idle' as RuntimeStatus,
  /** 运行期锁定蒙版开关（阶段 7 接入） */
  isLocked: false,
})

/** 切换当前页面（后续阶段会在此处追加 set_current_page 后端调用） */
export function setPage(page: AppPage): void {
  appStore.currentPage = page
}
