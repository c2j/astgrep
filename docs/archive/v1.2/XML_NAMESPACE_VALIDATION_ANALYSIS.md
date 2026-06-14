# XML 命名空间验证分析

## 问题描述

如何检测 XML 中未声明即使用的命名空间前缀？

### 示例

```xml
<root xmlns:app="http://example.com/app" xmlns:data="http://example.com/data">
  <app:configuration>
    <data:item>Value</data:item>
  </app:configuration>
  <app1:configuration>
    <!-- ❌ app1 前缀未声明，这是一个错误 -->
    <data:item>Value1</data:item>
  </app1:configuration>
</root>
```

## 技术挑战

### 1. Semgrep 的限制

Semgrep 的 `pattern-regex` 和 `pattern-not-regex` 无法：

- **上下文感知**：无法理解 XML 树结构
- **作用域跟踪**：无法跟踪命名空间声明的作用域
- **反向引用**：`pattern-not-regex` 不支持反向引用 `\1`
- **全局搜索**：`pattern-not-regex` 是全局搜索，不是行级或元素级

### 2. 正则表达式的局限

```regex
# 尝试检测所有未声明的前缀
pattern-regex: "<(\w+):(\w+)"
pattern-not-regex: "xmlns:\1\s*="
```

**问题**：
- `pattern-not-regex` 检查整个文件中是否存在 `xmlns:app=`
- 如果存在，就排除所有使用 `app` 前缀的地方
- 即使前缀在其他地方声明，也会被排除
- 导致对已声明前缀的误报

## 当前解决方案

### 方案 1：检测常见的标准前缀（推荐）

```yaml
- id: xml-namespace-prefix-001
  name: "Common Namespace Prefix Not Declared"
  patterns:
    - pattern-regex: "<(soap|xsi|xsd|wsdl):(\\w+)"
    - pattern-not-regex: "xmlns:\\1\\s*="
```

**优点**：
- ✅ 检测常见的标准命名空间前缀
- ✅ 避免对自定义前缀的误报
- ✅ 高准确性

**缺点**：
- ❌ 只能检测已知的前缀
- ❌ 无法检测自定义前缀

### 方案 2：改为检测其他 XML 最佳实践

```yaml
# 检测重复的命名空间声明
- id: xml-namespace-duplicate
  patterns:
    - pattern-regex: "xmlns:(\\w+)\\s*=.*xmlns:\\1\\s*="

# 检测未使用的命名空间声明
- id: xml-namespace-unused
  patterns:
    - pattern-regex: "xmlns:(\\w+)\\s*="
    - pattern-not-regex: "<\\w+:\\1"
```

## 为什么完整的验证很难

### 需要的信息

要正确检测未声明的命名空间前缀，需要：

1. **解析 XML 树结构** - 理解元素的层级关系
2. **跟踪命名空间作用域** - 记录每个元素的命名空间声明
3. **检查前缀可见性** - 验证前缀在其使用位置是否可见
4. **处理继承和覆盖** - 考虑命名空间的继承和重新声明

### 示例：复杂的命名空间场景

```xml
<root xmlns:app="http://example.com/app">
  <!-- app 前缀在这里可见 -->
  <app:element1>
    <!-- app 前缀在这里仍然可见（继承） -->
    <app:element2>
      <!-- 重新声明 app 前缀 -->
      <root xmlns:app="http://example.com/app2">
        <!-- app 前缀现在指向不同的 URI -->
        <app:element3/>
      </root>
    </app:element2>
  </app:element1>
  
  <!-- 在这里，app 前缀又指向原来的 URI -->
  <app:element4/>
</root>
```

## 推荐方案

### 对于 Semgrep 兼容性

使用当前的方案 1（检测常见的标准前缀）：
- ✅ 检测常见的标准命名空间前缀
- ✅ 避免误报
- ✅ 易于维护

### 对于完整的命名空间验证

建议使用专门的 XML 验证工具：
- **xmllint** - 命令行 XML 验证工具
- **XML Schema (XSD)** - 定义和验证 XML 结构
- **RelaxNG** - 另一种 XML 验证语言
- **自定义 XML 解析器** - 在 astgrep 中实现

## 结论

- ✅ 当前规则正确检测常见的标准前缀
- ✅ 避免了对自定义前缀的误报
- ❌ 无法检测所有未声明的前缀（这是 Semgrep 的限制）
- 💡 对于完整的验证，需要使用专门的 XML 验证工具

