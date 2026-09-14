# 代码上下文

## 当前链路

- `InputPanel.vue` 只监听输入框 `paste` 与区域 `drop`。
- `DownloadPage.vue` 的 `submit(value)` 已能写入 Pinia 输入并调用解析服务。
- `normalizeParseInput` 已是支持 URL/BV/AV 的权威前端校验规则。
- `main.ts` 负责运行时依赖注入；当前无剪贴板服务。
- Tauri 未注册 clipboard-manager 插件，能力文件也没有读文本权限。

## 根因

现有实现把“剪贴板自动解析”缩减成了输入框 paste 事件。复制动作不会向 BiliCatch WebView 派发 paste 事件，应用也没有主动读剪贴板的能力，因此无法实现期望流程。

## 影响面

仅涉及下载页生命周期、下载中心依赖注入、应用组合根、Tauri 插件注册与只读权限、相关测试和依赖锁文件。既有手动输入、粘贴、拖拽和解析 store 保持不变。
