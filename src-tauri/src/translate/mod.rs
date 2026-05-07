// 翻译模块

use crate::error::AppError;
use crate::ocr;
use serde::{Deserialize, Serialize};

/// 默认OCR识别语言（中文简体+英文）
const DEFAULT_OCR_LANG: &str = "chi_sim+eng";

/// 翻译结果块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedBlock {
    /// 原始文本
    pub original: String,
    /// 翻译后文本
    pub translated: String,
    /// 左上角 X 坐标（百分比 0.0-1.0）
    pub x: f64,
    /// 左上角 Y 坐标（百分比 0.0-1.0）
    pub y: f64,
    /// 宽度（百分比 0.0-1.0）
    pub width: f64,
    /// 高度（百分比 0.0-1.0）
    pub height: f64,
}

/// 翻译结果
#[derive(Debug, Clone, Serialize)]
pub struct TranslateResult {
    /// 翻译块列表
    pub blocks: Vec<TranslatedBlock>,
}

/// 翻译图像入口函数，使用OCR模式提取文字并翻译
pub async fn translate_image(
    app: &tauri::AppHandle,
    image_base64: &str,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    target_language: &str,
    ocr_lang: Option<&str>,
) -> Result<TranslateResult, AppError> {
    let lang = ocr_lang.unwrap_or(DEFAULT_OCR_LANG);
    translate_ocr_mode(app, image_base64, api_base_url, api_key, model, target_language, lang).await
}

/// OCR模式翻译：先本地Tesseract OCR提取文字及坐标，再翻译文本，最后合并坐标
async fn translate_ocr_mode(
    app: &tauri::AppHandle,
    image_base64: &str,
    api_base_url: &str,
    api_key: &str,
    model: &str,
    target_language: &str,
    ocr_lang: &str,
) -> Result<TranslateResult, AppError> {
    // 调用本地Tesseract OCR提取文字及坐标
    let ocr_blocks = ocr::extract_text_with_positions(app, image_base64, ocr_lang).await?;

    if ocr_blocks.is_empty() {
        log::info!("[TRANSLATE] OCR未识别到文字，返回空结果");
        return Ok(TranslateResult { blocks: Vec::new() });
    }

    // 拼接所有OCR文字，用空行分隔（每个段落对应一个OCR块）
    let all_text = ocr_blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    log::debug!("[TRANSLATE] OCR提取文本（{}段落）: {}", ocr_blocks.len(), all_text);

    // 调用文本模型翻译，要求按段落返回
    let translated_text = call_text_api(api_base_url, api_key, model, &all_text, target_language).await?;

    // 将翻译结果按空行(\n\n)拆分为段落，与OCR块一一对应
    let translated_paragraphs: Vec<&str> = translated_text
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    log::debug!("[TRANSLATE] 翻译结果（{}段落）: {}", translated_paragraphs.len(), translated_text);

    // 警告：翻译段落数与OCR块数不匹配的情况
    if translated_paragraphs.len() != ocr_blocks.len() {
        log::warn!(
            "[TRANSLATE] 翻译段落数({})与OCR块数({})不匹配",
            translated_paragraphs.len(),
            ocr_blocks.len()
        );
    }

    let translated_blocks: Vec<TranslatedBlock> = ocr_blocks
        .into_iter()
        .enumerate()
        .map(|(i, block)| {
            // 如果翻译段落数与OCR块数不匹配，缺少的段落用空字符串代替
            // 避免原文英文混入译文面板
            let translated = if i < translated_paragraphs.len() {
                translated_paragraphs[i].to_string()
            } else {
                String::new()
            };
            TranslatedBlock {
                original: block.text,
                translated,
                x: block.x,
                y: block.y,
                width: block.width,
                height: block.height,
            }
        })
        .collect();

    log::info!("[TRANSLATE] OCR模式翻译完成，共 {} 个块", translated_blocks.len());
    Ok(TranslateResult { blocks: translated_blocks })
}

/// 调用文本模型API（OpenAI兼容格式）
pub async fn call_text_api(
    api_base_url: &str,
    api_key: &str,
    model: &str,
    text: &str,
    target_language: &str,
) -> Result<String, AppError> {
    let url = format!("{}/chat/completions", api_base_url.trim_end_matches('/'));

    // 构建文本模型请求体
    let request_body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "你是翻译助手。用户会发送多段文本，段落之间用空行分隔。请逐段翻译，每段翻译结果单独用空行分隔，段落数量必须与原文完全一致。保持原文中的换行结构不变。不要合并、拆分或增减段落。"
            },
            {
                "role": "user",
                "content": format!("将以下文本翻译为{}：\n{}", target_language, text)
            }
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::TranslateError(format!(
            "文本模型API请求失败，状态码: {}，响应: {}",
            status, body
        )));
    }

    let response_json: serde_json::Value = response.json().await?;

    // 提取响应中的文本内容
    response_json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::TranslateError("文本模型响应中缺少content字段".to_string()))
}
