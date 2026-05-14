<template>
  <div
    class="overlay-container"
    :style="containerStyle"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @contextmenu.prevent="onContextMenu"
  >
    <canvas ref="canvasRef" />
    <div
      v-if="isSelecting && selectionWidth > 5 && selectionHeight > 5"
      class="size-tip"
      :style="sizeTipStyle"
    >
      {{ selectionWidth }} x {{ selectionHeight }}
    </div>
    <!-- 高可见性自定义光标：白色十字 + drop-shadow 暗色轮廓，在任何背景上都清晰可见 -->
    <div class="custom-cursor" :style="{ left: cursorX + 'px', top: cursorY + 'px' }">
      <svg width="20" height="20" viewBox="0 0 20 20">
        <line x1="10" y1="2" x2="10" y2="18" stroke="#fff" stroke-width="1.5" stroke-linecap="round"/>
        <line x1="2" y1="10" x2="18" y2="10" stroke="#fff" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { captureRegionFromCache, storePinImage } from '@/utils/tauri'
import { logger } from '@/utils/logger'

const TAG = 'Overlay'
const { t } = useI18n()

const canvasRef = ref<HTMLCanvasElement | null>(null)

const isSelecting = ref(false)
const startX = ref(0)
const startY = ref(0)
const endX = ref(0)
const endY = ref(0)
let fullscreenImgElement: HTMLImageElement | null = null

let keydownHandler: ((e: KeyboardEvent) => void) | null = null
let rafId: number | null = null

const selectionWidth = computed(() => Math.abs(endX.value - startX.value))
const selectionHeight = computed(() => Math.abs(endY.value - startY.value))

const cursorX = ref(0)
const cursorY = ref(0)

const containerStyle = computed(() => ({
  cursor: 'none',
}))

const sizeTipStyle = computed(() => {
  const rectX = Math.min(startX.value, endX.value)
  const rectY = Math.min(startY.value, endY.value)
  const h = selectionHeight.value
  return {
    left: `${rectX}px`,
    top: `${rectY + h + 4}px`,
  }
})

function drawCanvas() {
  const canvas = canvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const dpr = window.devicePixelRatio || 1

  ctx.clearRect(0, 0, canvas.width, canvas.height)

  // 有截图时绘制背景图，否则只显示半透明遮罩
  if (fullscreenImgElement) {
    ctx.drawImage(fullscreenImgElement, 0, 0, canvas.width, canvas.height)
  }

  ctx.fillStyle = 'rgba(0, 0, 0, 0.5)'
  ctx.fillRect(0, 0, canvas.width, canvas.height)

  if (isSelecting.value && selectionWidth.value > 0 && selectionHeight.value > 0) {
    const rx = Math.min(startX.value, endX.value) * dpr
    const ry = Math.min(startY.value, endY.value) * dpr
    const rw = selectionWidth.value * dpr
    const rh = selectionHeight.value * dpr

    ctx.save()
    ctx.beginPath()
    ctx.rect(rx, ry, rw, rh)
    ctx.clip()
    if (fullscreenImgElement) {
      ctx.drawImage(fullscreenImgElement, 0, 0, canvas.width, canvas.height)
    }
    ctx.restore()

    ctx.save()
    ctx.strokeStyle = '#ffffff'
    ctx.lineWidth = 1 * dpr
    ctx.setLineDash([4 * dpr, 4 * dpr])
    ctx.strokeRect(rx, ry, rw, rh)
    ctx.restore()
  }
}

function onMouseDown(e: MouseEvent) {
  e.preventDefault()
  isSelecting.value = true
  startX.value = e.clientX
  startY.value = e.clientY
  endX.value = e.clientX
  endY.value = e.clientY
}

function onMouseMove(e: MouseEvent) {
  cursorX.value = e.clientX
  cursorY.value = e.clientY
  if (!isSelecting.value) return
  endX.value = e.clientX
  endY.value = e.clientY
  if (rafId === null) {
    rafId = requestAnimationFrame(() => {
      drawCanvas()
      rafId = null
    })
  }
}

async function onMouseUp(e: MouseEvent) {
  if (!isSelecting.value) return
  isSelecting.value = false
  endX.value = e.clientX
  endY.value = e.clientY

  const dpr = window.devicePixelRatio || 1

  const cssX = Math.round(Math.min(startX.value, endX.value))
  const cssY = Math.round(Math.min(startY.value, endY.value))
  const cssW = Math.round(selectionWidth.value)
  const cssH = Math.round(selectionHeight.value)

  logger.info(TAG, `鼠标松开: cssX=${cssX}, cssY=${cssY}, cssW=${cssW}, cssH=${cssH}, dpr=${dpr}`)

  if (cssW < 5 || cssH < 5) {
    logger.info(TAG, '选区太小，忽略')
    drawCanvas()
    return
  }

  // 清除选区矩形，显示纯遮罩
  drawCanvas()

  const physX = Math.round(cssX * dpr)
  const physY = Math.round(cssY * dpr)
  const physW = Math.round(cssW * dpr)
  const physH = Math.round(cssH * dpr)

  logger.info(TAG, `物理像素: physX=${physX}, physY=${physY}, physW=${physW}, physH=${physH}`)

  // 直接从 CSS 坐标计算贴图窗口位置（无需等待后端编码结果）
  const PIN_PADDING = 14
  const CONTROL_BAR_H = 36
  const windowX = cssX - PIN_PADDING
  const windowY = cssY - PIN_PADDING
  const windowWidth = cssW + PIN_PADDING * 2
  const windowHeight = cssH + CONTROL_BAR_H + PIN_PADDING * 2

  // 创建贴图窗口（定位到正确位置，visible），与后端编码并行
  const label = `pin-${crypto.randomUUID()}`
  const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')

  const pinWindow = new WebviewWindow(label, {
    url: '/pin',
    title: t('pin.title'),
    decorations: false,
    alwaysOnTop: true,
    transparent: true,
    shadow: false,
    skipTaskbar: true,
    resizable: false,
    x: windowX,
    y: windowY,
    width: windowWidth,
    height: windowHeight,
  })

  pinWindow.once('tauri://error', (err) => {
    logger.error(TAG, `贴图窗口创建失败: ${err}`, err)
  })

  // 后台：裁剪编码 + 存储图像（蒙版保持，pin 窗口位于其上方）
  try {
    const cropResult = await captureRegionFromCache(physX, physY, physW, physH)
    logger.info(TAG, `captureRegionFromCache 返回: x=${cropResult.x}, y=${cropResult.y}, w=${cropResult.width}, h=${cropResult.height}`)

    await storePinImage(label, cropResult.base64_data)
    logger.info(TAG, `图像数据已存储，label=${label}`)
  } catch (err) {
    logger.error(TAG, `框选处理失败: ${err}`, err)
  }

  // 等 IPC 完成后销毁蒙版
  getCurrentWindow().destroy().catch((err) => {
    logger.error(TAG, `销毁蒙版失败: ${err}`, err)
  })
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    logger.info(TAG, '按下 Esc，销毁 overlay 窗口')
    getCurrentWindow().destroy().catch((err) => {
      logger.error(TAG, `Esc销毁窗口失败: ${err}`, err)
    })
  }
}

function onContextMenu(e: MouseEvent) {
  e.preventDefault()
  logger.info(TAG, '右键点击，销毁 overlay 窗口')
  getCurrentWindow().destroy().catch((err) => {
    logger.error(TAG, `右键销毁窗口失败: ${err}`, err)
  })
}

function initCanvasSize() {
  const canvas = canvasRef.value
  if (!canvas) return
  const dpr = window.devicePixelRatio || 1
  canvas.width = window.innerWidth * dpr
  canvas.height = window.innerHeight * dpr
  logger.info(TAG, `Canvas初始化: innerSize=${window.innerWidth}x${window.innerHeight}, dpr=${dpr}, canvasSize=${canvas.width}x${canvas.height}`)
}

function loadFullscreenImage(dataUrl: string) {
  logger.info(TAG, `收到全屏截图数据，dataUrl长度=${dataUrl.length}`)
  const img = new Image()
  img.onload = () => {
    fullscreenImgElement = img
    logger.info(TAG, `全屏截图Image加载完成: naturalSize=${img.naturalWidth}x${img.naturalHeight}`)
    drawCanvas()
    logger.info(TAG, 'Canvas绘制完成')
  }
  img.onerror = (err) => {
    logger.error(TAG, `全屏截图Image加载失败`, err)
  }
  img.src = dataUrl
}

onMounted(async () => {
  logger.info(TAG, 'OverlayView onMounted')
  initCanvasSize()
  // 立即绘制半透明遮罩，不等截图加载完成，让用户感知蒙版已响应
  drawCanvas()

  // 轮询拉取蒙版图像数据（后台线程异步编码截图）
  const POLL_INTERVAL_MS = 100
  const MAX_POLL_ATTEMPTS = 50  // 50 * 100ms = 5s 超时
  let imageLoaded = false

  for (let i = 0; i < MAX_POLL_ATTEMPTS; i++) {
    try {
      const overlayData = await invoke<{ data: string; mime: string } | null>('get_overlay_image')
      if (overlayData) {
        logger.info(TAG, `拉取到蒙版图像数据（第${i + 1}次轮询），mime=${overlayData.mime}，数据长度=${overlayData.data.length}`)
        const dataUrl = `data:${overlayData.mime};base64,${overlayData.data}`
        loadFullscreenImage(dataUrl)
        imageLoaded = true
        break
      }
    } catch (err) {
      logger.error(TAG, `拉取蒙版图像数据失败: ${err}`, err)
      break  // 命令出错不再重试
    }

    if (i < MAX_POLL_ATTEMPTS - 1) {
      await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL_MS))
    }
  }

  // 最终额外尝试一次，覆盖极微小的竞态窗口
  if (!imageLoaded) {
    try {
      const overlayData = await invoke<{ data: string; mime: string } | null>('get_overlay_image')
      if (overlayData) {
        logger.info(TAG, `最终尝试拉取到蒙版图像数据`)
        const dataUrl = `data:${overlayData.mime};base64,${overlayData.data}`
        loadFullscreenImage(dataUrl)
        imageLoaded = true
      }
    } catch { /* 忽略最终尝试的异常 */ }
  }

  if (!imageLoaded) {
    logger.error(TAG, `轮询超时（${MAX_POLL_ATTEMPTS * POLL_INTERVAL_MS}ms），未获取到蒙版图像数据`)
  }

  keydownHandler = onKeyDown
  window.addEventListener('keydown', keydownHandler)
  logger.info(TAG, 'OverlayView 初始化完成')
})

onUnmounted(() => {
  logger.info(TAG, 'OverlayView onUnmounted')
  if (keydownHandler) {
    window.removeEventListener('keydown', keydownHandler)
    keydownHandler = null
  }
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
  fullscreenImgElement = null
})
</script>

<style scoped>
.overlay-container {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
}

.overlay-container canvas {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}

.size-tip {
  position: fixed;
  font-size: 12px;
  color: #ffffff;
  background: rgba(0, 0, 0, 0.7);
  padding: 2px 6px;
  border-radius: 0;
  pointer-events: none;
  white-space: nowrap;
  z-index: 10;
}

.custom-cursor {
  position: fixed;
  pointer-events: none;
  z-index: 9999;
  transform: translate(-50%, -50%);
  filter: drop-shadow(0 0 2px rgba(0, 0, 0, 0.8));
}

.custom-cursor svg {
  display: block;
}

</style>
