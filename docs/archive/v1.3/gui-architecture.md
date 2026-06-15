# astgrep-gui 架构概览（v1.3）

目标：GUI 仅作为交互外壳，最大化复用 astgrep 核心（astgrep-core/astgrep-rules/astgrep-parser）。

## 组件结构
- App（src/app.rs）：应用状态与协调者
- MenuBar：菜单行为，设置 UiState 请求标志
- RuleEditor：YAML 规则编辑/校验/格式化
- CodeEditor：源码编辑与结果高亮
- ResultsPanel：结果列表、复制与跳转
- StatusBar：状态与耗时
- SettingsPanel：全局设置（主题/字号/行号）
- utils：文件对话框与读写、剪贴板、语法高亮（轻量）

## 数据流与解耦
- MenuBar 仅修改 UiState.*request_* 标志（action request pattern）
- App.update() 周期性读取这些标志并调用对应“用例函数”（menu_open_rule/menu_export_results/...）
- 组件之间不直接互调，统一通过 App 状态传递与方法调用，减少耦合

## 与核心能力的集成
- 规则解析：astgrep-rules::RuleParser
- 规则执行：astgrep-rules::RuleEngine
- 语言解析器：astgrep-parser::LanguageParserRegistry（按语言构造合适的 parser）
- 数据结构：astgrep-core::{Finding, Location, Severity, Confidence, OutputFormat}
- 序列化：serde（JSON/YAML），SARIF 2.1.0 由 GUI 做轻量映射

## 关键用例串联
1) Run Analysis
   - parse rule（RuleParser）→ parse code（LanguageParserRegistry）→ execute（RuleEngine）→ Vec<Finding>
   - 更新结果面板与 CodeEditor 高亮；StatusBar 记录开始/完成时间
2) Validate Rule / Format Rule
   - 解析当前 YAML，错误显示到 RuleEditor；格式化通过 serde_yaml round-trip
3) 打开/保存/导出
   - 通过 utils::FileOperations 调用 rfd 原生对话框与文件读写
   - 导出按照扩展名选择 JSON/YAML/SARIF 格式
4) 结果操作
   - 复制：用 egui 内置剪贴板接口（ui.output_mut）
   - 跳转：ResultsPanel 设置 pending_jump；App 检测后触发 CodeEditor.highlight_range

## 视图同步
- AppSettings 持有 show_line_numbers/theme/font_size 等全局设置
- MenuBar 变更设置 → AppSettings 更新 → CodeEditor 在 show() 前同步 show_line_numbers 与 font_size
- 主题：App.apply_theme(ctx) 统一设置（Dark/Light）

## 错误与状态
- 所有 IO/解析类错误都反馈到 UI（状态栏/弹出/编辑器错误列表）
- StatusBar 提供 analysis_started/analysis_completed/analysis_failed 钩子

## 后台任务与可取消分析
- 分析在后台线程执行：std::thread + std::sync::mpsc
- 取消令牌：Arc<AtomicBool>，在解析前后与执行后检查；若置位则返回 Cancelled
- 结果回传：AnalysisMessage::{Finished, Error, Cancelled}
- 代际编号（generation）：防止旧任务结果覆盖新任务（仅当 gen 匹配时采纳）


## 扩展点
- 新菜单项：在 UiState 新增 request_xxx 标志，MenuBar 置位，App.process_menu_actions() 分发
- 新导出格式：在 menu_export_results 中按扩展名分支；或在 astgrep-core 增加 OutputFormat 并复用
- 结果操作：在 ResultsPanel 增加按钮并通过 App 方法实现副作用

## 构建与运行
- 构建：`cargo build -p astgrep-gui`
- 运行：`cargo run -p astgrep-gui`

## 设计原则
- 不复写“分析引擎”逻辑，全部复用核心库
- 即时模式 UI，状态集中在 App，副作用集中在 App 用例函数
- 小步实现、渐进增强；保证与 CLI 一致的语义与结果

