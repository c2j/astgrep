#!/bin/bash

# 快速迁移所有测试用例 - 简化但有效版本

set -e

echo "🚀 快速迁移所有测试用例..."

# 确保基础目录存在
mkdir -p newtest/testcases/misc/basic
mkdir -p newtest/testcases/{java,javascript,python,sql,bash,php,csharp,c,cpp,ruby,kotlin,swift,xml,go,rust,perl,typescript,ocaml,clojure,html,json,dart,hack,lua,solidity,scala,terraform,r,js,dockerfile,move,cairo}/pattern-matching
mkdir -p newtest/testcases/{misc,e-rules,explanations,parsing_errors,semgrep_output}/basic

echo "✅ 目录创建完成"

echo ""
echo "📦 开始迁移所有测试用例..."

# 1. 直接复制所有yaml文件
echo "📁 迁移所有yaml文件..."
yaml_count=0
find tests -name "*.yaml" | while read yaml_file; do
    # 确定目标语言
    if [[ "$yaml_file" == *"/patterns/"* ]]; then
        # 从路径中提取语言
        lang=$(echo "$yaml_file" | sed 's|.*/patterns/||' | sed 's|/.*||')
        # 如果语言不在我们支持的列表中，归类为misc
        if [[ ! -d "newtest/testcases/$lang" ]]; then
            lang="misc"
        fi
        target_dir="newtest/testcases/$lang/pattern-matching"
    elif [[ "$yaml_file" == *"/java/"* ]]; then
        target_dir="newtest/testcases/java/basic"
    elif [[ "$yaml_file" == *"/misc/"* ]]; then
        target_dir="newtest/testcases/misc/basic"
    elif [[ "$yaml_file" == *"/e-rules/"* ]]; then
        target_dir="newtest/testcases/e-rules/basic"
    elif [[ "$yaml_file" == *"/explanations/"* ]]; then
        target_dir="newtest/testcases/explanations/basic"
    elif [[ "$yaml_file" == *"/parsing_errors/"* ]]; then
        target_dir="newtest/testcases/parsing_errors/basic"
    else
        target_dir="newtest/testcases/misc/basic"
    fi

    cp "$yaml_file" "$target_dir/"
    yaml_count=$((yaml_count + 1))
done

echo "  ✅ 迁移了 $yaml_count 个yaml文件"

# 2. 复制所有代码文件
echo "📁 迁移所有代码文件..."
code_count=0
# 常见的代码文件扩展
find tests -name "*.py" -o -name "*.java" -o -name "*.js" -o -name "*.cpp" -o -name "*.c" -o -name "*.rs" -o -name "*.go" -o -name "*.php" -o -name "*.rb" -o -name "*.kt" -o -name "*.swift" -o -name "*.sql" -o -name "*.xml" -o -name "*.html" -o -name "*.json" | while read code_file; do
    # 简单归类：根据文件扩展名和路径
    ext="${code_file##*.}"

    if [[ "$code_file" == *"/patterns/"* ]]; then
        # 从路径中提取语言
        lang=$(echo "$code_file" | sed 's|.*/patterns/||' | sed 's|/.*||')
        if [[ ! -d "newtest/testcases/$lang" ]]; then
            lang="misc"
        fi
        target_dir="newtest/testcases/$lang/pattern-matching"
    elif [[ "$code_file" == *"/java/"* ]]; then
        target_dir="newtest/testcases/java/basic"
    else
        target_dir="newtest/testcases/misc/basic"
    fi

    cp "$code_file" "$target_dir/"
    code_count=$((code_count + 1))
done

echo "  ✅ 迁移了 $code_count 个代码文件"

# 3. 复制所有脚本文件
echo "📁 迁移所有脚本文件..."
script_count=$(find tests -name "*.sh" | wc -l)
find tests -name "*.sh" -exec cp {} newtest/scripts/validation/ \;
echo "  ✅ 迁移了 $script_count 个脚本文件"

echo ""
echo "📊 迁移完成统计："
echo "原始tests目录yaml文件: $(find tests -name "*.yaml" | wc -l) 个"
echo "迁移到newtest目录yaml文件: $(find newtest -name "*.yaml" | wc -l) 个"
echo "迁移代码文件: $(find newtest -name "*.py" -o -name "*.java" -o -name "*.js" -o -name "*.cpp" -o -name "*.c" -o -name "*.rs" -o -name "*.go" -o -name "*.php" -o -name "*.rb" -o -name "*.kt" -o -name "*.swift" -o -name "*.sql" -o -name "*.xml" -o -name "*.html" -o -name "*.json" | wc -l) 个"
echo "迁移脚本文件: $script_count 个"
echo "总迁移文件: $(find newtest -type f | wc -l) 个"

echo ""
echo "📁 主要语言分布（前10）："
for lang in java python javascript sql bash php csharp cpp ruby kotlin swift xml go rust perl typescript; do
    count=$(find newtest/testcases/$lang -name "*.yaml" 2>/dev/null | wc -l)
    if [ $count -gt 0 ]; then
        echo "  $lang: $count 个yaml文件"
    fi
done | head -10

echo ""
echo "✅ 全面迁移完成！"
echo "💡 运行测试: python3 newtest/scripts/runners/comprehensive_test_runner.py --structured-only"