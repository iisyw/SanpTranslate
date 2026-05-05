import { invoke } from '@tauri-apps/api/core'

/** 应用配置，与后端 Rust AppConfig 结构体保持一致 */
export interface AppConfig {
  /** AI API 基础地址 */
  api_base_url: string
  /** AI 模型名称 */
  model: string
  /** 目标翻译语言 */
  target_language: string
  /** 快捷键配置 */
  shortcuts: ShortcutConfig
}

/** 快捷键配置 */
export interface ShortcutConfig {
  /** 截图快捷键 */
  capture: string
  /** 从剪贴板贴图快捷键 */
  pin_clipboard: string
}

/** 区域裁剪结果，包含图像数据和窗口位置信息 */
export interface CropResult {
  /** Base64 编码的 PNG 图像数据 */
  base64_data: string
  /** 贴图窗口 X 位置（逻辑像素） */
  x: number
  /** 贴图窗口 Y 位置（逻辑像素） */
  y: number
  /** 贴图窗口宽度（逻辑像素，含内边距） */
  width: number
  /** 贴图窗口高度（逻辑像素，含内边距和控制栏） */
  height: number
  /** 裁剪区域的物理像素宽度 */
  crop_width: number
  /** 裁剪区域的物理像素高度 */
  crop_height: number
}

/** OCR 文字块 */
export interface OcrBlock {
  /** 识别的文字 */
  text: string
  /** 左上角 X 坐标（百分比 0.0-1.0） */
  x: number
  /** 左上角 Y 坐标（百分比 0.0-1.0） */
  y: number
  /** 宽度（百分比 0.0-1.0） */
  width: number
  /** 高度（百分比 0.0-1.0） */
  height: number
}

/** 翻译结果块 */
export interface TranslatedBlock {
  /** 原始文本 */
  original: string
  /** 翻译后文本 */
  translated: string
  /** 左上角 X 坐标（百分比 0.0-1.0） */
  x: number
  /** 左上角 Y 坐标（百分比 0.0-1.0） */
  y: number
  /** 宽度（百分比 0.0-1.0） */
  width: number
  /** 高度（百分比 0.0-1.0） */
  height: number
}

/** 翻译结果，与后端 TranslateResult 对应 */
export interface TranslateResult {
  /** 翻译块列表 */
  blocks: TranslatedBlock[]
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config')
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke('save_config', { config })
}

export async function writeClipboardImage(imageData: string): Promise<void> {
  return invoke('write_clipboard_image', { imageData })
}

export async function readClipboardImage(): Promise<string | null> {
  return invoke<string | null>('read_clipboard_image')
}

export async function writeClipboardText(text: string): Promise<void> {
  return invoke('write_clipboard_text', { text })
}

export async function closePinWindow(windowId: string): Promise<void> {
  return invoke('close_pin_window', { windowId })
}

export async function getPinImage(windowId: string): Promise<string | null> {
  return invoke<string | null>('get_pin_image', { windowId })
}

// 从缓存的全屏截图中裁剪指定区域，返回裁剪结果（图像数据 + 位置信息）
export async function captureRegionFromCache(
  x: number,
  y: number,
  width: number,
  height: number
): Promise<CropResult> {
  return invoke<CropResult>('capture_region_from_cache', { x, y, width, height })
}

// 存储贴图图像数据到后端 PinImageStore，供 PinView 获取
export async function storePinImage(label: string, imageData: string): Promise<void> {
  return invoke('store_pin_image', { label, imageData })
}

/** 翻译图像，返回翻译结果 */
export async function translateImage(
  imageData: string,
  targetLanguage: string
): Promise<TranslateResult> {
  return invoke<TranslateResult>('translate_image', { imageData, targetLanguage })
}

/** 获取 API 密钥（从系统密钥环读取） */
export async function getApiKey(): Promise<string | null> {
  return invoke<string | null>('get_api_key')
}

/** 设置 API 密钥（保存到系统密钥环） */
export async function setApiKey(key: string): Promise<void> {
  return invoke('set_api_key', { key })
}

/** 测试 API 连接是否可用 */
export async function testApiConnection(
  apiBaseUrl: string,
  apiKey: string,
  model: string
): Promise<string> {
  return invoke<string>('test_api_connection', { apiBaseUrl, apiKey, model })
}
