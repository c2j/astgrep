# Java 规则验证分析

参考 XML 命名空间验证的方法，对 `tests/java` 下的案例进行确认。

---

## 📋 **规则 1: cs-eh-08-system-out-logging (r1)**

### 规则定义

```yaml
- id: cs-eh-08-system-out-logging
  name: "Avoid System.out for Logging"
  description: "Detects usage of System.out and System.err for logging"
  pattern-either:
    - pattern: System.out.println($MESSAGE)
    - pattern: System.out.print($MESSAGE)
    - pattern: System.err.println($MESSAGE)
    - pattern: System.err.print($MESSAGE)
```

### 规则意图

✅ 检测使用 `System.out` 或 `System.err` 进行日志输出的代码  
✅ 建议使用专业的日志框架（如 SLF4J、Log4j）

### 测试代码分析

#### 预期匹配（应该检测）

| 行号 | 代码 | 预期 | 实际 | 符合预期 |
|------|------|------|------|---------|
| 4 | `System.out.println("Debug: Processing user data");` | ✅ | ✅ | ✅ |
| 5 | `System.out.print("Status: ");` | ✅ | ✅ | ✅ |
| 6 | `System.err.println("Error occurred: " + getMessage());` | ✅ | ✅ | ✅ |
| 7 | `System.err.print("Warning: ");` | ✅ | ✅ | ✅ |
| 11 | `System.out.println("User ID: " + userId);` | ✅ | ✅ | ✅ |

#### 预期不匹配（不应该检测）

| 行号 | 代码 | 预期 | 实际 | 符合预期 |
|------|------|------|------|---------|
| 8 | `//System.err.print("Warning: ");` | ❌ | ❌ | ✅ |
| 16-20 | 使用 Logger 的代码 | ❌ | ❌ | ✅ |

### 结论

✅ **规则 1 符合预期** - 所有匹配都是正确的

---

## 📋 **规则 2: taint-maturity (r2)**

### 规则定义

```yaml
- id: taint-maturity
  languages: [java]
  severity: ERROR
  pattern-sources:
    - pattern: '"tainted"'
  pattern-sinks:
    - pattern: sink(...)
  pattern-sanitizers:
    - pattern: sanitize(...)
  mode: taint
```

### 规则意图

✅ 使用污点分析（taint mode）检测污染数据流  
✅ 追踪从 `"tainted"` 源到 `sink()` 的数据流  
✅ 识别被 `sanitize()` 清理的数据

### 测试代码分析

#### 预期匹配（应该检测）

| 行号 | 代码 | 预期 | 实际 | 符合预期 |
|------|------|------|------|---------|
| 6 | `sink("tainted");` | ✅ | ✅ | ✅ |
| 18 | `sink(z);` (z 来自 "tainted") | ✅ | ✅ | ✅ |

#### 预期不匹配（不应该检测）

| 行号 | 代码 | 预期 | 实际 | 符合预期 |
|------|------|------|------|---------|
| 20 | `sink(a);` (a 来自 "safe") | ❌ | ❌ | ✅ |
| 22 | `safe(z);` (不是 sink) | ❌ | ❌ | ✅ |
| 28 | `sink(sanitize("tainted"));` | ❌ | ❌ | ✅ |
| 37 | `sink(x);` (x 被 sanitize) | ❌ | ❌ | ✅ |

### 结论

✅ **规则 2 符合预期** - 污点分析工作正确

---

## 📋 **规则 3: java-thread (r3)**

### 规则定义

```yaml
- id: java-thread
  languages: [java]
  message: "自主线程"
  patterns:
    - pattern: '$thread.start()'
```

### 规则意图

✅ 检测调用 `.start()` 方法启动线程的代码  
✅ 识别所有创建和启动线程的位置

### 测试代码分析

#### 预期匹配（应该检测）

| 行号 | 代码 | 预期 | 实际 | 符合预期 |
|------|------|------|------|---------|
| 7 | `thread.start();` | ✅ | ✅ | ✅ |
| 11 | `myThread.start();` | ✅ | ✅ | ✅ |
| 14-16 | `new Thread(...).start();` | ✅ | ✅ | ✅ |

#### 预期不匹配（不应该检测）

| 行号 | 代码 | 预期 | 实际 | 符合预期 |
|------|------|------|------|---------|
| 22 | `thread.run();` (直接调用 run) | ❌ | ❌ | ✅ |

### 结论

✅ **规则 3 符合预期** - 所有匹配都是正确的

---

## 📊 **总体验证结果**

| 规则 | 总发现数 | 预期匹配 | 实际匹配 | 假阳性 | 假阴性 | 符合预期 |
|------|---------|---------|---------|--------|--------|---------|
| r1 (System.out) | 5 | 5 | 5 | 0 | 0 | ✅ |
| r2 (taint) | 2 | 2 | 2 | 0 | 0 | ✅ |
| r3 (thread) | 3 | 3 | 3 | 0 | 0 | ✅ |
| **总计** | **10** | **10** | **10** | **0** | **0** | **✅** |

---

## 🎯 **结论**

✅ **所有 Java 规则都符合预期**

- ✅ 没有假阳性（false positives）
- ✅ 没有假阴性（false negatives）
- ✅ 规则逻辑正确
- ✅ 模式匹配准确
- ✅ 污点分析工作正确

---

## 📝 **验证方法**

参考 XML 命名空间验证的方法：

1. **理解规则意图** - 明确规则想要检测什么
2. **分析测试代码** - 查看代码中哪些应该匹配，哪些不应该
3. **检查实际结果** - 运行规则并验证输出
4. **对比预期** - 确认实际结果与预期一致
5. **识别问题** - 如果有不符合预期的地方，分析原因

---

## ✅ **验证完成**

所有 Java 规则都已验证，符合预期。

