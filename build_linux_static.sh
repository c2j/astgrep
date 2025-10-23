#!/bin/bash

# astgrep Linux 静态编译脚本
# 用于在 macOS 上交叉编译 Linux 静态二进制文件

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_header() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# 显示帮助信息
show_help() {
    cat << EOF
astgrep Linux 静态编译脚本

用法: $0 [选项] [目标]

目标:
  x86_64      编译 x86_64 Linux 静态二进制文件（默认）
  aarch64     编译 ARM64 Linux 静态二进制文件
  all         编译所有架构

选项:
  --use-cross     使用 cross 工具编译（推荐）
  --use-docker    使用 Docker 编译
  --no-gui        跳过 GUI 编译（推荐，GUI 依赖系统库无法静态编译）
  --strip         编译后使用 strip 减小文件大小
  --compress      使用 upx 压缩二进制文件
  --help, -h      显示此帮助信息

示例:
  $0                          # 使用默认方法编译 x86_64
  $0 --use-cross              # 使用 cross 工具编译 x86_64
  $0 --use-cross all          # 使用 cross 编译所有架构
  $0 --use-docker x86_64      # 使用 Docker 编译 x86_64
  $0 --no-gui --strip         # 跳过 GUI，编译后 strip

环境变量:
  CROSS_COMPILE_METHOD        编译方法: native, cross, docker
  TARGET_ARCH                 目标架构: x86_64, aarch64, all

EOF
}

# 检查依赖
check_dependencies() {
    print_header "检查依赖"
    
    # 检查 Rust
    if ! command -v cargo &> /dev/null; then
        print_error "未找到 cargo，请先安装 Rust"
        exit 1
    fi
    
    print_info "Rust 版本: $(rustc --version)"
    print_info "Cargo 版本: $(cargo --version)"
    
    # 检查编译方法
    if [ "$USE_CROSS" = true ]; then
        if ! command -v cross &> /dev/null; then
            print_warning "未找到 cross 工具，正在安装..."
            cargo install cross --git https://github.com/cross-rs/cross
        fi
        print_info "Cross 版本: $(cross --version)"
    elif [ "$USE_DOCKER" = true ]; then
        if ! command -v docker &> /dev/null; then
            print_error "未找到 Docker，请先安装 Docker"
            exit 1
        fi
        print_info "Docker 版本: $(docker --version)"
    else
        # 本地编译，检查 musl-gcc
        if ! command -v x86_64-linux-musl-gcc &> /dev/null && [ "$TARGET_ARCH" = "x86_64" ]; then
            print_warning "未找到 x86_64-linux-musl-gcc"
            print_info "尝试安装: brew install FiloSottile/musl-cross/musl-cross"
            print_info "或使用 --use-cross 选项"
            exit 1
        fi
    fi
    
    print_success "依赖检查通过"
}

# 安装 Rust 目标
install_target() {
    local target=$1
    
    print_info "检查 Rust 目标: $target"
    
    if ! rustup target list --installed | grep -q "$target"; then
        print_info "安装 Rust 目标: $target"
        rustup target add "$target"
    else
        print_success "目标 $target 已安装"
    fi
}

# 编译单个包
build_package() {
    local package=$1
    local target=$2
    local binary=$3
    
    print_info "编译 $package ($target)..."
    
    local build_cmd
    if [ "$USE_CROSS" = true ]; then
        build_cmd="cross build --release --target $target -p $package"
    else
        build_cmd="cargo build --release --target $target -p $package"
    fi
    
    # 执行编译
    if $build_cmd 2>&1 | tee /tmp/build_${package}_${target}.log | grep -q "^error\["; then
        print_error "$package 编译失败"
        tail -20 /tmp/build_${package}_${target}.log
        return 1
    fi
    
    # 检查二进制文件
    local binary_path="target/$target/release/$binary"
    if [ -f "$binary_path" ]; then
        local size=$(ls -lh "$binary_path" | awk '{print $5}')
        print_success "$package 编译成功 ($size)"
        
        # Strip（如果启用）
        if [ "$DO_STRIP" = true ]; then
            print_info "Strip $binary..."
            strip "$binary_path" 2>/dev/null || true
            local new_size=$(ls -lh "$binary_path" | awk '{print $5}')
            print_success "Strip 完成 ($new_size)"
        fi
        
        # 压缩（如果启用）
        if [ "$DO_COMPRESS" = true ]; then
            if command -v upx &> /dev/null; then
                print_info "压缩 $binary..."
                upx --best --lzma "$binary_path" 2>/dev/null || true
                local compressed_size=$(ls -lh "$binary_path" | awk '{print $5}')
                print_success "压缩完成 ($compressed_size)"
            else
                print_warning "未找到 upx，跳过压缩"
            fi
        fi
        
        return 0
    else
        print_error "$package 编译失败：未找到二进制文件"
        return 1
    fi
}

# 编译所有包
build_all_packages() {
    local target=$1
    local arch_name=$2
    
    print_header "编译 $arch_name 二进制文件"
    
    local success_count=0
    local total_count=3
    
    # 编译 astgrep
    if build_package "astgrep" "$target" "astgrep"; then
        ((success_count++))
    fi
    echo ""
    
    # 编译 astgrep-cli
    if build_package "astgrep-cli" "$target" "astgrep-cli"; then
        ((success_count++))
    fi
    echo ""
    
    # 编译 astgrep-web
    if build_package "astgrep-web" "$target" "astgrep-web"; then
        ((success_count++))
    fi
    echo ""
    
    # 编译 astgrep-gui（如果未禁用）
    if [ "$NO_GUI" != true ]; then
        total_count=4
        if build_package "astgrep-gui" "$target" "astgrep-gui"; then
            ((success_count++))
        fi
        echo ""
    fi
    
    print_info "编译结果: $success_count/$total_count"
    
    return $((total_count - success_count))
}

# 复制和打包
package_binaries() {
    local target=$1
    local arch_name=$2
    
    print_header "打包 $arch_name 二进制文件"
    
    # 创建发布目录
    local dist_dir="dist/linux-$arch_name"
    mkdir -p "$dist_dir"
    
    # 复制二进制文件
    local source_dir="target/$target/release"
    local copied=0
    
    for binary in astgrep astgrep-cli astgrep-web astgrep-gui; do
        if [ -f "$source_dir/$binary" ]; then
            cp "$source_dir/$binary" "$dist_dir/"
            print_success "复制 $binary"
            ((copied++))
        fi
    done
    
    if [ $copied -eq 0 ]; then
        print_error "没有找到任何二进制文件"
        return 1
    fi
    
    # 创建压缩包
    print_info "创建压缩包..."
    cd dist
    tar czf "astgrep-linux-$arch_name.tar.gz" "linux-$arch_name"
    cd ..
    
    local tarball_size=$(ls -lh "dist/astgrep-linux-$arch_name.tar.gz" | awk '{print $5}')
    print_success "压缩包创建完成: dist/astgrep-linux-$arch_name.tar.gz ($tarball_size)"
    
    # 显示文件列表
    echo ""
    print_info "二进制文件列表:"
    ls -lh "$dist_dir"
}

# 验证二进制文件
verify_binaries() {
    local target=$1
    local arch_name=$2
    
    print_header "验证 $arch_name 二进制文件"
    
    local source_dir="target/$target/release"
    
    for binary in astgrep astgrep-cli astgrep-web astgrep-gui; do
        if [ -f "$source_dir/$binary" ]; then
            print_info "检查 $binary:"
            file "$source_dir/$binary" | sed 's/^/  /'
            
            # 检查是否为静态链接（仅在 Linux 上有效）
            if command -v ldd &> /dev/null; then
                echo "  依赖库:"
                ldd "$source_dir/$binary" 2>&1 | sed 's/^/    /' || echo "    静态链接"
            fi
            echo ""
        fi
    done
}

# 主函数
main() {
    print_header "🚀 astgrep Linux 静态编译"
    
    # 解析参数
    USE_CROSS=false
    USE_DOCKER=false
    NO_GUI=false
    DO_STRIP=false
    DO_COMPRESS=false
    TARGET_ARCH="${TARGET_ARCH:-x86_64}"
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --use-cross)
                USE_CROSS=true
                shift
                ;;
            --use-docker)
                USE_DOCKER=true
                shift
                ;;
            --no-gui)
                NO_GUI=true
                shift
                ;;
            --strip)
                DO_STRIP=true
                shift
                ;;
            --compress)
                DO_COMPRESS=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            x86_64|aarch64|all)
                TARGET_ARCH=$1
                shift
                ;;
            *)
                print_error "未知选项: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    # 检查依赖
    check_dependencies
    
    # 编译
    local failed=0
    
    if [ "$TARGET_ARCH" = "all" ]; then
        # 编译所有架构
        for arch in x86_64 aarch64; do
            local target="${arch}-unknown-linux-musl"
            install_target "$target"
            
            if build_all_packages "$target" "$arch"; then
                package_binaries "$target" "$arch"
                verify_binaries "$target" "$arch"
            else
                ((failed++))
            fi
        done
    else
        # 编译单个架构
        local target="${TARGET_ARCH}-unknown-linux-musl"
        install_target "$target"
        
        if build_all_packages "$target" "$TARGET_ARCH"; then
            package_binaries "$target" "$TARGET_ARCH"
            verify_binaries "$target" "$TARGET_ARCH"
        else
            failed=1
        fi
    fi
    
    # 总结
    print_header "🎉 编译完成"
    
    if [ $failed -eq 0 ]; then
        print_success "所有目标编译成功！"
        echo ""
        print_info "发布文件位置:"
        ls -lh dist/*.tar.gz 2>/dev/null || true
    else
        print_warning "部分目标编译失败"
        exit 1
    fi
}

# 运行主函数
main "$@"

