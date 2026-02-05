#!/bin/bash

# 简单批量迁移脚本
# 实际可行的迁移方案

set -e

echo "🚀 开始批量迁移测试用例..."

# 创建目标目录结构
echo "📁 创建目录结构..."
mkdir -p newtest/testcases/{java,javascript,python,sql,bash,php,csharp,c,ruby,kotlin,swift,xml,go,rust,perl}/{basic,pattern-matching,rule-validation,parsing,integration,performance,security,compatibility,custom}
mkdir -p newtest/scripts/{validation,performance,compatibility,benchmarking}

echo "✅ 目录结构创建完成"

# 迁移Java测试用例
echo "📦 迁移Java测试用例..."
if [ -d "tests/java" ]; then
    find tests/java -name "*.yaml" -exec cp {} newtest/testcases/java/basic/ \;
    find tests/java -name "*.java" -exec cp {} newtest/testcases/java/basic/ \;
    echo "✅ Java测试用例迁移完成"
fi

# 迁移Python测试用例
echo "📦 迁移Python测试用例..."
if [ -d "tests/python" ]; then
    find tests/python -name "*.yaml" -exec cp {} newtest/testcases/python/basic/ \; 2>/dev/null || true
    find tests/python -name "*.py" -exec cp {} newtest/testcases/python/basic/ \; 2>/dev/null || true
    echo "✅ Python测试用例迁移完成"
fi

# 迁移SQL测试用例
echo "📦 迁移SQL测试用例..."
if [ -d "tests/sql" ]; then
    find tests/sql -name "*.yaml" -exec cp {} newtest/testcases/sql/basic/ \; 2>/dev/null || true
    find tests/sql -name "*.sql" -exec cp {} newtest/testcases/sql/basic/ \; 2>/dev/null || true
    echo "✅ SQL测试用例迁移完成"
fi

# 迁移安全测试用例
echo "📦 迁移安全测试用例..."
find tests -name "*security*" -name "*.yaml" -exec cp {} newtest/testcases/security/basic/ \; 2>/dev/null || true
find tests -name "*security*" -name "*.java" -exec cp {} newtest/testcases/java/security/ \; 2>/dev/null || true
find tests -name "*security*" -name "*.py" -exec cp {} newtest/testcases/python/security/ \; 2>/dev/null || true
echo "✅ 安全测试用例迁移完成"

# 迁移脚本文件
echo "📦 迁移脚本文件..."
find tests -name "*.sh" -not -path "*/patterns/*" -exec cp {} newtest/scripts/validation/ \;
echo "✅ 脚本文件迁移完成"

# 统计迁移结果
echo ""
echo "📊 迁移结果统计："
echo "Java测试用例: $(find newtest/testcases/java -name "*.yaml" -o -name "*.java" | wc -l) 个文件"
echo "Python测试用例: $(find newtest/testcases/python -name "*.yaml" -o -name "*.py" | wc -l) 个文件"
echo "SQL测试用例: $(find newtest/testcases/sql -name "*.yaml" -o -name "*.sql" | wc -l) 个文件"
echo "脚本文件: $(find newtest/scripts -name "*.sh" | wc -l) 个文件"
echo "总迁移文件: $(find newtest -type f | wc -l) 个"

echo ""
echo "✅ 批量迁移完成！"
echo "💡 运行测试: python3 newtest/scripts/runners/comprehensive_test_runner.py --structured-only"