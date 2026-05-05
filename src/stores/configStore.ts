import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getConfig, saveConfig, type AppConfig } from '@/utils/tauri'

/** 应用配置状态管理 */
export const useConfigStore = defineStore('config', () => {
  /** 当前应用配置 */
  const config = ref<AppConfig | null>(null)
  /** 是否正在加载 */
  const loading = ref(false)
  /** 错误信息 */
  const error = ref<string | null>(null)
  /** 当前 API 密钥（从 keyring 读取，仅显示是否有密钥） */
  const apiKey = ref<string | null>(null)

  /** 从后端加载配置 */
  async function loadConfig() {
    loading.value = true
    error.value = null
    try {
      config.value = await getConfig()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  /** 更新并保存配置到后端 */
  async function updateConfig(newConfig: AppConfig) {
    loading.value = true
    error.value = null
    try {
      await saveConfig(newConfig)
      config.value = newConfig
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  /** 从后端 keyring 获取 API 密钥 */
  async function loadApiKey() {
    try {
      apiKey.value = await invoke<string | null>('get_api_key')
    } catch (e) {
      error.value = String(e)
    }
  }

  /** 设置 API 密钥到后端 keyring，失败时抛出异常以便调用方处理 */
  async function setApiKey(key: string) {
    await invoke('set_api_key', { key })
    apiKey.value = key
  }

  return { config, loading, error, apiKey, loadConfig, updateConfig, loadApiKey, setApiKey }
})
