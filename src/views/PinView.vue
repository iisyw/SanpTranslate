<template>
  <div
    class="pin-container"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @dblclick="onDoubleClick"
  >
    <!-- 内容行：左侧截图 + 右侧译文面板 -->
    <div class="content-row">
      <div class="image-area" ref="imageArea">
        <img
          v-if="imageDataUrl"
          :src="imageDataUrl"
          class="pin-image"
          draggable="false"
          @load="onImageLoad"
        />
      </div>
      <!-- 译文面板 -->
      <div
        v-if="hasTranslation && !showOriginal"
        class="translation-panel"
      >
        <div
          v-for="(block, index) in filteredBlocks"
          :key="index"
          class="translation-item"
        >
          <div class="translation-text">{{ block.translated }}</div>
          <div v-if="index < filteredBlocks.length - 1" class="translation-separator"></div>
        </div>
      </div>
    </div>
    <ControlBar
      :translate-status="translateStatus"
      :show-original="showOriginal"
      :has-translation="hasTranslation"
      :error-message="errorMessage"
      @translate="onTranslate"
      @copy-all="onCopyAll"
      @toggle-original="onToggleOriginal"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  getPinImage,
  getConfig,
  translateImage,
  writeClipboardText,
  type TranslatedBlock,
} from '@/utils/tauri'
import { logger } from '@/utils/logger'
import ControlBar from '@/components/ControlBar.vue'

const TAG = 'PinView'

// 阴影内边距，需与后端 window/mod.rs 中的 PIN_PADDING 保持一致
const PIN_PADDING = 4
// 译文面板最大宽度
const MAX_PANEL_WIDTH = 340

type TranslateStatus = 'idle' | 'translating' | 'done' | 'error'

const imageDataUrl = ref<string>('')
const pinId = ref<string>('')
const translateStatus = ref<TranslateStatus>('idle')
const showOriginal = ref(false)
const hasTranslation = ref(false)

// 翻译相关状态
const translatedBlocks = ref<TranslatedBlock[]>([])
const errorMessage = ref<string>('')

// 过滤掉空翻译的块，避免在译文面板中显示空白项
const filteredBlocks = computed(() =>
  translatedBlocks.value.filter(b => b.translated.length > 0)
)

// 保存原始 base64 数据用于翻译
let rawBase64Data = ''

// 图片逻辑像素尺寸（用于窗口大小计算）
let logicalImageWidth = 0
let logicalImageHeight = 0
// 译文面板宽度（翻译完成后固定）
let storedPanelWidth = 0

const imageArea = ref<HTMLElement | null>(null)

let mouseDownX = 0
let mouseDownY = 0
let hasStartedDrag = false
let unlistenTranslate: UnlistenFn | null = null

/** HTML 转义，防止译文内容中出现 HTML 标签破坏布局 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
}

/**
 * 离屏测量译文面板的自然宽度
 * 创建一个与面板样式相同的隐藏元素，测量其 scrollWidth 以确定内容宽度
 */
function measurePanelWidth(blocks: TranslatedBlock[]): number {
  const el = document.createElement('div')
  el.style.cssText = `
    position: fixed; left: -9999px; top: 0;
    font-size: 13px; line-height: 1.8;
    padding: 16px; max-width: ${MAX_PANEL_WIDTH}px;
    width: max-content;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    word-break: break-word; white-space: pre-wrap;
    color: #f0f0f0;
  `
  el.innerHTML = blocks.map((b, i) => {
    const text = escapeHtml(b.translated)
    const sep = i < blocks.length - 1
      ? '<div style="height:1px;margin:10px 0;background:rgba(255,255,255,0.08)"></div>'
      : ''
    return `<div style="margin-bottom:4px">${text}${sep}</div>`
  }).join('')

  document.body.appendChild(el)
  const w = Math.min(el.scrollWidth, MAX_PANEL_WIDTH)
  document.body.removeChild(el)
  return Math.max(w, 80) // 最小宽度 80px
}

/** 根据当前状态调整窗口大小 */
async function updateWindowSize(includePanel: boolean) {
  if (!logicalImageWidth || !logicalImageHeight) return

  const controlBarH = 36
  let width = logicalImageWidth + PIN_PADDING * 2
  const height = logicalImageHeight + controlBarH + PIN_PADDING * 2

  if (includePanel && storedPanelWidth > 0) {
    width += storedPanelWidth
  }

  try {
    await getCurrentWindow().setSize(new LogicalSize(width, height))
    logger.info(TAG, `窗口大小调整: ${width}x${height} (includePanel=${includePanel})`)
  } catch (err) {
    logger.error(TAG, `窗口大小调整失败: ${err}`, err)
  }
}

async function onImageLoad(event: Event) {
  const img = event.target as HTMLImageElement
  if (!img || !img.naturalWidth || !img.naturalHeight) return

  const dpr = window.devicePixelRatio || 1
  logicalImageWidth = img.naturalWidth / dpr
  logicalImageHeight = img.naturalHeight / dpr

  logger.info(TAG, `图片加载完成: naturalSize=${img.naturalWidth}x${img.naturalHeight}, dpr=${dpr}, logicalSize=${logicalImageWidth}x${logicalImageHeight}`)

  await updateWindowSize(false)
}

onMounted(async () => {
  const currentWindow = getCurrentWindow()
  pinId.value = currentWindow.label
  logger.info(TAG, `PinView onMounted, windowLabel=${pinId.value}`)

  try {
    logger.info(TAG, `调用 getPinImage, windowId=${pinId.value}`)
    const base64Data = await getPinImage(pinId.value)
    if (base64Data) {
      logger.info(TAG, `获取到图片数据，长度=${base64Data.length}, startsWithData=${base64Data.startsWith('data:')}`)
      if (base64Data.startsWith('data:')) {
        // 去掉 data URI 前缀，保存纯 base64 数据
        rawBase64Data = base64Data.replace(/^data:image\/[^;]+;base64,/, '')
        imageDataUrl.value = base64Data
      } else {
        rawBase64Data = base64Data
        imageDataUrl.value = `data:image/png;base64,${base64Data}`
      }
      logger.info(TAG, `imageDataUrl 已设置，长度=${imageDataUrl.value.length}`)
    } else {
      logger.error(TAG, 'getPinImage 返回 null！图片数据未找到')
    }
  } catch (err) {
    logger.error(TAG, `getPinImage 调用失败: ${err}`, err)
  }

  // 监听托盘"翻译最近一张贴图"事件
  unlistenTranslate = await listen('trigger-translate', () => {
    if (translateStatus.value === 'idle' || translateStatus.value === 'error') {
      onTranslate()
    }
  })
})

function onMouseDown(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (target.closest('.control-bar button')) return

  mouseDownX = e.clientX
  mouseDownY = e.clientY
  hasStartedDrag = false
}

async function onMouseMove(e: MouseEvent) {
  if (mouseDownX === 0 && mouseDownY === 0) return
  if (hasStartedDrag) return

  const dx = e.clientX - mouseDownX
  const dy = e.clientY - mouseDownY

  if (Math.abs(dx) > 3 || Math.abs(dy) > 3) {
    hasStartedDrag = true
    try {
      await getCurrentWindow().startDragging()
    } catch (err) {
      logger.error(TAG, `startDragging 失败: ${err}`, err)
    }
  }
}

function onMouseUp() {
  mouseDownX = 0
  mouseDownY = 0
  hasStartedDrag = false
}

async function onDoubleClick(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (imageArea.value && imageArea.value.contains(target)) {
    try {
      logger.info(TAG, '双击关闭贴图窗口')
      await getCurrentWindow().destroy()
    } catch (err) {
      logger.error(TAG, `双击关闭失败: ${err}`, err)
    }
  }
}

// 调用后端翻译命令
async function onTranslate() {
  translateStatus.value = 'translating'
  errorMessage.value = ''

  try {
    // 获取配置以确定目标语言
    const config = await getConfig()
    logger.info(TAG, `开始翻译，目标语言=${config.target_language}`)

    // 调用翻译命令
    const result = await translateImage(rawBase64Data, config.target_language)

    if (!result.blocks || result.blocks.length === 0) {
      logger.info(TAG, '翻译结果为空，回到空闲状态')
      translateStatus.value = 'idle'
      return
    }

    // 保存翻译块列表
    translatedBlocks.value = result.blocks
    hasTranslation.value = true
    translateStatus.value = 'done'

    logger.info(TAG, `翻译完成，共 ${translatedBlocks.value.length} 个翻译块`)

    // 在下一帧测量面板宽度并调整窗口大小
    await nextTick()
    storedPanelWidth = measurePanelWidth(result.blocks)
    logger.info(TAG, `译文面板测量宽度: ${storedPanelWidth}px`)

    await updateWindowSize(true)
  } catch (err) {
    errorMessage.value = String(err)
    translateStatus.value = 'error'
    logger.error(TAG, `翻译失败: ${err}`, err)
  }
}

// 复制全部翻译文本到剪贴板
async function onCopyAll() {
  if (filteredBlocks.value.length > 0) {
    const text = filteredBlocks.value.map(b => b.translated).join('\n')
    try {
      await writeClipboardText(text)
      logger.info(TAG, '翻译文本已复制到剪贴板')
    } catch (err) {
      logger.error(TAG, `复制失败: ${err}`, err)
    }
  }
}

// 切换原文/译文显示
async function onToggleOriginal() {
  showOriginal.value = !showOriginal.value
  // 切换后立即调整窗口大小
  await nextTick()
  await updateWindowSize(!showOriginal.value)
}

onUnmounted(() => {
  if (unlistenTranslate) {
    unlistenTranslate()
    unlistenTranslate = null
  }
})
</script>

<style scoped>
.pin-container {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  padding: 4px;
  background: transparent;
  user-select: none;
}

.content-row {
  display: flex;
  flex-direction: row;
  flex: 1;
  min-height: 0;
}

.image-area {
  flex: 0 0 auto;
  overflow: hidden;
  position: relative;
  display: flex;
  align-items: flex-start;
  justify-content: flex-start;
  box-shadow: 0 1px 4px 0 rgba(0, 0, 0, 0.35);
}

.pin-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: fill;
  pointer-events: none;
}

/* 译文面板 */
.translation-panel {
  background: rgba(30, 30, 30, 0.92);
  border-left: 1px solid rgba(255, 255, 255, 0.12);
  padding: 16px;
  max-width: 340px;
  overflow-y: auto;
  font-size: 13px;
  line-height: 1.8;
  color: #f0f0f0;
}

.translation-item {
  margin-bottom: 4px;
}

.translation-text {
  word-break: break-word;
  white-space: pre-wrap;
}

/* 翻译块之间的分隔线 */
.translation-separator {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 10px 0;
}

/* 译文面板滚动条样式 */
.translation-panel::-webkit-scrollbar {
  width: 4px;
}

.translation-panel::-webkit-scrollbar-track {
  background: transparent;
}

.translation-panel::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
}
</style>
