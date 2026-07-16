# Codex Duck Overlay 吉祥物架构预留

本文档只描述未来方向。本基线不实现小鸭动画、不修改当前 Overlay UI，也不增加资源依赖。

## 目标与边界

- 吉祥物是 Codex 用量状态的可选表现层，不是 provider、认证或刷新逻辑的一部分。
- 动画故障不能阻止额度读取、托盘、设置、刷新或窗口关闭。
- 无动画资源、资源损坏、低性能或 reduced motion 环境必须安全降级。
- 第一版只考虑 Windows；本文件不承诺多平台实现。

## 建议的数据流

```text
CodexQuotaSnapshot.windows
  -> MascotStateMapper（纯函数，无 IO）
  -> MascotViewModel
  -> MascotRenderer（PNG 序列帧或 Sprite Sheet）
```

`MascotState` 必须定义在独立前端模块中，不能导入 Codex provider 或 App Server 原始返回结构，也不能调用 OAuth、文件系统或 Tauri 刷新命令。状态映射只消费稳定的 `CodexQuotaSnapshot` 和少量纯 UI 上下文，例如当前时间、reduced motion 和一次性庆祝触发器。

当前已预留纯函数 `getMascotQuotaInput`，其输入不再依赖固定的 5 小时或每周字段，而是：

- 所有有效 `windows` 中最低的 `remainingPercent`；
- 最紧张窗口的稳定 `id`；
- 与上一快照比较后是否刚发生重置；
- 当前数据是否离线、错误或无效。

如果 `windows` 为 0，最低剩余百分比和窗口 ID 均为 `null`。多个窗口增加、减少或改变顺序时，状态映射必须按稳定 ID 比较，不能按数组下标比较。

## MascotState

计划支持以下稳定状态：

| 状态 | 含义 | 备注 |
|---|---|---|
| `happy` | 额度充足、近期刷新成功 | 与 `normal` 的阈值后续通过产品测试确定 |
| `normal` | 正常工作状态 | 默认静态/轻动画状态 |
| `tired` | 已用额度升高 | 不应暗示服务故障 |
| `anxious` | 接近阈值或 pace 明显偏高 | 阈值必须配置化并有测试 |
| `critical` | 额度接近耗尽或已耗尽 | 不能只靠颜色传达 |
| `sleepy` | 长时间无活动或等待重置 | 不应替代“数据过期”文本 |
| `celebrating` | 重置完成或额度恢复的一次性短状态 | 必须有防重复触发和最长时长 |
| `offline` | 离线、请求失败且没有可确认的新数据 | 与缓存标记同时呈现，不能伪装为实时数据 |

状态阈值不能散落在组件中。未来应由单个纯函数产生结果，并对边界值、缺失字段、缓存、401/403、离线和恢复场景建立表驱动测试。

## 动画资源格式

允许两种首选格式：

1. PNG 序列帧：实现简单、逐帧替换清晰，适合少量状态。
2. Sprite Sheet：减少文件数和解码切换，适合像素风或共享尺寸资源。

默认播放速率为 4～6 fps。每个资源包可以在允许范围内声明帧率；高于该范围需要单独性能验证。禁止把刷新频率、网络请求或 quota countdown 绑定到动画帧循环。

## 资源包 manifest

未来每套皮肤使用独立 manifest，示意结构如下：

```json
{
  "schemaVersion": 1,
  "id": "default-duck",
  "displayName": "Default Duck",
  "renderer": "png-sequence",
  "frameRate": 5,
  "states": {
    "normal": { "frames": ["normal-01.png"] },
    "offline": { "frames": ["offline-01.png"] }
  },
  "fallbackState": "normal"
}
```

正式 schema 需要约束：

- `schemaVersion`、资源包 ID 和显示名；
- renderer 类型、默认帧率、循环方式和单次状态最长时长；
- 每个 `MascotState` 的帧列表或 Sprite Sheet 坐标；
- 逻辑尺寸、像素密度、锚点和透明背景要求；
- fallback state；
- 可选作者、许可证和资源来源元数据。

manifest 只描述本地打包资源，不能包含远程脚本、认证信息或运行时下载地址。

## Reduced motion

- 尊重 Windows/浏览器 `prefers-reduced-motion`。
- reduced motion 开启时默认显示当前状态的首帧或专用静态帧。
- `celebrating` 等一次性动画不得绕过 reduced motion。
- 状态变化仍需通过文字、数值和可访问名称表达，不能只依赖运动。

## 安全降级

资源加载必须 fail-open 到额度 UI、fail-closed 到动画本身：

- manifest 缺失或 JSON 无效：禁用吉祥物并保留现有额度 UI。
- 某状态缺帧：使用 manifest 的 fallback state。
- fallback 也缺失：显示内置静态占位或完全隐藏吉祥物。
- 图片解码失败：停止该资源包，不能无限重试或占满日志。
- 帧率、尺寸或帧数超限：拒绝资源包并给出不含本地路径的诊断。
- 动画异常不得触发 provider 刷新、窗口重建或应用退出。

## 皮肤扩展

状态模型与 renderer 稳定后，可以增加像素风、扁平插画或其他本地皮肤。所有皮肤必须：

- 使用相同 `MascotState` 语义；
- 自带明确许可证和作者信息；
- 不改变认证、网络或 provider 行为；
- 不执行代码，不从远程地址加载资源；
- 通过资源缺失、reduced motion、DPI 缩放和内存占用验收。

## 后续阶段建议顺序

1. 固化 `MascotState` 纯类型和状态映射测试。
2. 定义 manifest JSON Schema 与资源限制。
3. 实现静态单帧 renderer 和安全降级。
4. 加入 4～6 fps 的最小帧循环与 reduced motion。
5. 在 100%、125%、150% 缩放及多显示器环境做性能和视觉验收。
6. 最后再设计默认小鸭资源和其他皮肤。
