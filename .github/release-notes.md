# SnapTranslate ${{ github.ref_name }}

## v1.0.1 更新内容

- **修复：** 截屏蒙版光标在白色背景下不可见的问题 — 改用白色十字 + `drop-shadow` 暗色轮廓的自定义光标，在任何背景色上都清晰可见
- **跨平台兼容：** 自定义光标方案已通过 Windows / macOS / Linux 三平台验证

## 功能特性

- **框选截图翻译** — `Ctrl+Alt+L` 框选屏幕区域，自动贴图到原位
- **剪贴板贴图** — `Ctrl+Alt+P` 从剪贴板贴图翻译
- **文本翻译** — `Ctrl+Alt+M` 打开文本翻译窗口
- **本地 OCR** — 内置 Tesseract 离线引擎，无需联网
- **AI 翻译** — 支持任意 OpenAI 兼容 API
- **智能缓存** — 相同内容自动匹配缓存，跳过 API 调用
- **翻译历史** — 本地 SQLite 存储，支持查看/复制/删除
- **双语界面** — 简体中文 / English，支持跟随系统

## 安装说明

### Windows
- 运行 `SnapTranslate_*_x64-setup.exe`（NSIS 安装包）
- 或双击 `SnapTranslate_*_x64_en-US.msi`（MSI 安装包）

### macOS
- 打开 `.dmg` 文件，将 SnapTranslate 拖入 Applications 文件夹

### Linux
```bash
# Debian/Ubuntu
sudo dpkg -i SnapTranslate_*_amd64.deb

# AppImage
chmod +x SnapTranslate_*_amd64.AppImage
./SnapTranslate_*_amd64.AppImage
```

## 使用前配置

首次使用需要配置 AI API：

1. 右键系统托盘图标 → **设置**
2. 填写 API 地址、密钥和模型名称
3. 选择目标翻译语言
4. 保存即可开始使用

> API 密钥通过操作系统凭据管理器安全存储，不会写入配置文件。

## 更新日志

请查看 [CHANGELOG](CHANGELOG.md) 获取详细更新内容。
