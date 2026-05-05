// OCR模块 - 基于Tesseract CLI的本地文字识别

use crate::error::AppError;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

/// OCR识别的文字块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBlock {
    /// 识别的文字
    pub text: String,
    /// 左上角 X 坐标（百分比 0.0-1.0）
    pub x: f64,
    /// 左上角 Y 坐标（百分比 0.0-1.0）
    pub y: f64,
    /// 宽度（百分比 0.0-1.0）
    pub width: f64,
    /// 高度（百分比 0.0-1.0）
    pub height: f64,
}

/// 去除Windows UNC长路径前缀（\\?\），Tesseract等外部程序无法识别该前缀
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\?\") {
        PathBuf::from(&s[4..])
    } else {
        path
    }
}

/// 从Tauri应用资源目录定位tesseract.exe路径
/// 优先查找资源目录中的捆绑版本，开发模式下回退到系统PATH
pub fn find_tesseract_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    // 优先从应用资源目录查找捆绑的Tesseract
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::OcrError(format!("获取资源目录失败: {}", e)))?;
    let bundled_path = strip_unc_prefix(
        resource_dir
            .join("resources")
            .join("tesseract")
            .join("tesseract.exe"),
    );

    if bundled_path.exists() {
        log::debug!("[OCR] 使用资源目录中的Tesseract: {}", bundled_path.display());
        return Ok(bundled_path);
    }

    log::debug!(
        "[OCR] 资源目录中未找到Tesseract: {}，尝试系统PATH",
        bundled_path.display()
    );

    // 开发模式下回退到系统PATH中查找tesseract.exe
    let output = Command::new("where")
        .arg("tesseract.exe")
        .output()
        .map_err(|_| {
            AppError::OcrError(
                "未找到Tesseract可执行文件：资源目录和系统PATH中均不存在".to_string(),
            )
        })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(first_line) = stdout.lines().next() {
            let path = PathBuf::from(first_line.trim());
            if path.exists() {
                log::debug!("[OCR] 使用系统PATH中的Tesseract: {}", path.display());
                return Ok(path);
            }
        }
    }

    Err(AppError::OcrError(format!(
        "未找到Tesseract可执行文件，已尝试路径: {} 和系统PATH",
        bundled_path.display()
    )))
}

/// 从Tauri应用资源目录定位tessdata目录路径
/// 优先查找资源目录中的捆绑版本，开发模式下回退到系统路径
/// 返回None表示未找到，调用方可不传--tessdata-dir参数让Tesseract使用默认搜索路径
pub fn find_tessdata_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    // 优先从应用资源目录查找捆绑的tessdata
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled_path = strip_unc_prefix(
            resource_dir
                .join("resources")
                .join("tesseract")
                .join("tessdata"),
        );
        if bundled_path.exists() {
            log::debug!("[OCR] 使用资源目录中的tessdata: {}", bundled_path.display());
            return Some(bundled_path);
        }
        log::debug!(
            "[OCR] 资源目录中未找到tessdata: {}",
            bundled_path.display()
        );
    }

    // 尝试TESSDATA_PREFIX环境变量（标准约定：TESSDATA_PREFIX/tessdata/）
    if let Ok(prefix) = std::env::var("TESSDATA_PREFIX") {
        let path = PathBuf::from(prefix).join("tessdata");
        if path.exists() {
            log::debug!("[OCR] 使用TESSDATA_PREFIX中的tessdata: {}", path.display());
            return Some(path);
        }
    }

    // 尝试Windows上Tesseract的常见安装路径
    let common_paths = [
        PathBuf::from("C:\\Program Files\\Tesseract-OCR\\tessdata"),
        PathBuf::from("C:\\Program Files (x86)\\Tesseract-OCR\\tessdata"),
    ];

    for path in &common_paths {
        if path.exists() {
            log::debug!("[OCR] 使用系统常见路径中的tessdata: {}", path.display());
            return Some(path.clone());
        }
    }

    log::warn!("[OCR] 未找到tessdata目录，将使用Tesseract默认搜索路径");
    None
}

/// 调用Tesseract CLI提取图像中的文字及坐标
pub async fn extract_text_with_positions(
    app: &tauri::AppHandle,
    image_base64: &str,
    lang: &str,
) -> Result<Vec<OcrBlock>, AppError> {
    // 获取Tesseract可执行文件路径
    let tesseract_path = find_tesseract_path(app)?;

    // 获取tessdata目录路径（可选，未找到时不传--tessdata-dir参数）
    let tessdata_path = find_tessdata_path(app);

    // 解码Base64图像数据
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| AppError::OcrError(format!("Base64解码失败: {}", e)))?;

    // 加载图像并获取尺寸（用于将像素坐标转换为百分比）
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| AppError::OcrError(format!("图像解码失败: {}", e)))?;
    let (img_width, img_height) = (img.width() as f64, img.height() as f64);

    log::debug!("[OCR] 图像尺寸: {}x{}", img_width, img_height);

    // 生成临时文件路径（使用UUID避免并发冲突）
    let temp_dir = std::env::temp_dir();
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let input_path = temp_dir.join(format!("snap_ocr_{}.png", uuid_str));
    let output_base = temp_dir.join(format!("snap_ocr_{}", uuid_str));

    // 将图像写入临时PNG文件
    img.save_with_format(&input_path, image::ImageFormat::Png)
        .map_err(|e| AppError::OcrError(format!("临时图像文件写入失败: {}", e)))?;

    log::debug!("[OCR] 临时图像文件: {}", input_path.display());

    // 在阻塞线程中执行Tesseract命令（避免阻塞异步运行时）
    let lang_owned = lang.to_string();
    let input_path_clone = input_path.clone();
    let output_base_clone = output_base.clone();
    let tsv_content = tauri::async_runtime::spawn_blocking(move || {
        execute_tesseract_and_read_tsv(
            &tesseract_path,
            &input_path_clone,
            &output_base_clone,
            &lang_owned,
            tessdata_path.as_ref(),
        )
    })
    .await
    .map_err(|e| AppError::OcrError(format!("Tesseract执行任务失败: {}", e)))??;

    // 解析TSV内容为OcrBlock列表
    let blocks = parse_tsv(&tsv_content, img_width, img_height)?;

    // 清理临时文件
    cleanup_temp_files(&input_path, &output_base);

    log::info!("[OCR] 识别到 {} 个文字块", blocks.len());
    Ok(blocks)
}

/// 执行Tesseract命令并读取TSV输出文件
fn execute_tesseract_and_read_tsv(
    tesseract_path: &PathBuf,
    input_path: &PathBuf,
    output_base: &PathBuf,
    lang: &str,
    tessdata_dir: Option<&PathBuf>,
) -> Result<String, AppError> {
    // 构建Tesseract命令
    let mut cmd = Command::new(tesseract_path);
    cmd.arg(input_path)
        .arg(output_base)
        .arg("-l")
        .arg(lang)
        .arg("--oem")
        .arg("1")
        .arg("-c")
        .arg("tessedit_create_tsv=1");

    // 如果指定了tessdata目录，添加--tessdata-dir参数
    if let Some(dir) = tessdata_dir {
        cmd.arg("--tessdata-dir").arg(dir);
    }

    log::debug!("[OCR] 执行命令: {:?}", cmd);

    // 执行Tesseract命令
    let output = cmd
        .output()
        .map_err(|e| AppError::OcrError(format!("Tesseract启动失败: {}", e)))?;

    // 记录stderr输出（可能包含警告信息）
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        log::debug!("[OCR] Tesseract stderr: {}", stderr.trim());
    }

    // 检查Tesseract执行结果
    if !output.status.success() {
        return Err(AppError::OcrError(format!(
            "Tesseract执行失败（退出码: {}）: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        )));
    }

    // 读取TSV输出文件
    let tsv_path = output_base.with_extension("tsv");
    let tsv_content = std::fs::read_to_string(&tsv_path).map_err(|e| {
        AppError::OcrError(format!(
            "读取TSV输出文件失败: {}，路径: {}",
            e,
            tsv_path.display()
        ))
    })?;

    Ok(tsv_content)
}

/// 解析Tesseract TSV输出为OcrBlock列表
/// TSV列: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
/// 将词级(level=5)结果按行号(line_num)合并为行级块，以便翻译时按行匹配
fn parse_tsv(tsv_content: &str, img_width: f64, img_height: f64) -> Result<Vec<OcrBlock>, AppError> {
    use std::collections::BTreeMap;

    /// 行内词的中间数据
    struct WordInfo {
        text: String,
        left: f64,
        top: f64,
        width: f64,
        height: f64,
    }

    // 按行号收集词级数据
    let mut line_words: BTreeMap<i32, Vec<WordInfo>> = BTreeMap::new();

    for line in tsv_content.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 12 {
            continue;
        }

        let level: i32 = match fields[0].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        // 只处理词级(level=5)
        if level != 5 {
            continue;
        }

        let left: f64 = fields[6].trim().parse().unwrap_or(0.0);
        let top: f64 = fields[7].trim().parse().unwrap_or(0.0);
        let width: f64 = fields[8].trim().parse().unwrap_or(0.0);
        let height: f64 = fields[9].trim().parse().unwrap_or(0.0);
        let text = fields[11].trim().to_string();

        // 跳过空文本
        if text.is_empty() {
            continue;
        }

        // 按行号分组
        let line_num: i32 = fields[4].trim().parse().unwrap_or(0);
        line_words.entry(line_num).or_default().push(WordInfo {
            text,
            left,
            top,
            width,
            height,
        });
    }

    // 将每行的词合并为一个行级OcrBlock
    let mut blocks = Vec::new();
    for (_line_num, mut words) in line_words {
        if words.is_empty() {
            continue;
        }

        // 按left坐标排序，确保文字从左到右拼接
        words.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal));

        // 拼接行文本
        let text = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");

        // 计算行级边界框
        let min_left = words.iter().map(|w| w.left).fold(f64::MAX, f64::min);
        let min_top = words.iter().map(|w| w.top).fold(f64::MAX, f64::min);
        let max_right = words
            .iter()
            .map(|w| w.left + w.width)
            .fold(f64::MIN, f64::max);
        let max_bottom = words
            .iter()
            .map(|w| w.top + w.height)
            .fold(f64::MIN, f64::max);

        let bbox_width = max_right - min_left;
        let bbox_height = max_bottom - min_top;

        // 转换为百分比坐标
        let x = if img_width > 0.0 { min_left / img_width } else { 0.0 };
        let y = if img_height > 0.0 { min_top / img_height } else { 0.0 };
        let w = if img_width > 0.0 { bbox_width / img_width } else { 0.0 };
        let h = if img_height > 0.0 { bbox_height / img_height } else { 0.0 };

        blocks.push(OcrBlock {
            text,
            x,
            y,
            width: w,
            height: h,
        });
    }

    Ok(blocks)
}

/// 清理Tesseract执行过程中产生的临时文件
fn cleanup_temp_files(input_path: &PathBuf, output_base: &PathBuf) {
    // 删除输入图像文件
    if let Err(e) = std::fs::remove_file(input_path) {
        log::warn!("[OCR] 删除临时图像文件失败: {}", e);
    }

    // 删除Tesseract可能生成的输出文件（.txt, .tsv等）
    for ext in &["txt", "tsv"] {
        let path = output_base.with_extension(ext);
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("[OCR] 删除临时文件 {} 失败: {}", path.display(), e);
            }
        }
    }
}
