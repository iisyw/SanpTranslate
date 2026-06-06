# SnapTranslate v1.2.1

## 更新内容

- **改进：跨平台截图蒙版窗口逻辑** — 适配多平台（Windows/macOS/Linux）的截图蒙版窗口行为
- **改进：自定义十字准星光标** — 截图蒙版窗口新增高可见度自定义十字准星光标，提升框选精度
- **修复：GTK 依赖限制** — 将 GTK 依赖限制在 Linux 系统，防止在 Windows/macOS 中报错
- **优化：OCR 源语言选择** — 支持 OCR 源语言选择与自动检测
- **优化：贴图窗口布局与拉伸稳定性** — 深度优化贴图窗口的布局和面板拉伸功能

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
