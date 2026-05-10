<template>
  <div
    class="shortcut-input"
    :class="{ recording: isRecording }"
    tabindex="0"
    @keydown="onKeyDown"
    @keyup="onKeyUp"
    @focus="onFocus"
    @blur="onBlur"
    ref="inputRef"
  >
    <span v-if="isRecording" class="shortcut-hint">{{ t('settings.pressShortcut') }}</span>
    <span v-else-if="displayValue" class="shortcut-value">{{ displayValue }}</span>
    <span v-else class="shortcut-placeholder">{{ placeholder }}</span>
    <span v-if="isRecording" class="recording-dot"></span>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const inputRef = ref<HTMLElement | null>(null)
const isRecording = ref(false)
const pressedModifiers = ref<Set<string>>(new Set())
const currentKey = ref<string | null>(null)

// 显示值：优先显示格式化后的快捷键
const displayValue = computed(() => {
  if (!props.modelValue) return ''
  return formatShortcut(props.modelValue)
})

// 支持的普通按键映射（event.code -> 显示名称）
const keyCodeMap: Record<string, string> = {
  // 字母键
  KeyA: 'A', KeyB: 'B', KeyC: 'C', KeyD: 'D', KeyE: 'E',
  KeyF: 'F', KeyG: 'G', KeyH: 'H', KeyI: 'I', KeyJ: 'J',
  KeyK: 'K', KeyL: 'L', KeyM: 'M', KeyN: 'N', KeyO: 'O',
  KeyP: 'P', KeyQ: 'Q', KeyR: 'R', KeyS: 'S', KeyT: 'T',
  KeyU: 'U', KeyV: 'V', KeyW: 'W', KeyX: 'X', KeyY: 'Y',
  KeyZ: 'Z',
  // 数字键
  Digit0: '0', Digit1: '1', Digit2: '2', Digit3: '3', Digit4: '4',
  Digit5: '5', Digit6: '6', Digit7: '7', Digit8: '8', Digit9: '9',
  // 功能键
  F1: 'F1', F2: 'F2', F3: 'F3', F4: 'F4', F5: 'F5',
  F6: 'F6', F7: 'F7', F8: 'F8', F9: 'F9', F10: 'F10',
  F11: 'F11', F12: 'F12',
}

// 判断是否为修饰键
function isModifierKey(code: string): boolean {
  return code.startsWith('Control') || code.startsWith('Alt') ||
         code.startsWith('Shift') || code.startsWith('Meta')
}

// 获取修饰键的规范化名称
function getModifierName(code: string): string {
  if (code.startsWith('Control')) return 'Control'
  if (code.startsWith('Alt')) return 'Alt'
  if (code.startsWith('Shift')) return 'Shift'
  if (code.startsWith('Meta')) return 'Meta'
  return code
}

// 格式化快捷键字符串为显示格式
function formatShortcut(shortcut: string): string {
  return shortcut
    .split('+')
    .map(part => {
      const trimmed = part.trim()
      const lower = trimmed.toLowerCase()
      if (lower === 'ctrl' || lower === 'control') return 'Ctrl'
      if (lower === 'alt') return 'Alt'
      if (lower === 'shift') return 'Shift'
      if (lower === 'super' || lower === 'win' || lower === 'meta') return 'Win'
      return trimmed.toUpperCase()
    })
    .join(' + ')
}

// 构建快捷键字符串（后端格式）
function buildShortcutString(modifiers: Set<string>, key: string | null): string | null {
  if (!key) return null

  const parts: string[] = []
  // 按固定顺序排列修饰键
  if (modifiers.has('Control')) parts.push('Ctrl')
  if (modifiers.has('Alt')) parts.push('Alt')
  if (modifiers.has('Shift')) parts.push('Shift')
  if (modifiers.has('Meta')) parts.push('Win')

  // 至少需要一个修饰键
  if (parts.length === 0) return null

  parts.push(key)
  return parts.join('+')
}

function onKeyDown(e: KeyboardEvent) {
  if (!isRecording.value) return

  e.preventDefault()
  e.stopPropagation()

  const code = e.code

  // Escape 取消录制
  if (code === 'Escape') {
    stopRecording()
    return
  }

  // 处理修饰键
  if (isModifierKey(code)) {
    pressedModifiers.value.add(getModifierName(code))
    return
  }

  // 处理普通按键
  const keyName = keyCodeMap[code]
  if (keyName) {
    currentKey.value = keyName
    // 构建快捷键字符串
    const shortcut = buildShortcutString(pressedModifiers.value, currentKey.value)
    if (shortcut) {
      emit('update:modelValue', shortcut)
      // 短暂延迟后停止录制，让用户看到结果
      setTimeout(() => stopRecording(), 150)
    }
  }
}

function onKeyUp(e: KeyboardEvent) {
  if (!isRecording.value) return

  e.preventDefault()
  e.stopPropagation()

  const code = e.code

  // 修饰键释放时从集合中移除
  if (isModifierKey(code)) {
    pressedModifiers.value.delete(getModifierName(code))
  }
}

function onFocus() {
  isRecording.value = true
  pressedModifiers.value.clear()
  currentKey.value = null
}

function onBlur() {
  // 失焦时停止录制
  stopRecording()
}

function stopRecording() {
  isRecording.value = false
  pressedModifiers.value.clear()
  currentKey.value = null
}

// 全局点击事件：点击组件外部时取消录制
function onGlobalClick(e: MouseEvent) {
  if (inputRef.value && !inputRef.value.contains(e.target as Node)) {
    if (isRecording.value) {
      stopRecording()
    }
  }
}

onMounted(() => {
  document.addEventListener('click', onGlobalClick)
})

onUnmounted(() => {
  document.removeEventListener('click', onGlobalClick)
})
</script>

<style scoped>
.shortcut-input {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  min-height: 34px;
  min-width: 180px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 3px;
  background-color: rgba(255, 255, 255, 0.06);
  cursor: pointer;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
  user-select: none;
}

.shortcut-input:hover {
  border-color: rgba(255, 255, 255, 0.24);
}

.shortcut-input.recording {
  border-color: #63e2b7;
  box-shadow: 0 0 0 2px rgba(99, 226, 183, 0.2);
}

.shortcut-value {
  color: rgba(255, 255, 255, 0.82);
  font-size: 14px;
  font-family: 'Consolas', 'Monaco', monospace;
  letter-spacing: 0.5px;
}

.shortcut-placeholder {
  color: rgba(255, 255, 255, 0.3);
  font-size: 14px;
}

.shortcut-hint {
  color: #63e2b7;
  font-size: 13px;
}

.recording-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #63e2b7;
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
</style>
