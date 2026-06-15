# Linux 静态编译快速参考

## 🚀 三种编译方法

### 方法 1: 使用 cross 工具（最简单，推荐）

```bash
# 1. 安装 cross
cargo install cross --git https://github.com/cross-rs/cross

# 2. 编译
cross build --release --target x86_64-unknown-linux-musl -p astgrep
cross build --release --target x86_64-unknown-linux-musl -p astgrep-cli
cross build --release --target x86_64-unknown-linux-musl -p astgrep-web

# 3. 查看结果
ls -lh target/x86_64-unknown-linux-musl/release/astgrep*
```

### 方法 2: 使用编译脚本（功能最全）

```bash
# 使用 cross 工具编译
./build_linux_static.sh --use-cross

# 编译所有架构
./build_linux_static.sh --use-cross all

# 编译并优化
./build_linux_static.sh --use-cross --strip --no-gui

# 查看帮助
./build_linux_static.sh --help
```

### 方法 3: 使用 Docker（最可靠）

```bash
# 一键编译
./build_with_docker.sh

# 或手动使用 Docker
docker build -f Dockerfile.linux-static --target export -t astgrep-builder .
docker create --name temp astgrep-builder
docker cp temp:/export/astgrep ./astgrep-linux
docker rm temp
```

---

## 📦 编译结果位置

```bash
# cross 或原生编译
target/x86_64-unknown-linux-musl/release/
├── astgrep
├── astgrep-cli
└── astgrep-web

# 使用脚本编译
dist/linux-x86_64/
├── astgrep
├── astgrep-cli
└── astgrep-web

dist/astgrep-linux-x86_64.tar.gz  # 压缩包
```

---

## ✅ 验证静态链接

```bash
# 检查文件类型
file target/x86_64-unknown-linux-musl/release/astgrep
# 应显示: statically linked

# 在 Docker 中测试
docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app \
  alpine:latest /app/astgrep --version

# 在不同 Linux 发行版测试
docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app \
  ubuntu:latest /app/astgrep --version

docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app \
  centos:latest /app/astgrep --version
```

---

## 🎯 常用命令

### 安装 Rust 目标

```bash
# x86_64 Linux (musl)
rustup target add x86_64-unknown-linux-musl

# ARM64 Linux (musl)
rustup target add aarch64-unknown-linux-musl

# 查看已安装目标
rustup target list --installed
```

### 使用 cargo 别名

```bash
# 在 .cargo/config.toml 中已配置别名

# 编译 Linux x86_64
cargo build-linux -p astgrep

# 编译 Linux ARM64
cargo build-linux-arm -p astgrep
```

### 优化二进制大小

```bash
# 使用 strip
strip target/x86_64-unknown-linux-musl/release/astgrep

# 使用 upx 压缩
brew install upx
upx --best --lzma target/x86_64-unknown-linux-musl/release/astgrep
```

---

## 🐛 常见问题

### 问题 1: 找不到 musl-gcc

```bash
# 解决方案 1: 安装 musl-cross
brew install FiloSottile/musl-cross/musl-cross

# 解决方案 2: 使用 cross
cargo install cross
cross build --release --target x86_64-unknown-linux-musl

# 解决方案 3: 使用 Docker
./build_with_docker.sh
```

### 问题 2: OpenSSL 链接错误

```bash
# 在 Cargo.toml 中添加
[dependencies]
openssl = { version = "0.10", features = ["vendored"] }

# 或设置环境变量
export OPENSSL_STATIC=1
```

### 问题 3: GUI 编译失败

```bash
# GUI 不适合静态编译，跳过它
./build_linux_static.sh --use-cross --no-gui
```

---

## 📊 性能对比

| 方法 | 难度 | 速度 | 可靠性 | 推荐度 |
|------|------|------|--------|--------|
| cross | ⭐ 最简单 | ⭐⭐⭐⭐ 快 | ⭐⭐⭐⭐⭐ 最高 | ⭐⭐⭐⭐⭐ 强烈推荐 |
| 脚本 | ⭐⭐ 简单 | ⭐⭐⭐⭐ 快 | ⭐⭐⭐⭐ 高 | ⭐⭐⭐⭐ 推荐 |
| Docker | ⭐⭐⭐ 中等 | ⭐⭐⭐ 中等 | ⭐⭐⭐⭐⭐ 最高 | ⭐⭐⭐ 推荐 |

---

## 🎓 完整示例

### 示例 1: 快速编译单个工具

```bash
# 安装 cross
cargo install cross

# 编译 astgrep
cross build --release --target x86_64-unknown-linux-musl -p astgrep

# 测试
docker run --rm -v $(pwd)/target/x86_64-unknown-linux-musl/release:/app \
  alpine:latest /app/astgrep --version
```

### 示例 2: 编译所有工具并打包

```bash
# 使用脚本
./build_linux_static.sh --use-cross --strip

# 结果
ls -lh dist/astgrep-linux-x86_64.tar.gz

# 部署到服务器
scp dist/astgrep-linux-x86_64.tar.gz user@server:/tmp/
ssh user@server 'cd /tmp && tar xzf astgrep-linux-x86_64.tar.gz && ./astgrep --version'
```

### 示例 3: 编译多个架构

```bash
# 编译 x86_64 和 ARM64
./build_linux_static.sh --use-cross all

# 结果
ls -lh dist/
# astgrep-linux-x86_64.tar.gz
# astgrep-linux-aarch64.tar.gz
```

---

## 📚 相关文档

- [完整交叉编译指南](docs/CROSS_COMPILE_GUIDE.md)
- [编译指南](docs/BUILD_GUIDE.md)
- [快速参考](COMPILE_QUICK_REFERENCE.md)

---

## 💡 最佳实践

1. **开发阶段**: 使用本地编译
   ```bash
   cargo build --release
   ```

2. **发布阶段**: 使用 cross 编译静态二进制
   ```bash
   cross build --release --target x86_64-unknown-linux-musl
   ```

3. **CI/CD**: 使用 Docker 编译
   ```bash
   docker build -f Dockerfile.linux-static .
   ```

4. **优化大小**: 使用 strip 和 upx
   ```bash
   ./build_linux_static.sh --use-cross --strip --compress
   ```

---

**快速开始**: `./build_linux_static.sh --use-cross`

