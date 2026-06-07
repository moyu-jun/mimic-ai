/**
 * 按键映射表 — DESIGN 8.1
 * 前端 KeyboardEvent.code → { label, scanCode, accelerator }
 * 白名单：字母 / 数字 / F1-F12 / Space / Tab / Esc / 左右 Shift/Ctrl/Alt
 */

import type { CapturedKey } from '../types/config'

interface KeyMapEntry extends CapturedKey {
  code: string
}

const KEY_MAP: KeyMapEntry[] = [
  // 字母
  { code: 'KeyA', keyLabel: 'A', scanCode: 30 },
  { code: 'KeyB', keyLabel: 'B', scanCode: 48 },
  { code: 'KeyC', keyLabel: 'C', scanCode: 46 },
  { code: 'KeyD', keyLabel: 'D', scanCode: 32 },
  { code: 'KeyE', keyLabel: 'E', scanCode: 18 },
  { code: 'KeyF', keyLabel: 'F', scanCode: 33 },
  { code: 'KeyG', keyLabel: 'G', scanCode: 34 },
  { code: 'KeyH', keyLabel: 'H', scanCode: 35 },
  { code: 'KeyI', keyLabel: 'I', scanCode: 23 },
  { code: 'KeyJ', keyLabel: 'J', scanCode: 36 },
  { code: 'KeyK', keyLabel: 'K', scanCode: 37 },
  { code: 'KeyL', keyLabel: 'L', scanCode: 38 },
  { code: 'KeyM', keyLabel: 'M', scanCode: 50 },
  { code: 'KeyN', keyLabel: 'N', scanCode: 49 },
  { code: 'KeyO', keyLabel: 'O', scanCode: 24 },
  { code: 'KeyP', keyLabel: 'P', scanCode: 25 },
  { code: 'KeyQ', keyLabel: 'Q', scanCode: 16 },
  { code: 'KeyR', keyLabel: 'R', scanCode: 19 },
  { code: 'KeyS', keyLabel: 'S', scanCode: 31 },
  { code: 'KeyT', keyLabel: 'T', scanCode: 20 },
  { code: 'KeyU', keyLabel: 'U', scanCode: 22 },
  { code: 'KeyV', keyLabel: 'V', scanCode: 47 },
  { code: 'KeyW', keyLabel: 'W', scanCode: 17 },
  { code: 'KeyX', keyLabel: 'X', scanCode: 45 },
  { code: 'KeyY', keyLabel: 'Y', scanCode: 21 },
  { code: 'KeyZ', keyLabel: 'Z', scanCode: 44 },
  // 数字
  { code: 'Digit1', keyLabel: '1', scanCode: 2 },
  { code: 'Digit2', keyLabel: '2', scanCode: 3 },
  { code: 'Digit3', keyLabel: '3', scanCode: 4 },
  { code: 'Digit4', keyLabel: '4', scanCode: 5 },
  { code: 'Digit5', keyLabel: '5', scanCode: 6 },
  { code: 'Digit6', keyLabel: '6', scanCode: 7 },
  { code: 'Digit7', keyLabel: '7', scanCode: 8 },
  { code: 'Digit8', keyLabel: '8', scanCode: 9 },
  { code: 'Digit9', keyLabel: '9', scanCode: 10 },
  { code: 'Digit0', keyLabel: '0', scanCode: 11 },
  // F1-F12
  { code: 'F1', keyLabel: 'F1', scanCode: 59 },
  { code: 'F2', keyLabel: 'F2', scanCode: 60 },
  { code: 'F3', keyLabel: 'F3', scanCode: 61 },
  { code: 'F4', keyLabel: 'F4', scanCode: 62 },
  { code: 'F5', keyLabel: 'F5', scanCode: 63 },
  { code: 'F6', keyLabel: 'F6', scanCode: 64 },
  { code: 'F7', keyLabel: 'F7', scanCode: 65 },
  { code: 'F8', keyLabel: 'F8', scanCode: 66 },
  { code: 'F9', keyLabel: 'F9', scanCode: 67 },
  { code: 'F10', keyLabel: 'F10', scanCode: 68 },
  { code: 'F11', keyLabel: 'F11', scanCode: 87 },
  { code: 'F12', keyLabel: 'F12', scanCode: 88 },
  // 功能键
  { code: 'Space', keyLabel: 'Space', scanCode: 57 },
  { code: 'Tab', keyLabel: 'Tab', scanCode: 15 },
  { code: 'Escape', keyLabel: 'Esc', scanCode: 1 },
  // 修饰键（阶段 13 恢复：Interception 支持修饰键作为独立热键）
  // 注意：Right Ctrl/Alt 的 scanCode 包含 E0 前缀位，Interception 内部处理
  { code: 'ShiftLeft', keyLabel: 'Left Shift', scanCode: 42 },
  { code: 'ShiftRight', keyLabel: 'Right Shift', scanCode: 54 },
  { code: 'ControlLeft', keyLabel: 'Left Ctrl', scanCode: 29 },
  { code: 'ControlRight', keyLabel: 'Right Ctrl', scanCode: 157 },
  { code: 'AltLeft', keyLabel: 'Left Alt', scanCode: 56 },
  { code: 'AltRight', keyLabel: 'Right Alt', scanCode: 184 },
]

const codeToEntry = new Map(KEY_MAP.map(e => [e.code, e]))

/** 根据 KeyboardEvent.code 查询映射；白名单外返回 null */
export function lookupKey(code: string): CapturedKey | null {
  const entry = codeToEntry.get(code)
  return entry ? { keyLabel: entry.keyLabel, scanCode: entry.scanCode } : null
}
