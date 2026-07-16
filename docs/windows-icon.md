# Windows 图标入口

当前 Windows 临时图标的唯一可编辑来源是：

- `rust/icons/icon.png`

该 PNG 同时由运行时窗口和托盘加载；`rust/icons/icon.ico` 是从它生成的 Windows 可执行文件资源，不应独立手工修改。重新生成命令：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\generate-windows-icon.ps1
```

生成的 ICO 包含 16、20、24、32、48、64、128 和 256 像素帧。Tauri bundle 配置引用同一 PNG 及其派生 ICO，窗口创建代码在首次显示前通过 `app_icon` 模块设置同一图标。运行时不得再按额度状态替换托盘图形。

未来替换正式小鸭图标时，只替换 `rust/icons/icon.png`，再运行上述生成脚本并执行完整 Windows 构建与人工图标缓存验收；不要新增第二套窗口或托盘品牌资源。
