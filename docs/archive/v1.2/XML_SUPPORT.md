# XML Language Support

## 概述

cr-semservice 现已支持 XML 语言的静态代码分析。

## 支持的文件扩展名

- `.xml` - 标准 XML 文件
- `.xsd` - XML Schema 定义文件
- `.xsl` - XSL 样式表文件
- `.xslt` - XSLT 转换文件
- `.svg` - SVG 图形文件
- `.pom` - Maven POM 文件

## 实现细节

### 修改的文件

1. **核心类型** (`crates/astgrep-core/src/types.rs`)
   - 在 `Language` 枚举中添加了 `Xml` 变体
   - 更新了 `extensions()` 方法以返回 XML 文件扩展名
   - 更新了 `as_str()` 和 `from_str()` 方法

2. **语言常量** (`crates/astgrep-core/src/constants.rs`)
   - 将 `Language::Xml` 添加到 `ALL_LANGUAGES` 数组

3. **语言解析器特征** (`crates/astgrep-core/src/traits.rs`)
   - 在 `extensions()` 方法中添加了 XML 扩展名支持

4. **XML 解析器** (`crates/astgrep-parser/src/xml.rs`)
   - 创建了 `XmlAdapter` 结构体
   - 创建了 `XmlParser` 结构体实现 `LanguageParser` trait
   - 添加了全面的测试用例

5. **解析器注册** (`crates/astgrep-parser/src/registry.rs`)
   - 在 `ParserFactory::create_parser()` 中添加了 XML 支持
   - 在 `get_default_config()` 中添加了 XML 配置

6. **适配器** (`crates/astgrep-parser/src/adapters.rs`)
   - 在 `is_keyword()` 函数中添加了 XML 处理

7. **基础适配器** (`crates/astgrep-parser/src/base_adapter.rs`)
   - 添加了 XML 适配器元数据
   - 实现了 `parse_xml_style()` 方法用于基本 XML 解析

8. **CLI 命令**
   - `crates/astgrep-cli/src/commands/analyze_enhanced.rs` - 添加了 XML 文件扩展名匹配和语言检测
   - `crates/astgrep-cli/src/commands/info.rs` - 添加了 XML 语言信息显示
   - `crates/astgrep-cli/src/commands/languages.rs` - 添加了 XML 语言描述

## 测试文件

在 `tests/xml/` 目录下创建了以下测试文件:

- `sample.xml` - 书籍目录示例
- `config.xml` - 配置文件示例
- `pom.xml` - Maven 项目文件示例
- `image.svg` - SVG 图形示例

## 使用示例

```rust
use astgrep_core::Language;
use astgrep_parser::ParserFactory;
use std::path::Path;

// 创建 XML 解析器
let parser = ParserFactory::create_parser(Language::Xml)?;

// 解析 XML 代码
let xml_code = r#"<?xml version="1.0"?>
<root>
    <item id="1">Hello</item>
</root>"#;

let ast = parser.parse(xml_code, Path::new("test.xml"))?;
println!("Parsed XML: {}", ast.node_type());
```

## 验证

运行以下命令验证 XML 支持:

```bash
# 编译项目
cargo build --lib

# 运行测试
cargo test --lib xml
```

## 状态

✅ XML 语言支持已完全集成到 cr-semservice 项目中
✅ 所有相关的 match 语句已更新以处理 `Language::Xml`
✅ 编译成功,无错误
✅ 基本功能测试通过

