# astgrep-gui 实现说明（v1.3）

目标：逐项补齐主界面按钮能力，同时尽量复用 astgrep 核心库，GUI 只做交互与可视化。

## 已实现要点一览
- MenuBar 全量打通：File（Open/Save/Export）、Analysis（Run/Validate/Format）、View（Line numbers/Theme/Font size）、Help（Documentation/Examples/Report）
- StatusBar 与分析流程联动：开始/完成/失败记录耗时
- ResultsPanel：复制到剪贴板、跳转至代码位置
- CodeEditor：与 AppSettings 同步行号与字体大小；高亮几何由统一字体大小驱动
- RuleEditor：Validate、Format（serde_yaml 往返）

## 关键模块与复用点
- 规则解析：astgrep-rules::RuleParser
- 分析执行：astgrep-rules::RuleEngine
- 代码解析：astgrep-parser::LanguageParserRegistry
- 数据类型：astgrep-core::{Finding, Location, OutputFormat, Severity, Confidence}
- 序列化：serde_json、serde_yaml；SARIF 2.1.0 由 GUI 轻量映射生成
- 文件对话框/读写：utils::FileOperations（rfd + std::fs）
- 剪贴板：egui 内置（ui.output_mut）

## 动作请求模式（Action Request Pattern）
- UiState 中定义 request_open_rule/request_run_analysis 等布尔标志
- MenuBar 仅设置标志，不直接调用业务逻辑
- App.update() 周期性调用 process_menu_actions()，将请求派发到具体实现（menu_open_rule 等）
- 好处：组件低耦合；便于后续扩展与测试

## 导出实现（JSON/YAML/SARIF）
- JSON/YAML：直接对 Vec<Finding> 序列化
- SARIF：考虑到核心库未直接输出 SARIF，GUI 将 Finding 映射为 runs[0].results；ruleId/message/locations（artifactLocation + region）等字段最小闭包满足 2.1.0 规范
- 扩展：如需更丰富的 SARIF 字段，可在 serialize_sarif() 中补充 rules 元数据、toolDriver、taxonomies 等

## 视图同步与主题
- AppSettings 中维护 show_line_numbers/theme/font_size
- CodeEditor::show() 每帧读取 settings 同步自有状态，字体大小影响：
  - 编辑文本 FontId 与行号绘制
  - 高亮层矩形尺寸与偏移
- 主题通过 App.apply_theme(ctx) 应用（Dark/Light）

## 跳转与高亮
- ResultsPanel 点击“Go to location”→ 设置 pending_jump: Option<Location>
- App.update() 检测并调用 jump_to_location()：
  - 计算选择范围 → CodeEditor.highlight_range(start_line,start_col,end_line,end_col)
  - 切换焦点到 CodeEditor 面板

## 错误处理策略
- 文件 IO/解析错误：以对话框或状态栏信息反馈
- 规则解析错误：在 RuleEditor 显示错误列表
- 分析失败：StatusBar.analysis_failed + 日志

## 扩展与二次开发建议
- 新菜单项：
  1) 在 UiState 增加 request_xxx
  2) MenuBar 设置该标志
  3) App.process_menu_actions() 中分支处理
- 新导出格式：
  - 在 FileOperations::get_export_file_filters 中增加过滤
  - 在 menu_export_results 根据扩展名分支输出
- 新结果动作：
  - 在 ResultsPanel 新增按钮，设置专用 pending_* 状态，由 App 执行副作用

## 构建与验证
- 构建：`cargo build -p astgrep-gui`
- 运行：`cargo run -p astgrep-gui`
- 核心回归用例：
  - 打开/保存 规则与代码文件
  - 运行分析，查看结果卡片数量、复制与跳转是否生效
  - 格式化/校验 规则
  - 切换行号/主题/字体大小；验证几何与文本同步
  - 导出结果到 JSON/YAML/SARIF，并使用外部查看器校验

## 已知限制
- Stop Analysis：采用合作式取消（取消令牌）。在核心执行函数返回之前无法“强中断”，但会丢弃取消中的任务结果（通过代际编号防陈旧覆盖）
- SARIF 输出为最小满足规范的映射，未包含完整 tool/rules/taxa 信息
- Edit 菜单为轻量实现：
  - Undo/Redo 基于整篇内容快照；无细粒度光标/选区级历史
  - Cut/Copy/Paste 作用于整个编辑内容；Paste 采用末尾追加
  - Find/Replace 的不区分大小写匹配采用 ASCII 语义，复杂 Unicode 情况未完全覆盖

