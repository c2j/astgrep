#!/bin/bash

# Test script for astgrep-gui Chinese font support

echo "🚀 Starting astgrep-gui..."
echo "📝 Please check if Chinese characters display correctly in the GUI"
echo ""
echo "Expected Chinese text in the GUI:"
echo "  - 简单 (Simple)"
echo "  - 高级 (Advanced)"
echo "  - 检查规则 (Inspect Rule)"
echo "  - 测试代码 (Test Code)"
echo "  - 元数据 (Metadata)"
echo "  - 文档 (Docs)"
echo "  - 匹配 (Matches)"
echo ""
echo "If you see boxes (□) instead of Chinese characters, the font loading failed."
echo ""

./target/release/astgrep-gui

