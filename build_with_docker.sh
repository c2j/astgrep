#!/bin/bash

# 使用 Docker 编译 Linux 静态二进制文件

set -e

# 颜色定义
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}🐳 使用 Docker 编译 Linux 静态二进制文件${NC}"
echo ""

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}❌ 未找到 Docker，请先安装 Docker${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Docker 版本: $(docker --version)${NC}"
echo ""

# 构建 Docker 镜像
echo -e "${BLUE}📦 构建 Docker 镜像...${NC}"
docker build -f Dockerfile.linux-static --target export -t astgrep-builder .

# 创建输出目录
mkdir -p dist/linux-x86_64

# 提取二进制文件
echo ""
echo -e "${BLUE}📤 提取二进制文件...${NC}"

# 创建临时容器
CONTAINER_ID=$(docker create astgrep-builder)

# 复制二进制文件
docker cp "$CONTAINER_ID:/export/astgrep" dist/linux-x86_64/
docker cp "$CONTAINER_ID:/export/astgrep-cli" dist/linux-x86_64/
docker cp "$CONTAINER_ID:/export/astgrep-web" dist/linux-x86_64/

# 复制压缩包
docker cp "$CONTAINER_ID:/astgrep-linux-x86_64.tar.gz" dist/

# 删除临时容器
docker rm "$CONTAINER_ID" > /dev/null

echo -e "${GREEN}✅ 二进制文件已提取到 dist/linux-x86_64/${NC}"
echo ""

# 显示结果
echo -e "${BLUE}📊 编译结果:${NC}"
echo ""
ls -lh dist/linux-x86_64/
echo ""
echo -e "${BLUE}📦 压缩包:${NC}"
ls -lh dist/astgrep-linux-x86_64.tar.gz
echo ""

# 验证二进制文件
echo -e "${BLUE}🔍 验证二进制文件:${NC}"
echo ""
for binary in astgrep astgrep-cli astgrep-web; do
    echo "  $binary:"
    file "dist/linux-x86_64/$binary" | sed 's/^/    /'
done
echo ""

echo -e "${GREEN}🎉 编译完成！${NC}"
echo ""
echo "使用方法:"
echo "  1. 复制到 Linux 服务器:"
echo "     scp dist/astgrep-linux-x86_64.tar.gz user@server:/tmp/"
echo ""
echo "  2. 在 Linux 上解压:"
echo "     tar xzf /tmp/astgrep-linux-x86_64.tar.gz"
echo ""
echo "  3. 运行:"
echo "     ./astgrep --version"

