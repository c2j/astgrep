# CR-SemService Playground 快速参考

**最后更新**: 2025-10-18

---

## 🚀 快速开始 (30 秒)

### 1. 启动服务

```bash
cd /Volumes/Raiden_C2J/Projects/Desktop_Projects/CR/cr-semservice
cargo run -p cr-web --bin cr-web
```

### 2. 打开浏览器

```
http://127.0.0.1:8080/playground
```

### 3. 开始分析

- 选择语言
- 输入代码
- 点击 "▶ Run"

---

## 📋 界面布局

```
┌─────────────────────────────────────────────────────┐
│ Header: CR-SemService Playground                    │
├──────────────────────┬──────────────────────────────┤
│ Code | File          │ Results | Metadata | Docs    │
├──────────────────────┼──────────────────────────────┤
│                      │ JSON | SARIF                 │
│ Input Panel          │ Results Panel                │
│ (45%)                │ (55%)                        │
│                      │                              │
│ Language Select      │ Findings Display             │
│ Code Editor          │ Statistics Cards             │
│ Examples             │ Metadata JSON                │
│ Run Button           │ API Docs                     │
│                      │                              │
└──────────────────────┴──────────────────────────────┘
```

---

## 🎯 功能速查

### 代码分析

| 步骤 | 操作 |
|------|------|
| 1 | 选择语言 (JavaScript, Python, Java, SQL, Bash, PHP, C#, C) |
| 2 | 输入代码 |
| 3 | 点击 "▶ Run" |
| 4 | 查看结果 |

### 快速示例

| 按钮 | 功能 |
|------|------|
| JS: eval() | 加载 JavaScript eval 漏洞示例 |
| Python: pickle | 加载 Python pickle 反序列化示例 |
| SQL Injection | 加载 SQL 注入示例 |
| Java: SQL | 加载 Java SQL 注入示例 |

### 格式选择

| 按钮 | 功能 |
|------|------|
| JSON | 返回 JSON 格式结果 |
| SARIF | 返回 SARIF 2.1.0 格式结果 |

### 标签页

| 标签 | 内容 |
|------|------|
| Results | 分析结果和统计信息 |
| Metadata | 完整的 JSON 响应 |
| Docs | API 文档 |

---

## 📊 结果解释

### 严重级别图标

| 图标 | 级别 | 说明 |
|------|------|------|
| 🔴 | Critical | 严重安全问题 |
| 🟠 | High | 高风险问题 |
| 🟡 | Warning | 警告问题 |
| 🔵 | Info | 信息提示 |

### 统计信息

| 指标 | 说明 |
|------|------|
| Total Findings | 发现的问题总数 |
| Duration | 分析耗时 (毫秒) |
| Files | 分析的文件数 |
| Rules | 执行的规则数 |

---

## 🔧 API 端点

### 分析

```bash
# 代码分析
curl -X POST http://127.0.0.1:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{"code": "eval(x)", "language": "javascript"}'

# SARIF 格式
curl -X POST http://127.0.0.1:8080/api/v1/analyze/sarif \
  -H "Content-Type: application/json" \
  -d '{"code": "eval(x)", "language": "javascript"}'

# 文件分析
curl -X POST http://127.0.0.1:8080/api/v1/analyze/file \
  -F "file=@example.js" \
  -F "language=javascript"
```

### 规则

```bash
# 列出规则
curl http://127.0.0.1:8080/api/v1/rules

# 获取规则详情
curl http://127.0.0.1:8080/api/v1/rules/eval-usage

# 验证规则
curl -X POST http://127.0.0.1:8080/api/v1/rules/validate \
  -H "Content-Type: application/json" \
  -d '{"id": "test", "name": "Test", "pattern": "eval"}'
```

### 系统

```bash
# 健康检查
curl http://127.0.0.1:8080/health

# API 文档
curl http://127.0.0.1:8080/docs

# Playground
curl http://127.0.0.1:8080/playground
```

---

## 🎓 示例代码

### JavaScript - eval() 漏洞

```javascript
function unsafe(input) { 
  return eval(input); 
}
```

### Python - pickle 反序列化

```python
import pickle
data = pickle.loads(user_input)
```

### SQL - SQL 注入

```sql
SELECT * FROM users WHERE id = " + userId + "
```

### Java - SQL 注入

```java
String query = "SELECT * FROM users WHERE id = " + userId;
```

---

## 🐛 常见问题

### Q: 无法连接到服务

**A**: 
1. 确保服务已启动
2. 检查端口 8080 是否被占用
3. 尝试访问 http://127.0.0.1:8080/health

### Q: 分析失败

**A**:
1. 检查代码语法
2. 确保选择了正确的语言
3. 查看浏览器控制台的错误信息

### Q: 文件上传失败

**A**:
1. 检查文件大小 (最大 100MB)
2. 确保文件格式正确
3. 检查文件权限

---

## 📚 相关文档

- **PLAYGROUND_GUIDE.md** - 完整使用指南
- **PLAYGROUND_UI_OPTIMIZATION.md** - UI 优化详情
- **HOW_TO_RUN_CR_WEB.md** - 运行指南
- **FINAL_PLAYGROUND_SUMMARY.md** - 最终总结

---

## 🔗 快速链接

| 链接 | 地址 |
|------|------|
| Playground | http://127.0.0.1:8080/playground |
| API 文档 | http://127.0.0.1:8080/docs |
| 健康检查 | http://127.0.0.1:8080/health |
| 规则列表 | http://127.0.0.1:8080/api/v1/rules |

---

## ⌨️ 快捷操作

| 操作 | 快捷方式 |
|------|---------|
| 运行分析 | 点击 "▶ Run" 或 Ctrl+Enter |
| 切换格式 | 点击 "JSON" 或 "SARIF" |
| 查看元数据 | 点击 "Metadata" 标签 |
| 查看文档 | 点击 "Docs" 标签 |
| 加载示例 | 点击示例按钮 |

---

## 📞 支持

- **项目**: CR-SemService
- **仓库**: https://github.com/c2j/cr-semservice
- **问题**: 查看 GitHub Issues

---

**最后更新**: 2025-10-18  
**版本**: 1.0  
**状态**: ✅ 完成

