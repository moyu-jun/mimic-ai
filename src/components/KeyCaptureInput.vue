<script setup lang="ts">
/**
 * 按键捕获输入框 — 需求 3.7 / DESIGN 15.6
 * 聚焦进入捕获状态，keydown 拦截 + 查白名单，回显 label + scanCode。
 * 白名单外按键不提示，继续等待；失焦未捕获时回显原值。
 */
import { ref, watch } from 'vue'
import { lookupKey } from '../lib/keyMap'
import type { CapturedKey } from '../types/config'

interface Props {
  modelValue: CapturedKey | null
  placeholder?: string
}

interface Emits {
  (e: 'update:modelValue', value: CapturedKey): void
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: '点击捕获按键',
})

const emit = defineEmits<Emits>()

const inputRef = ref<HTMLInputElement | null>(null)
const isCapturing = ref(false)
const displayText = ref('')

watch(
  () => props.modelValue,
  (val) => {
    if (val) {
      displayText.value = val.keyLabel
    } else if (!isCapturing.value) {
      displayText.value = ''
    }
  },
  { immediate: true }
)

function onFocus(): void {
  isCapturing.value = true
  displayText.value = '请按下按键...'
}

function onBlur(): void {
  isCapturing.value = false
  // 失焦时一律以最新的 modelValue 为显示源（无论是否在捕获中变化过）
  displayText.value = props.modelValue?.keyLabel ?? ''
}

function onKeyDown(e: KeyboardEvent): void {
  if (!isCapturing.value) return

  e.preventDefault()
  e.stopPropagation()

  const key = lookupKey(e.code)
  if (key) {
    // 白名单内 → 回显并上报
    displayText.value = key.keyLabel
    emit('update:modelValue', key)
    inputRef.value?.blur()
  }
  // 白名单外 → 不提示错误，继续等待下一次按键（需求反馈 Q7）
}
</script>

<template>
  <input
    ref="inputRef"
    type="text"
    readonly
    class="key-input"
    :class="{ capturing: isCapturing }"
    :value="displayText"
    :placeholder="placeholder"
    @focus="onFocus"
    @blur="onBlur"
    @keydown="onKeyDown"
  />
</template>

<style scoped>
.key-input {
  width: 140px;
  height: 30px;
  padding: 0 10px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  font-size: 13px;
  color: var(--text-primary);
  text-align: center;
  cursor: pointer;
  transition:
    border-color var(--transition-fast) var(--ease-default),
    background var(--transition-fast) var(--ease-default);
}

.key-input::placeholder {
  color: var(--text-disabled);
  font-size: 12px;
}

.key-input:focus {
  outline: none;
  border-color: var(--accent);
  background: var(--bg-primary);
}

.key-input.capturing {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
