# astgrep 编译指南 / Build Guide

本文档详细说明如何编译 astgrep 项目的各个二进制文件。

## 📦 项目结构

astgrep 是一个 Rust workspace 项目，包含以下二进制程序：

| 二进制文件 | 包名 | 用途 | 大小 |
|-----------|------|------|------|
| `astgrep` | astgrep | 主程序 - 命令行工具 | ~6.3 MB |
| `astgrep-cli` | astgrep-cli | 独立的 CLI 工具 | ~6.3 MB |
| `astgrep-web` | astgrep-web | Web 服务和 REST API | ~6.9 MB |
| `astgrep-gui` | astgrep-gui | 图形界面 Playground | ~8.9 MB |

## 🚀 快速开始

### 方法 1: 编译所有二进制文件

```bash
# 编译所有二进制文件（Debug 模式）
cargo build --bins

# 编译所有二进制文件（Release 模式，推荐）
cargo build --release --bins
```

编译后的文件位置：
- Debug 模式: `target/debug/`
- Release 模式: `target/release/`

### 方法 2: 编译单个二进制文件

#### 编译 astgrep（主程序）

```bash
# Debug 模式
cargo build --bin astgrep

# Release 模式
cargo build --release --bin astgrep

# 运行
./target/release/astgrep --help
```

#### 编译 astgrep-cli

```bash
# Debug 模式
cargo build -p astgrep-cli

# Release 模式
cargo build --release -p astgrep-cli

# 运行
./target/release/astgrep-cli --help
```

#### 编译 astgrep-web

```bash
# Debug 模式
cargo build -p astgrep-web

# Release 模式
cargo build --release -p astgrep-web

# 运行（需要先创建 rules 目录）
mkdir -p rules
./target/release/astgrep-web
```

#### 编译 astgrep-gui

```bash
# Debug 模式
cargo build -p astgrep-gui

# Release 模式
cargo build --release -p astgrep-gui

# 运行
./target/release/astgrep-gui
```

## 📝 编译命令详解

### 使用 `--bin` 参数

`--bin` 参数用于编译特定的二进制目标：

```bash
cargo build --bin <binary-name>
```

**示例**：
```bash
cargo build --release --bin astgrep
cargo build --release --bin astgrep-cli
cargo build --release --bin astgrep-web
cargo build --release --bin astgrep-gui
```

### 使用 `-p` 参数

`-p` (或 `--package`) 参数用于编译特定的包（package）：

```bash
cargo build -p <package-name>
```

**示例**：
```bash
cargo build --release -p astgrep
cargo build --release -p astgrep-cli
cargo build --release -p astgrep-web
cargo build --release -p astgrep-gui
```

### 区别说明

| 参数 | 用途 | 适用场景 |
|------|------|----------|
| `--bin` | 编译特定的二进制目标 | 当一个包有多个二进制文件时 |
| `-p` | 编译整个包（包括库和二进制） | 编译独立的 crate |
| `--bins` | 编译所有二进制文件 | 一次性编译所有可执行文件 |

## 🔧 常用编译选项

### Release 模式（推荐生产使用）

```bash
cargo build --release -p astgrep-cli
```

**特点**：
- ✅ 优化编译，性能更好
- ✅ 文件体积更小
- ⏱️ 编译时间较长

### Debug 模式（开发调试）

```bash
cargo build -p astgrep-cli
```

**特点**：
- ✅ 编译速度快
- ✅ 包含调试信息
- ❌ 性能较差，文件较大

### 并行编译

```bash
# 使用 4 个线程编译
cargo build --release -j 4

# 使用所有可用 CPU 核心
cargo build --release -j $(nproc)  # Linux/macOS
```

### 详细输出

```bash
# 显示详细编译信息
cargo build --release -p astgrep-cli -v

# 显示非常详细的信息
cargo build --release -p astgrep-cli -vv
```

## 📋 完整编译流程

### 一键编译所有工具（推荐）

创建一个编译脚本 `build_all.sh`：

```bash
#!/bin/bash

echo "🚀 开始编译 astgrep 所有二进制文件..."
echo ""

# 清理之前的构建
echo "🧹 清理旧的构建文件..."
cargo clean

# 编译所有二进制文件
echo "📦 编译所有二进制文件（Release 模式）..."
cargo build --release --bins

# 检查编译结果
echo ""
echo "✅ 编译完成！二进制文件列表："
echo ""
ls -lh target/release/astgrep* | grep -v "\.d$"

echo ""
echo "🎉 所有工具编译完成！"
```

使用方法：

```bash
chmod +x build_all.sh
./build_all.sh
```

### 分步编译

```bash
# 1. 编译主程序
echo "编译 astgrep..."
cargo build --release --bin astgrep

# 2. 编译 CLI 工具
echo "编译 astgrep-cli..."
cargo build --release -p astgrep-cli

# 3. 编译 Web 服务
echo "编译 astgrep-web..."
cargo build --release -p astgrep-web

# 4. 编译 GUI 应用
echo "编译 astgrep-gui..."
cargo build --release -p astgrep-gui

# 5. 查看结果
ls -lh target/release/astgrep*
```

## 🧪 验证编译结果

### 检查二进制文件

```bash
# 列出所有编译的二进制文件
ls -lh target/release/astgrep* | grep -v "\.d$"

# 检查文件类型
file target/release/astgrep
file target/release/astgrep-cli
file target/release/astgrep-web
file target/release/astgrep-gui
```

### 测试运行

```bash
# 测试主程序
./target/release/astgrep --version
./target/release/astgrep --help

# 测试 CLI 工具
./target/release/astgrep-cli --version
./target/release/astgrep-cli --help

# 测试 Web 服务（需要 Ctrl+C 停止）
./target/release/astgrep-web &
curl http://localhost:3000/health
kill %1

# 测试 GUI 应用
./target/release/astgrep-gui
```

## 🐛 常见问题

### 问题 1: 找不到二进制文件

**错误**：
```
bash: ./target/release/astgrep-cli: No such file or directory
```

**解决方案**：
```bash
# 确认是否编译成功
cargo build --release -p astgrep-cli

# 检查文件是否存在
ls -la target/release/ | grep astgrep
```

### 问题 2: 编译错误

**错误**：
```
error: couldn't read `examples/validate_rule.rs`: No such file or directory
```

**解决方案**：
这个错误已经在主 `Cargo.toml` 中修复。如果仍然出现，请确保使用最新的代码。

### 问题 3: 依赖问题

**错误**：
```
error: failed to load manifest for dependency `astgrep-core`
```

**解决方案**：
```bash
# 更新依赖
cargo update

# 清理并重新编译
cargo clean
cargo build --release --bins
```

### 问题 4: GUI 中文显示为方框

**解决方案**：
已在 `crates/astgrep-gui/src/main.rs` 中添加了中文字体支持。重新编译即可：

```bash
cargo build --release -p astgrep-gui
```

## 📊 编译时间参考

在 MacBook Pro (M1) 上的编译时间：

| 模式 | 首次编译 | 增量编译 |
|------|----------|----------|
| Debug | ~3 分钟 | ~30 秒 |
| Release | ~5 分钟 | ~1 分钟 |

## 🎯 最佳实践

1. **开发时使用 Debug 模式**：
   ```bash
   cargo build -p astgrep-cli
   ```

2. **发布时使用 Release 模式**：
   ```bash
   cargo build --release -p astgrep-cli
   ```

3. **使用 cargo check 快速检查**：
   ```bash
   cargo check --all-targets
   ```

4. **使用 cargo clippy 检查代码质量**：
   ```bash
   cargo clippy --all-targets
   ```

5. **运行测试**：
   ```bash
   cargo test --all
   ```

## 📦 安装到系统

### 方法 1: 使用 cargo install

```bash
# 从本地源码安装
cargo install --path .
cargo install --path crates/astgrep-cli
cargo install --path crates/astgrep-web
cargo install --path crates/astgrep-gui

# 安装后可以直接运行
astgrep --help
astgrep-cli --help
astgrep-web
astgrep-gui
```

### 方法 2: 手动复制

```bash
# 复制到系统路径
sudo cp target/release/astgrep /usr/local/bin/
sudo cp target/release/astgrep-cli /usr/local/bin/
sudo cp target/release/astgrep-web /usr/local/bin/
sudo cp target/release/astgrep-gui /usr/local/bin/

# 验证
which astgrep
astgrep --version
```

## 🔗 相关资源

- [Rust 官方文档](https://doc.rust-lang.org/cargo/)
- [Cargo Book](https://doc.rust-lang.org/cargo/index.html)
- [astgrep 项目主页](https://github.com/c2j/astgrep)
- [astgrep 规则编写指南](./astgrep-Guide.md)

## 📞 获取帮助

如果遇到编译问题，请：

1. 查看本文档的"常见问题"部分
2. 在 GitHub 上提交 Issue
3. 查看项目的 CI/CD 配置文件

---

**最后更新**: 2025-10-22

