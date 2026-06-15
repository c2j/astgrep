# astgrep Playground 使用指南

**功能**: 交互式 API 测试和代码分析工具  
**访问地址**: http://127.0.0.1:8080/playground  
**状态**: ✅ 已启动

---

## 🚀 快速开始

### 1. 启动服务

```bash
cd /Volumes/Raiden_C2J/Projects/Desktop_Projects/CR/astgrep
cargo run -p cr-web --bin cr-web
```

**预期输出**:
```
2025-10-18T13:00:00.000Z INFO  cr_web: Starting CR Web Service
2025-10-18T13:00:00.000Z INFO  cr_web: Server listening on 127.0.0.1:8080
```

### 2. 打开 Playground

在浏览器中访问:
```
http://127.0.0.1:8080/playground
```

### 3. 开始分析

选择语言，输入代码，点击 "🚀 Analyze" 按钮

---

## 📝 Playground 功能

### 左侧面板 - 输入

#### 代码分析标签
- **语言选择**: JavaScript, Python, Java, SQL, Bash, PHP, C#, C
- **代码输入**: 输入要分析的代码
- **快速示例**: 
  - JS: eval() - JavaScript eval 漏洞
  - Python: pickle - Python pickle 反序列化
  - SQL Injection - SQL 注入漏洞
  - Java: SQL - Java SQL 注入

#### 文件分析标签
- **文件上传**: 选择本地文件
- **语言选择**: 指定文件语言
- **上传分析**: 上传并分析文件

#### 输出格式
- **JSON**: 标准 JSON 格式
- **SARIF**: SARIF 2.1.0 格式 (CI/CD 集成)

### 右侧面板 - 结果

#### 分析结果
- **Findings**: 发现的安全问题
- **Statistics**: 分析统计信息
- **Error Messages**: 错误信息

---

## 🎯 使用示例

### 示例 1: 分析 JavaScript eval() 漏洞

1. **选择语言**: JavaScript
2. **输入代码**:
   ```javascript
   function unsafe(input) { 
     return eval(input); 
   }
   ```
3. **点击**: 🚀 Analyze
4. **查看结果**: 
   - 发现 eval() 使用
   - 严重级别: High
   - 置信度: High

### 示例 2: 分析 Python pickle 漏洞

1. **选择语言**: Python
2. **输入代码**:
   ```python
   import pickle
   data = pickle.loads(user_input)
   ```
3. **点击**: 🚀 Analyze
4. **查看结果**:
   - 发现 pickle.loads() 使用
   - 严重级别: High
   - 置信度: High

### 示例 3: 分析 SQL 注入

1. **选择语言**: SQL
2. **输入代码**:
   ```sql
   SELECT * FROM users WHERE id = " + userId + "
   ```
3. **点击**: 🚀 Analyze
4. **查看结果**:
   - 发现 SQL 注入风险
   - 严重级别: Critical
   - 置信度: High

### 示例 4: 获取 SARIF 格式结果

1. **输入代码**: `eval(x)`
2. **选择语言**: JavaScript
3. **点击**: 📋 SARIF
4. **查看结果**: SARIF 2.1.0 格式输出

---

## 📊 结果解释

### 分析结果结构

```json
{
  "findings": [
    {
      "id": "JS-001",
      "rule_id": "eval-usage",
      "message": "Use of eval() is dangerous",
      "severity": "high",
      "confidence": "high",
      "location": {
        "file": "input",
        "line": 1,
        "column": 40
      }
    }
  ],
  "summary": {
    "total_findings": 1,
    "findings_by_severity": {
      "high": 1
    },
    "files_analyzed": 1,
    "rules_executed": 5,
    "duration_ms": 45
  }
}
```

### 字段说明

| 字段 | 说明 |
|------|------|
| `id` | 发现的唯一 ID |
| `rule_id` | 触发的规则 ID |
| `message` | 问题描述 |
| `severity` | 严重级别 (info, warning, error, critical) |
| `confidence` | 置信度 (low, medium, high) |
| `location` | 代码位置 (文件、行、列) |
| `total_findings` | 发现总数 |
| `duration_ms` | 分析耗时 (毫秒) |

---

## 🔧 高级功能

### 1. 上传文件分析

1. 切换到 "File" 标签
2. 选择本地文件
3. 选择文件语言
4. 点击 "📤 Upload & Analyze"

### 2. SARIF 格式输出

用于 CI/CD 集成:
```bash
curl -X POST http://127.0.0.1:8080/api/v1/analyze/sarif \
  -H "Content-Type: application/json" \
  -d '{
    "code": "eval(x)",
    "language": "javascript"
  }'
```

### 3. 快速示例

点击快速示例按钮快速加载预定义的代码:
- **JS: eval()** - JavaScript eval 漏洞
- **Python: pickle** - Python pickle 反序列化
- **SQL Injection** - SQL 注入漏洞
- **Java: SQL** - Java SQL 注入

---

## 🌐 其他 API 端点

### 健康检查

```bash
curl http://127.0.0.1:8080/health
```

### API 文档

```
http://127.0.0.1:8080/docs
```

### 列出规则

```bash
curl http://127.0.0.1:8080/api/v1/rules
```

### 获取特定规则

```bash
curl http://127.0.0.1:8080/api/v1/rules/eval-usage
```

### 验证规则

```bash
curl -X POST http://127.0.0.1:8080/api/v1/rules/validate \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-rule",
    "name": "Test Rule",
    "pattern": "eval",
    "severity": "high"
  }'
```

---

## 🎨 Playground 特性

### 用户界面

- **现代化设计**: 渐变背景，响应式布局
- **标签页**: 代码分析和文件分析
- **实时反馈**: 加载状态，错误提示
- **快速示例**: 一键加载常见漏洞示例
- **格式选择**: JSON 和 SARIF 格式

### 交互功能

- **代码编辑**: 支持多行代码输入
- **文件上传**: 支持本地文件上传
- **语言选择**: 支持 8 种编程语言
- **结果展示**: 格式化 JSON 输出
- **统计信息**: 分析统计和性能指标

---

## 🐛 故障排除

### 问题 1: 无法连接到服务

**错误**: `Failed to fetch`

**解决**:
1. 确保服务已启动
2. 检查端口 8080 是否被占用
3. 尝试访问 http://127.0.0.1:8080/health

### 问题 2: 分析失败

**错误**: `Error: ...`

**解决**:
1. 检查代码语法是否正确
2. 确保选择了正确的语言
3. 查看浏览器控制台的错误信息

### 问题 3: 文件上传失败

**错误**: `Upload failed`

**解决**:
1. 检查文件大小 (最大 100MB)
2. 确保文件格式正确
3. 检查文件权限

---

## 📈 性能优化

### 快速分析

- 使用快速示例快速测试
- 分析小代码片段
- 使用 JSON 格式 (比 SARIF 更快)

### 批量分析

- 使用 API 端点进行批量分析
- 使用文件上传分析整个项目
- 使用存档分析 (ZIP/TAR)

---

## 🔐 安全建议

1. **本地使用**: Playground 仅在本地使用，不要暴露到互联网
2. **代码隐私**: 不要分析包含敏感信息的代码
3. **文件大小**: 限制上传文件大小以提高性能
4. **API 密钥**: 如果部署到生产环境，添加认证

---

## 📚 相关文档

- **HOW_TO_RUN_CR_WEB.md** - 如何运行 CR-Web
- **README.md** - 项目概述
- **API 文档** - http://127.0.0.1:8080/docs

---

## ✨ 总结

**Playground 是一个强大的交互式工具，用于**:
- ✅ 快速测试代码分析功能
- ✅ 验证规则和模式
- ✅ 学习 API 使用方法
- ✅ 调试分析结果
- ✅ 演示安全漏洞

**立即开始**:
1. 启动服务: `cargo run -p cr-web --bin cr-web`
2. 打开浏览器: http://127.0.0.1:8080/playground
3. 输入代码并分析

---

**准备好了吗？** 打开 Playground 开始分析代码！

