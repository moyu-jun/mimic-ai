/**
 * 配置持久化工具 — 阶段 9
 *
 * 封装 save_config 命令调用，统一错误处理。
 * 前端各页面在结构性变更（增/删/勾选）和数字输入提交时调用。
 */
import { invoke } from '@tauri-apps/api/core'
import { appStore } from '../stores/appStore'

/**
 * 持久化当前配置到 mimic.ini
 *
 * @throws 持久化失败时抛出错误，调用方决定是否需要进一步处理
 */
export async function persistConfig(): Promise<void> {
  try {
    await invoke('save_config', {
      config: {
        keyboardActions: appStore.keyboardActions,
        mouseActions: appStore.mouseActions,
        hotkeys: appStore.hotkeys,
      }
    })
  } catch (err) {
    // 阶段 12：运行态守卫会返回 "busy: simulation running"
    if (String(err).includes('busy')) {
      console.warn('[persistConfig] 模拟运行中，跳过持久化')
      return // 静默跳过
    }
    console.error('[persistConfig] 保存配置失败:', err)
    throw err
  }
}
