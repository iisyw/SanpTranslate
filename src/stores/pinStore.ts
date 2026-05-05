import { defineStore } from 'pinia'
import { ref } from 'vue'

/** 翻译文本块 */
export interface TranslatedBlock {
  /** 原始文本 */
  original: string
  /** 翻译后文本 */
  translated: string
  /** 文本块位置 X 坐标 */
  x: number
  /** 文本块位置 Y 坐标 */
  y: number
  /** 文本块宽度 */
  width: number
  /** 文本块高度 */
  height: number
}

/** 贴图状态 */
export interface PinState {
  /** 贴图唯一标识 */
  pinId: string
  /** 截图数据 URL */
  imageDataUrl: string
  /** 贴图位置 */
  position: { x: number; y: number }
  /** 贴图尺寸 */
  size: { width: number; height: number }
  /** 翻译状态 */
  translateStatus: 'idle' | 'translating' | 'done' | 'error'
  /** 翻译块列表 */
  translatedBlocks: TranslatedBlock[]
  /** 是否显示原文 */
  showOriginal: boolean
}

/** 贴图状态管理 */
export const usePinStore = defineStore('pin', () => {
  /** 所有贴图实例 */
  const pins = ref<Map<string, PinState>>(new Map())

  /** 添加贴图 */
  function addPin(pin: PinState) {
    pins.value.set(pin.pinId, pin)
  }

  /** 移除贴图 */
  function removePin(pinId: string) {
    pins.value.delete(pinId)
  }

  /** 获取贴图 */
  function getPin(pinId: string): PinState | undefined {
    return pins.value.get(pinId)
  }

  /** 更新贴图属性 */
  function updatePin(pinId: string, updates: Partial<PinState>) {
    const pin = pins.value.get(pinId)
    if (pin) {
      Object.assign(pin, updates)
    }
  }

  return { pins, addPin, removePin, getPin, updatePin }
})
