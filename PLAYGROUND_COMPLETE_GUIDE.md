# Playground 完整使用指南

**最后更新**: 2025-10-18  
**版本**: 2.0 (功能增强版)  
**状态**: ✅ 完成

---

## 🚀 快速开始

### 1. 启动服务

```bash
cd /Volumes/Raiden_C2J/Projects/Desktop_Projects/CR/cr-semservice
cargo run -p cr-web --bin cr-web
```

### 2. 打开 Playground

```
http://127.0.0.1:8080/playground
```

### 3. 开始使用

- 左侧编写规则
- 右侧编写代码
- 点击 "Run" 执行分析

---

## 📋 界面布局

```
┌─────────────────────────────────────────────────────┐
│ Header: CR-SemService Playground                    │
├──────────────────────┬──────────────────────────────┤
│ simple | advanced    │ test code | metadata | docs  │
├──────────────────────┼──────────────────────────────┤
│                      │ Pro | Turbo                  │
│ YAML Rules Editor    │ Code Editor                  │
│ (45%)                │ (55%)                        │
│                      │                              │
│ Rule YAML Input      │ Language Select              │
│ Simple/Advanced      │ Code Input                   │
│ Tabs                 │ Run Button                   │
│                      │                              │
│ ▼ Inspect Rule       │ Matches Results              │
│ pattern: $VAR1 *...  │ Statistics                   │
│                      │                              │
└──────────────────────┴──────────────────────────────┘
```

---

## 📝 左侧面板 - 规则编辑

### Simple 标签页

用于编写简单的 YAML 规则。

#### 规则格式

```yaml
rules:
  - id: rule_id
    pattern: pattern_expression
    message: Error message
    languages:
      - javascript
    severity: INFO
```

#### 必需字段

- `rules:` - 规则列表
- `id:` - 规则唯一标识
- `pattern:` - 匹配模式
- `message:` - 错误消息
- `languages:` - 支持的语言
- `severity:` - 严重级别

#### 示例

```yaml
rules:
  - id: eval_usage
    pattern: eval($ARG)
    message: Avoid using eval()
    languages:
      - javascript
    severity: HIGH
```

### Advanced 标签页

用于编写高级规则配置。

#### 高级选项

```yaml
metadata:
  cwe: CWE-123
  owasp: A1
  confidence: HIGH
  
patterns:
  - pattern-either:
      - pattern: $VAR1 * $VAR2
      - pattern: Math.pow($VAR1, 2)
```

### Inspect Rule 部分

显示从规则中提取的 pattern。

#### 功能

- ✅ 实时显示 pattern
- ✅ 验证规则格式
- ✅ 显示错误信息

#### 示例

```
▼ Inspect Rule
pattern: $VAR1 * $VAR2;
```

---

## 💻 右侧面板 - 代码编辑和结果

### Test Code 标签页

#### 语言选择

支持的编程语言：
- JavaScript
- Python
- Java
- SQL
- Bash
- PHP
- C#
- C

#### 代码编辑器

- 输入要分析的代码
- 支持语法高亮
- 支持多行代码

#### Run 按钮

- 点击执行分析
- 快捷键: Ctrl+↵
- 验证规则后执行

#### 结果显示

分析结果以彩色编码的格式显示：

```
🔴 Line 9
Use Math.pow(<number>, 2);
Rule: multiplication_rule | Severity: INFO | Confidence: HIGH

✓ 1 match
Semgrep v1.41.0 · in 0.6s · ● tests passed ▼
```

### Metadata 标签页

显示完整的 JSON 响应数据。

#### 内容

- 分析结果的完整 JSON
- 包含所有字段和元数据
- 用于调试和检查

### Docs 标签页

显示 API 文档。

#### 内容

- API 端点说明
- 请求格式
- 响应格式

---

## 🎯 使用流程

### 流程 1: 简单规则测试

1. **编写规则** (左侧 simple 标签页)
   ```yaml
   rules:
     - id: test_rule
       pattern: eval($ARG)
       message: Avoid eval
       languages:
         - javascript
       severity: HIGH
   ```

2. **查看 Inspect Rule** (左侧底部)
   ```
   ▼ Inspect Rule
   pattern: eval($ARG)
   ```

3. **编写测试代码** (右侧 test code 标签页)
   ```javascript
   function unsafe(input) {
     return eval(input);
   }
   ```

4. **选择语言** (JavaScript)

5. **点击 Run** 执行分析

6. **查看结果** (右侧结果区域)
   ```
   🔴 Line 2
   Avoid eval
   Rule: test_rule | Severity: HIGH | Confidence: HIGH
   
   ✓ 1 match
   ```

### 流程 2: 高级规则测试

1. **编写高级规则** (左侧 advanced 标签页)
   ```yaml
   metadata:
     cwe: CWE-95
     owasp: A1
     confidence: HIGH
   
   patterns:
     - pattern-either:
         - pattern: eval($ARG)
         - pattern: Function($ARG)
   ```

2. **编写测试代码** (右侧)
   ```javascript
   var fn = new Function("return 1+1");
   ```

3. **执行分析** (点击 Run)

4. **查看结果** (右侧)

---

## 🔍 规则验证

### 自动验证

规则在以下情况下自动验证：

- ✅ 输入 YAML 时实时验证
- ✅ 点击 Run 按钮前验证
- ✅ 切换标签页时验证

### 验证项目

- ✅ 检查 `rules:` 部分
- ✅ 检查 `id:` 字段
- ✅ 检查 `pattern:` 字段
- ✅ 提取并显示 pattern

### 错误提示

如果规则无效，Inspect Rule 部分会显示错误：

```
▼ Inspect Rule
❌ Missing "rules:" section
```

---

## 📊 结果解释

### 严重级别

| 图标 | 级别 | 说明 |
|------|------|------|
| 🔴 | Critical | 严重安全问题 |
| 🟠 | High | 高风险问题 |
| 🟡 | Warning | 警告问题 |
| 🔵 | Info | 信息提示 |

### 结果项目

每个结果项目包含：

- **行号**: 代码中的行号
- **消息**: 规则的错误消息
- **规则 ID**: 规则的唯一标识
- **严重级别**: 问题的严重程度
- **置信度**: 匹配的置信度

### 统计信息

- **匹配数**: 找到的问题总数
- **版本**: Semgrep 版本
- **耗时**: 分析耗时 (毫秒)
- **状态**: 测试通过状态

---

## 🎓 示例

### 示例 1: JavaScript eval() 检测

**规则**:
```yaml
rules:
  - id: js_eval
    pattern: eval($ARG)
    message: Avoid using eval()
    languages:
      - javascript
    severity: HIGH
```

**代码**:
```javascript
function process(input) {
  return eval(input);
}
```

**结果**:
```
🔴 Line 2
Avoid using eval()
Rule: js_eval | Severity: HIGH | Confidence: HIGH

✓ 1 match
```

### 示例 2: Python pickle 检测

**规则**:
```yaml
rules:
  - id: py_pickle
    pattern: pickle.loads($ARG)
    message: Avoid pickle.loads()
    languages:
      - python
    severity: HIGH
```

**代码**:
```python
import pickle
data = pickle.loads(user_input)
```

**结果**:
```
🔴 Line 2
Avoid pickle.loads()
Rule: py_pickle | Severity: HIGH | Confidence: HIGH

✓ 1 match
```

### 示例 3: SQL 注入检测

**规则**:
```yaml
rules:
  - id: sql_injection
    pattern: SELECT * FROM $TABLE WHERE $COL = " + $VAR + "
    message: SQL injection vulnerability
    languages:
      - sql
    severity: CRITICAL
```

**代码**:
```sql
SELECT * FROM users WHERE id = " + userId + "
```

**结果**:
```
🔴 Line 1
SQL injection vulnerability
Rule: sql_injection | Severity: CRITICAL | Confidence: HIGH

✓ 1 match
```

---

## 🐛 常见问题

### Q: 规则验证失败

**A**: 检查以下内容：
- 是否包含 `rules:` 部分
- 是否包含 `id:` 字段
- 是否包含 `pattern:` 字段
- YAML 格式是否正确

### Q: 分析没有找到匹配

**A**: 可能的原因：
- 规则 pattern 不匹配代码
- 代码语言选择错误
- Pattern 表达式有误

### Q: 如何查看完整的响应数据

**A**: 点击 "metadata" 标签页查看完整的 JSON 响应。

### Q: 如何切换输出格式

**A**: 目前支持 JSON 格式，SARIF 格式在开发中。

---

## 📚 相关文档

1. **PLAYGROUND_REDESIGN_SUMMARY.md** - 界面重新设计
2. **PLAYGROUND_FEATURES_ENHANCEMENT.md** - 功能增强
3. **PLAYGROUND_REDESIGN_DETAILS.md** - 详细变更说明

---

## 🔗 快速链接

| 链接 | 地址 |
|------|------|
| Playground | http://127.0.0.1:8080/playground |
| API 文档 | http://127.0.0.1:8080/docs |
| 健康检查 | http://127.0.0.1:8080/health |

---

**最后更新**: 2025-10-18  
**版本**: 2.0  
**状态**: ✅ 完成

