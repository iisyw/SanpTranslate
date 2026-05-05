import { defineStore } from 'pinia'
import { ref } from 'vue'

/** 历史记录条目 */
export interface HistoryEntry {
  /** 记录 ID */
  id: number
  /** 缩略图数据 */
  thumbnail: string
  /** OCR 识别文本 */
  ocr_text: string | null
  /** 翻译结果文本 */
  translated_text: string
  /** 创建时间 */
  created_at: string
}

/** 历史记录状态管理（占位，S5 阶段实现） */
export const useHistoryStore = defineStore('history', () => {
  /** 历史记录列表 */
  const historyList = ref<HistoryEntry[]>([])
  /** 是否正在加载 */
  const loading = ref(false)

  /** 加载历史记录 */
  async function loadHistory() {
    // S5 阶段实现
  }

  /** 删除指定历史记录 */
  async function deleteHistory(_id: number) {
    // S5 阶段实现
  }

  /** 清空所有历史记录 */
  async function clearHistory() {
    // S5 阶段实现
  }

  return { historyList, loading, loadHistory, deleteHistory, clearHistory }
})
