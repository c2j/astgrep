# astgrep 交叉编译指南 - Linux 静态链接

本文档详细说明如何在 macOS 上交叉编译出 Linux 下可独立运行的静态链接二进制文件。

## 📋 目录

- [环境准备](#环境准备)
- [方法 1: 使用 musl 目标（推荐）](#方法-1-使用-musl-目标推荐)
- [方法 2: 使用 Docker 容器编译](#方法-2-使用-docker-容器编译)
- [方法 3: 使用 cross 工具](#方法-3-使用-cross-工具)
- [验证和测试](#验证和测试)
- [常见问题](#常见问题)

---

## 🔧 环境准备

### 1. 安装 Rust 交叉编译工具链

```bash
# 添加 Linux x86_64 musl 目标（静态链接）
rustup target add x86_64-unknown-linux-musl

# 添加 Linux x86_64 gnu 目标（动态链接）
rustup target add x86_64-unknown-linux-gnu

# 添加 Linux aarch64 musl 目标（ARM64 静态链接）
rustup target add aarch64-unknown-linux-musl

# 查看已安装的目标
rustup target list --installed
```

### 2. 安装交叉编译工具链

```bash
# 使用 Homebrew 安装 musl-cross
brew install FiloSottile/musl-cross/musl-cross

# 或者安装完整的交叉编译工具链
brew install messense/macos-cross-toolchains/x86_64-unknown-linux-musl
brew install messense/macos-cross-toolchains/aarch64-unknown-linux-musl
```

---

## 方法 1: 使用 musl 目标（推荐）

### 优点
- ✅ 完全静态链接，无依赖
- ✅ 二进制文件可在任何 Linux 发行版运行
- ✅ 不需要 Docker
- ✅ 编译速度快

### 缺点
- ⚠️ 某些 C 库可能不兼容
- ⚠️ GUI 应用可能需要额外配置

### 步骤 1: 配置 Cargo

创建或编辑 `.cargo/config.toml`：

```toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static"]

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"
rustflags = ["-C", "target-feature=+crt-static"]
```

### 步骤 2: 编译

```bash
# 编译 x86_64 Linux 静态二进制文件
cargo build --release --target x86_64-unknown-linux-musl -p astgrep
cargo build --release --target x86_64-unknown-linux-musl -p astgrep-cli
cargo build --release --target x86_64-unknown-linux-musl -p astgrep-web

# 编译 ARM64 Linux 静态二进制文件
cargo build --release --target aarch64-unknown-linux-musl -p astgrep
cargo build --release --target aarch64-unknown-linux-musl -p astgrep-cli
cargo build --release --target aarch64-unknown-linux-musl -p astgrep-web
```

### 步骤 3: 查看编译结果

```bash
# x86_64 二进制文件
ls -lh target/x86_64-unknown-linux-musl/release/astgrep*

# ARM64 二进制文件
ls -lh target/aarch64-unknown-linux-musl/release/astgrep*

# 验证是否为静态链接
file target/x86_64-unknown-linux-musl/release/astgrep
# 输出应包含: statically linked
```

---

## 方法 2: 使用 Docker 容器编译

### 优点
- ✅ 环境隔离，不污染本地系统
- ✅ 与 Linux 环境完全一致
- ✅ 支持所有依赖库

### 缺点
- ⚠️ 需要安装 Docker
- ⚠️ 编译速度较慢（首次）

### 步骤 1: 创建 Dockerfile

创建 `Dockerfile.linux-static`：

```dockerfile
# 使用 Alpine Linux 作为基础镜像（musl libc）
FROM rust:alpine AS builder

# 安装必要的构建工具
RUN apk add --no-cache \
    musl-dev \
    gcc \
    g++ \
    make \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

# 设置工作目录
WORKDIR /build

# 复制项目文件
COPY . .

# 编译静态二进制文件
RUN cargo build --release --target x86_64-unknown-linux-musl

# 创建最小运行镜像
FROM scratch
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/astgrep /astgrep
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/astgrep-cli /astgrep-cli
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/astgrep-web /astgrep-web
ENTRYPOINT ["/astgrep"]
```

### 步骤 2: 构建 Docker 镜像

```bash
# 构建镜像
docker build -f Dockerfile.linux-static -t astgrep-builder .

# 提取二进制文件
docker create --name astgrep-extract astgrep-builder
docker cp astgrep-extract:/astgrep ./target/astgrep-linux-x86_64
docker cp astgrep-extract:/astgrep-cli ./target/astgrep-cli-linux-x86_64
docker cp astgrep-extract:/astgrep-web ./target/astgrep-web-linux-x86_64
docker rm astgrep-extract
```

### 步骤 3: 使用 docker-compose（可选）

创建 `docker-compose.yml`：

```yaml
version: '3.8'

services:
  builder:
    build:
      context: .
      dockerfile: Dockerfile.linux-static
    volumes:
      - ./target:/output
    command: sh -c "
      cp /build/target/x86_64-unknown-linux-musl/release/astgrep /output/astgrep-linux-x86_64 &&
      cp /build/target/x86_64-unknown-linux-musl/release/astgrep-cli /output/astgrep-cli-linux-x86_64 &&
      cp /build/target/x86_64-unknown-linux-musl/release/astgrep-web /output/astgrep-web-linux-x86_64
      "
```

运行：

```bash
docker-compose up builder
```

---

## 方法 3: 使用 cross 工具

### 优点
- ✅ 自动管理 Docker 环境
- ✅ 配置简单
- ✅ 支持多种目标平台

### 步骤 1: 安装 cross

```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### 步骤 2: 配置 Cross.toml

创建 `Cross.toml`：

```toml
[build]
# 使用预构建的镜像
pre-build = []

[target.x86_64-unknown-linux-musl]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-musl:latest"

[target.aarch64-unknown-linux-musl]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-musl:latest"
```

### 步骤 3: 使用 cross 编译

```bash
# 编译 x86_64 Linux 静态二进制文件
cross build --release --target x86_64-unknown-linux-musl -p astgrep
cross build --release --target x86_64-unknown-linux-musl -p astgrep-cli
cross build --release --target x86_64-unknown-linux-musl -p astgrep-web

# 编译 ARM64 Linux 静态二进制文件
cross build --release --target aarch64-unknown-linux-musl -p astgrep
cross build --release --target aarch64-unknown-linux-musl -p astgrep-cli
cross build --release --target aarch64-unknown-linux-musl -p astgrep-web
```

---

## 🔍 验证和测试

### 1. 检查二进制文件类型

```bash
# 检查文件类型
file target/x86_64-unknown-linux-musl/release/astgrep

# 期望输出:
# astgrep: ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, ...
```

### 2. 检查依赖库

```bash
# 在 Linux 上检查动态库依赖（应该为空或只有 linux-vdso）
ldd target/x86_64-unknown-linux-musl/release/astgrep

# 期望输出:
# not a dynamic executable
# 或
# linux-vdso.so.1 (0x00007ffd...)
# statically linked
```

### 3. 在 Linux 上测试

```bash
# 复制到 Linux 机器
scp target/x86_64-unknown-linux-musl/release/astgrep user@linux-host:/tmp/

# 在 Linux 上运行
ssh user@linux-host '/tmp/astgrep --version'
```

### 4. 使用 Docker 测试

```bash
# 在 Alpine Linux 容器中测试
docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app alpine:latest /app/astgrep --version

# 在 Ubuntu 容器中测试
docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app ubuntu:latest /app/astgrep --version

# 在 CentOS 容器中测试
docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app centos:latest /app/astgrep --version
```

---

## 📦 一键编译脚本

创建 `build_linux_static.sh`：

```bash
#!/bin/bash

set -e

echo "🚀 开始交叉编译 Linux 静态二进制文件..."

# 检查目标是否已安装
if ! rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
    echo "📦 安装 x86_64-unknown-linux-musl 目标..."
    rustup target add x86_64-unknown-linux-musl
fi

# 编译所有二进制文件
echo "🔨 编译 astgrep..."
cargo build --release --target x86_64-unknown-linux-musl -p astgrep

echo "🔨 编译 astgrep-cli..."
cargo build --release --target x86_64-unknown-linux-musl -p astgrep-cli

echo "🔨 编译 astgrep-web..."
cargo build --release --target x86_64-unknown-linux-musl -p astgrep-web

# 创建发布目录
mkdir -p dist/linux-x86_64

# 复制二进制文件
echo "📦 复制二进制文件到 dist/linux-x86_64/..."
cp target/x86_64-unknown-linux-musl/release/astgrep dist/linux-x86_64/
cp target/x86_64-unknown-linux-musl/release/astgrep-cli dist/linux-x86_64/
cp target/x86_64-unknown-linux-musl/release/astgrep-web dist/linux-x86_64/

# 压缩文件
echo "📦 创建压缩包..."
cd dist/linux-x86_64
tar czf ../astgrep-linux-x86_64.tar.gz *
cd ../..

echo "✅ 编译完成！"
echo ""
echo "📍 二进制文件位置:"
ls -lh dist/linux-x86_64/
echo ""
echo "📦 压缩包:"
ls -lh dist/astgrep-linux-x86_64.tar.gz
```

使用方法：

```bash
chmod +x build_linux_static.sh
./build_linux_static.sh
```

---

## 🐛 常见问题

### 问题 1: 找不到 musl-gcc

**错误**：
```
error: linker `x86_64-linux-musl-gcc` not found
```

**解决方案**：

```bash
# 方法 1: 安装 musl-cross
brew install FiloSottile/musl-cross/musl-cross

# 方法 2: 使用 cross 工具
cargo install cross
cross build --release --target x86_64-unknown-linux-musl

# 方法 3: 使用 Docker 编译
```

### 问题 2: OpenSSL 链接错误

**错误**：
```
error: failed to run custom build command for `openssl-sys`
```

**解决方案**：

在 `Cargo.toml` 中添加：

```toml
[dependencies]
openssl = { version = "0.10", features = ["vendored"] }
```

或设置环境变量：

```bash
export OPENSSL_STATIC=1
export OPENSSL_DIR=/usr/local/opt/openssl@3
cargo build --release --target x86_64-unknown-linux-musl
```

### 问题 3: GUI 应用编译失败

**问题**: astgrep-gui 依赖图形库，musl 编译可能失败

**解决方案**：

GUI 应用不适合静态编译，建议：

```bash
# 只编译 CLI 工具
cargo build --release --target x86_64-unknown-linux-musl -p astgrep
cargo build --release --target x86_64-unknown-linux-musl -p astgrep-cli
cargo build --release --target x86_64-unknown-linux-musl -p astgrep-web

# GUI 使用 Docker 或在 Linux 上编译
```

### 问题 4: 二进制文件过大

**解决方案**：

```bash
# 1. 使用 strip 减小文件大小
strip target/x86_64-unknown-linux-musl/release/astgrep

# 2. 在 Cargo.toml 中优化
[profile.release]
opt-level = "z"     # 优化大小
lto = true          # 链接时优化
codegen-units = 1   # 更好的优化
strip = true        # 自动 strip
panic = "abort"     # 减小 panic 处理代码

# 3. 使用 upx 压缩（可选）
brew install upx
upx --best --lzma target/x86_64-unknown-linux-musl/release/astgrep
```

---

## 📊 性能对比

| 编译方法 | 编译时间 | 文件大小 | 兼容性 | 难度 |
|---------|---------|---------|--------|------|
| musl 本地编译 | ⭐⭐⭐⭐⭐ 快 | ⭐⭐⭐ 中等 | ⭐⭐⭐⭐⭐ 最好 | ⭐⭐⭐ 中等 |
| Docker 编译 | ⭐⭐⭐ 中等 | ⭐⭐⭐ 中等 | ⭐⭐⭐⭐⭐ 最好 | ⭐⭐ 简单 |
| cross 工具 | ⭐⭐⭐⭐ 较快 | ⭐⭐⭐ 中等 | ⭐⭐⭐⭐⭐ 最好 | ⭐ 最简单 |

---

## 🎯 推荐方案

### 对于 CLI 工具（astgrep, astgrep-cli, astgrep-web）

**推荐**: 使用 **cross 工具**

```bash
# 安装 cross
cargo install cross

# 编译
cross build --release --target x86_64-unknown-linux-musl -p astgrep
cross build --release --target x86_64-unknown-linux-musl -p astgrep-cli
cross build --release --target x86_64-unknown-linux-musl -p astgrep-web
```

### 对于 GUI 应用（astgrep-gui）

**推荐**: 在 **Linux 环境**中编译，或使用 Docker

```bash
# 使用 Docker
docker run --rm -v $(pwd):/workspace -w /workspace rust:latest \
  cargo build --release -p astgrep-gui
```

---

## 📚 相关资源

- [Rust 交叉编译文档](https://rust-lang.github.io/rustup/cross-compilation.html)
- [cross 工具文档](https://github.com/cross-rs/cross)
- [musl libc 官网](https://musl.libc.org/)
- [Rust 编译优化指南](https://doc.rust-lang.org/cargo/reference/profiles.html)

---

**最后更新**: 2025-10-23

