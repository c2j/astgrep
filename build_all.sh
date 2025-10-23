#!/bin/bash

# astgrep 一键编译脚本
# 用于编译所有二进制文件

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

# 打印标题
print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# 检查 Rust 环境
check_rust() {
    print_header "检查 Rust 环境"
    
    if ! command -v cargo &> /dev/null; then
        print_error "未找到 cargo，请先安装 Rust"
        print_info "访问 https://rustup.rs/ 安装 Rust"
        exit 1
    fi
    
    print_info "Rust 版本: $(rustc --version)"
    print_info "Cargo 版本: $(cargo --version)"
    print_success "Rust 环境检查通过"
}

# 清理旧的构建
clean_build() {
    print_header "清理旧的构建文件"
    
    if [ "$1" == "--clean" ]; then
        print_info "执行完全清理..."
        cargo clean
        print_success "清理完成"
    else
        print_warning "跳过清理（使用 --clean 参数进行完全清理）"
    fi
}

# 编译单个二进制文件
build_binary() {
    local package=$1
    local binary=$2
    local description=$3

    print_info "编译 ${binary}... (${description})"

    # 捕获编译输出并检查错误
    local build_output
    build_output=$(cargo build --release -p "$package" 2>&1)
    local build_status=$?

    # 检查是否有编译错误
    if echo "$build_output" | grep -q "^error\["; then
        print_error "${binary} 编译失败"
        echo "$build_output" | grep "^error\[" | head -5
        return 1
    fi

    # 检查编译是否成功
    if [ $build_status -ne 0 ]; then
        print_error "${binary} 编译失败（退出码: $build_status）"
        return 1
    fi

    # 检查二进制文件是否存在
    if [ -f "target/release/${binary}" ]; then
        local size=$(ls -lh "target/release/${binary}" | awk '{print $5}')
        print_success "${binary} 编译成功 (${size})"
        return 0
    else
        print_error "${binary} 编译失败：未找到二进制文件"
        return 1
    fi
}

# 编译所有二进制文件
build_all() {
    print_header "编译所有二进制文件"
    
    local start_time=$(date +%s)
    local success_count=0
    local total_count=4
    
    # 编译 astgrep 主程序
    if build_binary "astgrep" "astgrep" "主程序 - 命令行工具"; then
        ((success_count++))
    fi
    
    echo ""
    
    # 编译 astgrep-cli
    if build_binary "astgrep-cli" "astgrep-cli" "独立 CLI 工具"; then
        ((success_count++))
    fi
    
    echo ""
    
    # 编译 astgrep-web
    if build_binary "astgrep-web" "astgrep-web" "Web 服务和 REST API"; then
        ((success_count++))
    fi
    
    echo ""
    
    # 编译 astgrep-gui
    if build_binary "astgrep-gui" "astgrep-gui" "图形界面 Playground"; then
        ((success_count++))
    fi
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo ""
    print_header "编译结果"
    
    if [ $success_count -eq $total_count ]; then
        print_success "所有 ${total_count} 个二进制文件编译成功！"
    else
        print_warning "成功: ${success_count}/${total_count}"
    fi
    
    print_info "总耗时: ${duration} 秒"
}

# 显示编译结果
show_results() {
    print_header "编译的二进制文件"
    
    echo ""
    printf "%-20s %-10s %-40s\n" "文件名" "大小" "路径"
    echo "────────────────────────────────────────────────────────────────────"
    
    for binary in astgrep astgrep-cli astgrep-web astgrep-gui; do
        if [ -f "target/release/${binary}" ]; then
            local size=$(ls -lh "target/release/${binary}" | awk '{print $5}')
            local path="target/release/${binary}"
            printf "%-20s %-10s %-40s\n" "$binary" "$size" "$path"
        fi
    done
    
    echo ""
}

# 运行测试
run_tests() {
    print_header "运行快速测试"
    
    local test_count=0
    local pass_count=0
    
    for binary in astgrep astgrep-cli astgrep-web astgrep-gui; do
        if [ -f "target/release/${binary}" ]; then
            ((test_count++))
            print_info "测试 ${binary}..."
            
            if [ "$binary" == "astgrep-web" ] || [ "$binary" == "astgrep-gui" ]; then
                # Web 和 GUI 只检查文件是否可执行
                if [ -x "target/release/${binary}" ]; then
                    print_success "${binary} 可执行"
                    ((pass_count++))
                else
                    print_error "${binary} 不可执行"
                fi
            else
                # CLI 工具测试 --version
                if ./target/release/${binary} --version &> /dev/null; then
                    print_success "${binary} --version 测试通过"
                    ((pass_count++))
                else
                    print_error "${binary} --version 测试失败"
                fi
            fi
        fi
    done
    
    echo ""
    if [ $pass_count -eq $test_count ]; then
        print_success "所有测试通过 (${pass_count}/${test_count})"
    else
        print_warning "测试结果: ${pass_count}/${test_count}"
    fi
}

# 显示使用说明
show_usage() {
    print_header "使用说明"
    
    echo "运行编译的程序："
    echo ""
    echo "  # 主程序"
    echo "  ./target/release/astgrep --help"
    echo ""
    echo "  # CLI 工具"
    echo "  ./target/release/astgrep-cli --help"
    echo ""
    echo "  # Web 服务（需要先创建 rules 目录）"
    echo "  mkdir -p rules"
    echo "  ./target/release/astgrep-web"
    echo ""
    echo "  # GUI 应用"
    echo "  ./target/release/astgrep-gui"
    echo ""
    echo "安装到系统："
    echo ""
    echo "  sudo cp target/release/astgrep* /usr/local/bin/"
    echo ""
    echo "查看详细文档："
    echo ""
    echo "  cat docs/BUILD_GUIDE.md"
    echo ""
}

# 主函数
main() {
    print_header "🚀 astgrep 编译脚本"
    
    echo "开始编译 astgrep 所有二进制文件..."
    echo ""
    
    # 检查 Rust 环境
    check_rust
    
    # 清理（如果指定）
    clean_build "$1"
    
    # 编译所有二进制文件
    build_all
    
    # 显示结果
    show_results
    
    # 运行测试
    if [ "$1" != "--no-test" ]; then
        run_tests
    fi
    
    # 显示使用说明
    show_usage
    
    print_header "🎉 完成"
    print_success "所有任务完成！"
}

# 显示帮助
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "astgrep 编译脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --clean      编译前清理所有构建文件"
    echo "  --no-test    跳过测试步骤"
    echo "  --help, -h   显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0              # 正常编译"
    echo "  $0 --clean      # 清理后编译"
    echo "  $0 --no-test    # 编译但不测试"
    exit 0
fi

# 运行主函数
main "$@"

