# Playground 快速开始指南

**版本**: 2.2 (Bug 修复版)  
**完成日期**: 2025-10-18  
**状态**: ✅ 完成

---

## 🚀 快速启动

### 1. 启动服务

```bash
cd /Volumes/Raiden_C2J/Projects/Desktop_Projects/CR/astgrep
cargo run -p cr-web --bin cr-web
```

### 2. 打开浏览器

```
http://127.0.0.1:8080/playground
```

---

## 📖 界面说明

### 左侧面板: YAML 规则编辑

```
┌─────────────────────────────────┐
│ simple | advanced               │  ← 标签页
├─────────────────────────────────┤
│                                 │
│  Rule YAML                      │
│  ┌─────────────────────────────┐│
│  │ rules:                      ││
│  │   - id: rule_id             ││
│  │     pattern: ...            ││
│  │     message: ...            ││
│  │     languages:              ││
│  │       - javascript          ││
│  │     severity: INFO          ││
│  └─────────────────────────────┘│
│                                 │
├─────────────────────────────────┤
│ ▼ Inspect Rule                  │
│ pattern: ...                    │
└─────────────────────────────────┘
```

**功能**:
- ✅ 编写 YAML 规则
- ✅ 实时验证规则
- ✅ 查看规则详情

### 右侧面板: 代码和结果

```
┌──────────────────────────────────────────┐
│ test code | metadata | docs  Pro  Turbo  │  ← 标签页
├──────────────────────────────────────────┤
│                                          │
│  Language: [JavaScript ▼]                │
│                                          │
│  Code                                    │
│  ┌──────────────────────────────────────┐│
│  │ function test() {                    ││
│  │   return eval(input);                ││
│  │ }                                    ││
│  └──────────────────────────────────────┘│
│                                          │
│  [Run Ctrl+↵] [▼]                        │
│                                          │
│  Matches                                 │
│  ┌──────────────────────────────────────┐│
│  │ 🔵 Line 2                            ││
│  │ Avoid using eval()                   ││
│  │ Rule: eval_usage | Severity: HIGH    ││
│  └──────────────────────────────────────┘│
│  ✓ 1 match                               │
└──────────────────────────────────────────┘
```

**功能**:
- ✅ 选择编程语言
- ✅ 编写测试代码
- ✅ 执行代码分析
- ✅ 查看分析结果

---

## 🎯 使用流程

### 步骤 1: 编写规则

在左侧 "simple" 标签页编写 YAML 规则:

```yaml
rules:
  - id: eval_usage
    pattern: eval($ARG)
    message: Avoid using eval()
    languages:
      - javascript
    severity: HIGH
```

### 步骤 2: 编写测试代码

在右侧 "test code" 标签页编写代码:

```javascript
function process(input) {
  return eval(input);
}
```

### 步骤 3: 执行分析

点击 "Run Ctrl+↵" 按钮

### 步骤 4: 查看结果

在 "Matches" 部分查看分析结果:

```
🔵 Line 2
Avoid using eval()
Rule: eval_usage | Severity: HIGH | Confidence: HIGH

✓ 1 match
```

---

## 🔄 标签页切换

### 左侧标签页

| 标签页 | 功能 |
|--------|------|
| simple | 简单规则编辑 |
| advanced | 高级规则配置 |

**特点**: 
- ✅ 切换时右侧内容保持不变
- ✅ 支持两种规则编辑模式

### 右侧标签页

| 标签页 | 功能 |
|--------|------|
| test code | 代码编辑和结果显示 |
| metadata | 完整的 JSON 响应 |
| docs | API 文档 |

**特点**:
- ✅ 切换时左侧内容保持不变
- ✅ 支持多种查看方式

---

## 💡 常见用法

### 用法 1: 检测 eval() 使用

**规则**:
```yaml
rules:
  - id: eval_usage
    pattern: eval($ARG)
    message: Avoid using eval()
    languages:
      - javascript
    severity: HIGH
```

**代码**:
```javascript
var result = eval(userInput);
```

**结果**: ✓ 1 match

---

### 用法 2: 检测 SQL 注入

**规则**:
```yaml
rules:
  - id: sql_injection
    pattern: query($ARG)
    message: Potential SQL injection
    languages:
      - python
    severity: CRITICAL
```

**代码**:
```python
query(f"SELECT * FROM users WHERE id = {user_id}")
```

**结果**: ✓ 1 match

---

### 用法 3: 检测硬编码密钥

**规则**:
```yaml
rules:
  - id: hardcoded_secret
    pattern: password = "..."
    message: Hardcoded password detected
    languages:
      - python
    severity: CRITICAL
```

**代码**:
```python
password = "admin123"
```

**结果**: ✓ 1 match

---

## 🧪 测试场景

### 场景 1: 修改代码后刷新结果

```
1. 编写规则
2. 编写代码
3. 点击 Run
4. ✅ 显示结果
5. 修改代码
6. 点击 Run
7. ✅ 结果更新
```

### 场景 2: 修改规则后刷新结果

```
1. 编写规则
2. 编写代码
3. 点击 Run
4. ✅ 显示结果
5. 修改规则
6. 点击 Run
7. ✅ 结果更新
```

### 场景 3: 标签页切换不丢失内容

```
1. 在右侧输入代码
2. 点击左侧 "advanced"
3. ✅ 右侧代码保持
4. 点击左侧 "simple"
5. ✅ 右侧代码仍然保持
```

---

## 🐛 已知问题 (已解决)

### ✅ Bug 1: Matches 结果不刷新
- **状态**: 已解决
- **修复**: 改进 switchTab 函数

### ✅ Bug 2: 标签页切换时内容被清空
- **状态**: 已解决
- **修复**: 改进 switchTab 函数

---

## 📚 相关文档

| 文档 | 说明 |
|------|------|
| PLAYGROUND_BUG_FIXES.md | 详细的 Bug 修复说明 |
| PLAYGROUND_TESTING_GUIDE.md | 完整的测试指南 |
| PLAYGROUND_ISSUES_RESOLVED.md | 问题解决总结 |

---

## 🔗 快速链接

| 链接 | 地址 |
|------|------|
| Playground | http://127.0.0.1:8080/playground |
| API 文档 | http://127.0.0.1:8080/docs |
| 健康检查 | http://127.0.0.1:8080/health |

---

## ✨ 功能特性

- ✅ 实时规则验证
- ✅ 多语言支持 (8 种语言)
- ✅ 彩色编码结果
- ✅ 完整的元数据显示
- ✅ 响应式设计
- ✅ 专业级 UI

---

## 🎓 总结

Playground 是一个强大的代码分析工具，支持：

1. **规则编辑** - 编写和验证 YAML 规则
2. **代码分析** - 分析多种编程语言的代码
3. **结果显示** - 清晰的分析结果展示
4. **元数据查看** - 完整的 JSON 响应

所有功能都已完善，可以放心使用！

---

**版本**: 2.2  
**完成日期**: 2025-10-18  
**状态**: ✅ 完成  
**用户体验**: ⭐⭐⭐⭐⭐

