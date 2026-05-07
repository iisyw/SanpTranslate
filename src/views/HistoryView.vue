<template>
  <n-config-provider :theme="darkTheme">
    <div class="history-container">
      <!-- 头部 -->
      <div class="history-header">
        <h2 class="history-title">{{ t('history.title') }}</h2>
        <n-button
          v-if="historyStore.historyList.length > 0"
          type="error"
          size="small"
          ghost
          @click="onClearAll"
        >
          {{ t('history.clearAll') }}
        </n-button>
      </div>

      <!-- 内容区域 -->
      <n-spin :show="historyStore.loading">
        <!-- 空状态 -->
        <div v-if="!historyStore.loading && historyStore.historyList.length === 0" class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.15)" stroke-width="1.5">
            <circle cx="12" cy="12" r="10"/>
            <polyline points="12 6 12 12 16 14"/>
          </svg>
          <p>{{ t('history.empty') }}</p>
        </div>

        <!-- 历史列表 -->
        <div v-else class="history-list">
          <HistoryItem
            v-for="entry in historyStore.historyList"
            :key="entry.id"
            :entry="entry"
            @detail="onDetail"
            @copy="onCopy"
            @delete="onDelete"
          />
        </div>
      </n-spin>

      <!-- 详情弹窗 -->
      <n-modal
        v-model:show="showDetail"
        preset="card"
        :title="t('history.detail')"
        :style="{ maxWidth: '520px', width: '90%' }"
        :bordered="false"
        size="medium"
      >
        <div v-if="historyStore.currentDetail" class="detail-content">
          <!-- 缩略图（点击放大） -->
          <div class="detail-thumbnail-wrapper" @click="showImagePreview = true" style="cursor: pointer;">
            <img
              v-if="detailThumbnailUrl"
              :src="detailThumbnailUrl"
              class="detail-thumbnail"
              draggable="false"
            />
          </div>

          <!-- 原文（带复制按钮） -->
          <div class="detail-section" v-if="historyStore.currentDetail.ocr_text">
            <div class="detail-label-row">
              <span class="detail-label">{{ t('history.original') }}</span>
              <n-button text size="tiny" @click="onCopyDetailOriginal" style="color: rgba(255,255,255,0.5); font-size: 12px;">
                {{ t('common.copy') }}
              </n-button>
            </div>
            <div class="detail-text">{{ historyStore.currentDetail.ocr_text }}</div>
          </div>

          <!-- 译文（带复制按钮） -->
          <div class="detail-section">
            <div class="detail-label-row">
              <span class="detail-label">{{ t('history.translation') }}</span>
              <n-button text size="tiny" @click="onCopyDetailTranslation" style="color: rgba(255,255,255,0.5); font-size: 12px;">
                {{ t('common.copy') }}
              </n-button>
            </div>
            <div class="detail-text">{{ historyStore.currentDetail.translated_text }}</div>
          </div>

          <!-- 时间 -->
          <div class="detail-time">{{ historyStore.currentDetail.created_at }}</div>
        </div>

        <template #footer>
          <n-space justify="end">
            <n-button type="error" ghost @click="onDeleteDetail">{{ t('common.delete') }}</n-button>
          </n-space>
        </template>
      </n-modal>

      <!-- 图片放大预览弹窗 -->
      <n-modal
        v-model:show="showImagePreview"
        :style="{ maxWidth: '90vw', maxHeight: '90vh', width: 'auto', padding: '8px' }"
        :bordered="false"
        :mask-closable="true"
        @click="showImagePreview = false"
      >
        <img
          v-if="detailImageUrl"
          :src="detailImageUrl"
          style="width: auto; height: auto; max-width: 85vw; max-height: 85vh; object-fit: contain; border-radius: 8px; display: block;"
          draggable="false"
        />
      </n-modal>
    </div>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  darkTheme,
  NConfigProvider,
  NButton,
  NSpace,
  NSpin,
  NModal,
  createDiscreteApi,
} from 'naive-ui'
import { useHistoryStore } from '@/stores/historyStore'
import type { HistoryListItem } from '@/utils/tauri'
import { writeClipboardText } from '@/utils/tauri'
import { logger } from '@/utils/logger'
import HistoryItem from '@/components/HistoryItem.vue'

const TAG = 'HistoryView'
const { t } = useI18n()

// 创建独立的 message 实例，配合深色主题
const { message, dialog } = createDiscreteApi(['message', 'dialog'], {
  configProviderProps: {
    theme: darkTheme,
  },
})

const historyStore = useHistoryStore()

// 详情弹窗状态
const showDetail = ref(false)
// 图片放大预览状态
const showImagePreview = ref(false)

// 详情缩略图 URL
const detailThumbnailUrl = computed(() => {
  if (historyStore.currentDetail?.thumbnail) {
    return `data:image/jpeg;base64,${historyStore.currentDetail.thumbnail}`
  }
  return ''
})

// 详情原图 URL（用于放大预览）
const detailImageUrl = computed(() => {
  if (historyStore.currentDetail?.image_data) {
    return `data:image/png;base64,${historyStore.currentDetail.image_data}`
  }
  // 无原图时回退到缩略图（旧数据兼容）
  return detailThumbnailUrl.value
})

/** 查看详情 */
async function onDetail(entry: HistoryListItem) {
  try {
    await historyStore.loadDetail(entry.id)
    showDetail.value = true
  } catch (err) {
    message.error(t('history.detailLoadFailed'))
    logger.error(TAG, `加载详情失败: ${err}`, err)
  }
}

/** 复制翻译文本 */
async function onCopy(entry: HistoryListItem) {
  try {
    // 需要获取详情才能拿到完整翻译文本
    await historyStore.loadDetail(entry.id)
    if (historyStore.currentDetail?.translated_text) {
      await historyStore.copyTranslation(historyStore.currentDetail.translated_text)
      message.success(t('history.copySuccess'))
    }
  } catch (err) {
    message.error(t('history.copyFailed'))
    logger.error(TAG, `复制失败: ${err}`, err)
  }
}

/** 删除单条记录 */
function onDelete(id: number) {
  dialog.warning({
    title: t('common.confirm'),
    content: t('history.confirmDelete'),
    positiveText: t('common.delete'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      try {
        await historyStore.deleteHistory(id)
        message.success(t('history.deleted'))
      } catch (err) {
        message.error(t('history.deleteFailed'))
        logger.error(TAG, `删除失败: ${err}`, err)
      }
    },
  })
}

/** 清空全部历史 */
function onClearAll() {
  dialog.warning({
    title: t('common.confirm'),
    content: t('history.confirmClearAll'),
    positiveText: t('history.clearAll'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      try {
        await historyStore.clearHistory()
        message.success(t('history.cleared'))
      } catch (err) {
        message.error(t('history.clearFailed'))
        logger.error(TAG, `清空失败: ${err}`, err)
      }
    },
  })
}

/** 复制详情中的原文 */
async function onCopyDetailOriginal() {
  if (!historyStore.currentDetail?.ocr_text) return
  try {
    await writeClipboardText(historyStore.currentDetail.ocr_text)
    message.success(t('history.copySuccess'))
  } catch (err) {
    message.error(t('history.copyFailed'))
    logger.error(TAG, `复制原文失败: ${err}`, err)
  }
}

/** 复制详情中的译文 */
async function onCopyDetailTranslation() {
  if (!historyStore.currentDetail?.translated_text) return
  try {
    await writeClipboardText(historyStore.currentDetail.translated_text)
    message.success(t('history.copySuccess'))
  } catch (err) {
    message.error(t('history.copyFailed'))
    logger.error(TAG, `复制译文失败: ${err}`, err)
  }
}

/** 删除详情中的记录 */
function onDeleteDetail() {
  if (!historyStore.currentDetail) return
  const id = historyStore.currentDetail.id
  dialog.warning({
    title: t('common.confirm'),
    content: t('history.confirmDelete'),
    positiveText: t('common.delete'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      try {
        await historyStore.deleteHistory(id)
        showDetail.value = false
        message.success(t('history.deleted'))
      } catch (err) {
        message.error(t('history.deleteFailed'))
        logger.error(TAG, `删除失败: ${err}`, err)
      }
    },
  })
}

// 页面加载时获取历史记录
onMounted(async () => {
  try {
    await historyStore.loadHistory()
    logger.info(TAG, '历史记录页面初始化完成')
  } catch (err) {
    message.error(t('history.loadFailed'))
    logger.error(TAG, `加载历史记录失败: ${err}`, err)
  }
})
</script>

<style scoped>
.history-container {
  padding: 16px;
  height: 100vh;
  overflow-y: auto;
  box-sizing: border-box;
  background-color: #101014;
}

.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.history-title {
  font-size: 18px;
  font-weight: 600;
  color: #e0e0e0;
  margin: 0;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 0;
  color: rgba(255, 255, 255, 0.3);
  gap: 12px;
}

.empty-state p {
  font-size: 14px;
  margin: 0;
}

.history-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* 详情弹窗内容样式 */
.detail-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.detail-thumbnail-wrapper {
  display: flex;
  justify-content: center;
}

.detail-thumbnail {
  max-width: 100%;
  max-height: 200px;
  border-radius: 4px;
  object-fit: contain;
}

.detail-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.detail-label {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.45);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.detail-text {
  font-size: 14px;
  line-height: 1.7;
  color: #e0e0e0;
  white-space: pre-wrap;
  word-break: break-word;
  background: rgba(255, 255, 255, 0.04);
  padding: 10px 12px;
  border-radius: 6px;
}

.detail-time {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.3);
  text-align: right;
}
</style>
