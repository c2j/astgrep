# astgrep-gui 用户指南（v1.3）

本指南帮助你快速上手 astgrep 的图形界面 astgrep-gui。GUI 仅作为“人机交互界面”，所有规则解析、语言解析和规则执行均复用 astgrep 核心能力（astgrep-core、astgrep-rules、astgrep-parser）。

## 快速开始
- 构建：`cargo build -p astgrep-gui`
- 运行：`cargo run -p astgrep-gui`
- 预置示例：首次启动会加载一条 Java 规则和示例代码，可直接点击 Analyze 观察结果

## 主界面结构
- 左侧：Rule Editor（规则编辑，Simple/Advanced 两个标签）
- 右上：Code Editor（测试代码编辑）
- 右下：Results（分析结果列表）
- 顶部：Menu Bar（文件/分析/视图/帮助）
- 底部：Status Bar（状态与耗时）

## 文件菜单（File）
- Open Rule…：打开规则文件（YAML/JSON）。读取后立即触发规则解析与校验
- Save Rule…：保存当前规则内容
- Open Code…：打开源代码文件（自动按扩展名推断语言：java/js/py/…）
- Save Code…：保存当前代码内容
- Export Results…：导出当前分析结果（JSON/YAML/SARIF）。导出格式按文件扩展名识别
- Exit：退出程序

实现说明：上述操作均通过 utils::FileOperations 调用原生文件对话框，并使用标准的读写 API；导出使用 astgrep-core 中的 Finding 结构序列化，SARIF 由 GUI 做轻量转换生成 2.1.0 版本 JSON。

## 分析菜单（Analysis）
- Run Analysis：运行分析。核心流程：
  1) 使用 astgrep-rules::RuleParser 解析左侧规则
  2) 使用 astgrep-parser::LanguageParserRegistry 解析右侧代码（按所选语言构造正确的文件扩展）
  3) 将规则加载到 astgrep-rules::RuleEngine 并执行
  4) 将结果映射为 astgrep-core::Finding 列表，并高亮到代码编辑器
- Stop Analysis：立即取消后台分析任务（使用取消令牌）；已运行完成的旧任务结果将被丢弃（代际编号防止陈旧结果覆盖）
- Validate Rule：重新解析当前 YAML，显示错误
- Format Rule：格式化 YAML（serde_yaml 往返格式化）


## 编辑菜单（Edit）
- Undo / Redo：基于内容快照的轻量撤销/重做（针对 Code Editor）
- Cut / Copy / Paste：复制到系统剪贴板；Cut 同时清空编辑器内容；Paste 以追加方式粘贴
- Find... / Replace...：打开查找替换窗口，支持大小写可选；
  - Find Next：在代码中高亮下一个匹配位置
  - Replace All：替换全部匹配并记录撤销快照

说明：当前实现作用于 Code Editor 文本区域；后续如需扩展到 Rule Editor 可继续复用同样机制。

## 视图菜单（View）
- Show line numbers：切换是否显示行号
- Theme：Dark/Light 主题（即时应用）
- Font Size：12/14/16/18 像素（代码编辑器与高亮几何同步更新）

实现说明：这些设置写入 AppSettings，全局生效；Code Editor 会在每次绘制前同步 line numbers 与 font size。

## 帮助菜单（Help）
- Documentation：打开项目文档（GitHub）
- Examples：加载内置示例（当前为加载默认示例规则与代码）
- Report Issue：打开 GitHub Issues

## 代码编辑器（Code Editor）
- 顶部工具栏：Language 下拉、Open/Save/Clear、Line numbers 开关、Analyze 按钮
- 编辑区：
  - 显示/隐藏行号（按 View 或工具栏开关）
  - 高亮分析结果（背景矩形与提示信息）
- Analyze：与菜单行为一致，触发完整分析链路

注意：Language 下拉控制代码解析语言；独立于 MenuBar 右上角“全局语言展示”。

## 规则编辑器（Rule Editor）
- Simple：简化视图（YAML 文本 + 错误列表 + Validate/Load Example）
- Advanced：完整 YAML 编辑 + Validate/Format/Load Example
- 任意编辑都会触发重新解析并展示错误（Validate 会再次触发一次解析）

## 结果面板（Results）
- 匹配项以卡片列表展示，按严重程度着色
- 每项提供：
  - 📋 复制到剪贴板：复制包含 Rule、Message、Location、Severity、File 的文本
  - 🔍 跳转定位：切换到 Code Editor 并高亮对应范围
- 底部统计显示匹配数量

## 状态栏（Status Bar）
- 在 Run Analysis 前后显示状态与耗时：analysis_started/analysis_completed/analysis_failed

## 导出结果
- 支持 JSON/YAML/SARIF（2.1.0）。建议：
  - 机器可读集成：JSON/YAML
  - 安全工具链集成：SARIF

## 常见问题
- 分析无结果：检查规则语言与代码语言是否匹配；确认规则能被成功解析
- 高亮不对齐：确认 View → Font Size 与编辑器字体一致；GUI 已使用相同字号进行几何计算
- Stop Analysis：可取消后台任务；若发现仍未即时停止，说明当前引擎执行处于不可中断段，结果会被丢弃，不会覆盖当前界面

## 兼容性与定位
- GUI 不重复实现引擎逻辑，全部复用 astgrep 核心；GUI 仅做交互与可视化
- 目标是与 CLI 同源同能，易用可视化为主

