<template>
  <div
    class="pin-container"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @dblclick="onDoubleClick"
  >
    <div class="image-area" ref="imageArea">
      <img
        v-if="imageDataUrl"
        :src="imageDataUrl"
        class="pin-image"
        draggable="false"
        @load="onImageLoad"
      />
      <!-- 翻译覆盖层 -->
      <div v-if="translatedBlocks.length > 0 && !showOriginal" class="translate-overlay">
        <div
          v-for="(block, index) in translatedBlocks"
          :key="index"
          class="translate-label"
          :style="{
            left: (block.x * 100) + '%',
            top: (block.y * 100) + '%',
            width: (block.width * 100) + '%',
            height: (block.height * 100) + '%',
          }"
          :title="block.original"
        >
          {{ block.translated }}
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
import { ref, onMounted, onUnmounted } from 'vue'
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

type TranslateStatus = 'idle' | 'translating' | 'done' | 'error'

const imageDataUrl = ref<string>('')
const pinId = ref<string>('')
const translateStatus = ref<TranslateStatus>('idle')
const showOriginal = ref(false)
const hasTranslation = ref(false)

// 翻译相关状态
const translatedBlocks = ref<TranslatedBlock[]>([])
const errorMessage = ref<string>('')

// 保存原始 base64 数据用于翻译
let rawBase64Data = ''

const imageArea = ref<HTMLElement | null>(null)

let mouseDownX = 0
let mouseDownY = 0
let hasStartedDrag = false
let unlistenTranslate: UnlistenFn | null = null

async function onImageLoad(event: Event) {
  const img = event.target as HTMLImageElement
  if (!img || !img.naturalWidth || !img.naturalHeight) return

  const dpr = window.devicePixelRatio || 1
  const logicalW = img.naturalWidth / dpr
  const logicalH = img.naturalHeight / dpr
  const controlBarH = 36

  logger.info(TAG, `图片加载完成: naturalSize=${img.naturalWidth}x${img.naturalHeight}, dpr=${dpr}, logicalSize=${logicalW}x${logicalH}`)

  try {
    await getCurrentWindow().setSize(new LogicalSize(
      logicalW + PIN_PADDING * 2,
      logicalH + controlBarH + PIN_PADDING * 2
    ))
    logger.info(TAG, `窗口大小调整成功: ${logicalW + PIN_PADDING * 2}x${logicalH + controlBarH + PIN_PADDING * 2}`)
  } catch (err) {
    logger.error(TAG, `窗口大小调整失败: ${err}`, err)
  }
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

    // 保存翻译块列表
    translatedBlocks.value = result.blocks
    logger.info(TAG, `翻译完成，共 ${translatedBlocks.value.length} 个翻译块`)

    hasTranslation.value = true
    translateStatus.value = 'done'
  } catch (err) {
    errorMessage.value = String(err)
    translateStatus.value = 'error'
    logger.error(TAG, `翻译失败: ${err}`, err)
  }
}

// 复制全部翻译文本到剪贴板
async function onCopyAll() {
  if (translatedBlocks.value.length > 0) {
    const text = translatedBlocks.value.map(b => b.translated).join('\n')
    try {
      await writeClipboardText(text)
      logger.info(TAG, '翻译文本已复制到剪贴板')
    } catch (err) {
      logger.error(TAG, `复制失败: ${err}`, err)
    }
  }
}

// 切换原文/译文显示
function onToggleOriginal() {
  showOriginal.value = !showOriginal.value
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

.image-area {
  flex: 1;
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

/* 翻译覆盖层 */
.translate-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

/* 单个翻译标签 */
.translate-label {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.75);
  color: #ffffff;
  font-size: 12px;
  line-height: 1.2;
  padding: 1px 3px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  pointer-events: auto;
  cursor: default;
  border-radius: 2px;
  transition: background 0.15s;
}

/* 鼠标悬停时展开显示完整译文 */
.translate-label:hover {
  background: rgba(0, 0, 0, 0.9);
  white-space: normal;
  word-break: break-all;
  z-index: 10;
}
</style>
