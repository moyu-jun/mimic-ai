/**
 * 前后端共享类型定义 — 来源 DESIGN.md 4.1
 *
 * 阶段 2 仅需要 AppPage 与 RuntimeStatus；其余类型（KeyboardAction /
 * MouseAction / CapturedKey / HotkeyConfig / AppConfig / DriverStatus）
 * 在阶段 4-8 按需补全，此处先不引入以避免未使用告警。
 */

/** 左侧菜单的四个页面标识 */
export type AppPage = 'home' | 'keyboard' | 'mouse' | 'settings'

/** 运行状态机（DESIGN 3.10 / 15.3-bis） */
export type RuntimeStatus =
  | 'Idle'
  | 'ReadyKeyboard'
  | 'ReadyMouse'
  | 'RunningKeyboard'
  | 'RunningMouse'
  | 'PickingMouse'
  | 'Error'
