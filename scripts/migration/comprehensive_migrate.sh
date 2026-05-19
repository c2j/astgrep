#!/bin/bash

# 全面迁移所有测试用例
# 根据tests目录的实际结构进行智能迁移

set -e

echo "🚀 开始全面迁移所有测试用例..."

# 确保目标目录结构存在
echo "📁 确认目录结构..."
mkdir -p newtest/testcases/{java,javascript,python,sql,bash,php,csharp,c,cpp,ruby,kotlin,swift,xml,go,rust,perl,typescript,ocaml,clojure,html,json,dart,hack,lua,solidity,scala,terraform,r,js,dockerfile,move,cairo}/{basic,pattern-matching,rule-validation,parsing,integration,performance,security,compatibility,custom}
mkdir -p newtest/testcases/{misc,explanations,e-rules,parsing_errors,semgrep_output}
echo "✅ 目录结构确认完成"

# 迁移计数器
total_yaml=0
total_code=0
total_migrated=0

echo ""
echo "📦 开始迁移测试用例..."

# 1. 按语言目录迁移
echo "🔄 按语言目录迁移..."

languages=("java" "javascript" "python" "sql" "bash" "php" "csharp" "cpp" "ruby" "kotlin" "swift" "xml" "go" "rust" "perl" "typescript" "ocaml" "clojure" "html" "json" "dart" "hack" "lua" "solidity" "scala" "terraform" "r" "js" "dockerfile" "move" "cairo")

for lang in "${languages[@]}"; do
    if [ -d "tests/patterns/$lang" ]; then
        echo "  📁 迁移 $lang 测试用例..."

        # 查找所有yaml文件
        yaml_count=$(find tests/patterns/$lang -name "*.yaml" | wc -l)
        code_count=$(find tests/patterns/$lang -name "*.$lang" -o -name "*.py" -o -name "*.js" -o -name "*.java" -o -name "*.cpp" -o -name "*.c" -o -name "*.rs" -o -name "*.go" | wc -l)

        # 复制yaml文件
        find tests/patterns/$lang -name "*.yaml" -exec cp {} newtest/testcases/$lang/pattern-matching/ \;

        # 复制对应的代码文件
        find tests/patterns/$lang -name "*.$lang" -o -name "*.py" -o -name "*.js" -o -name "*.java" -o -name "*.cpp" -o -name "*.c" -o -name "*.rs" -o -name "*.go" | while read file; do
            cp "$file" "newtest/testcases/$lang/pattern-matching/"
        done

        echo "    ✅ $lang: $yaml_count yaml, $code code 文件"
        total_yaml=$((total_yaml + yaml_count))
        total_code=$((total_code + code_count))
        total_migrated=$((total_migrated + yaml_count + code_count))
    fi
done

# 2. 迁移java目录
if [ -d "tests/java" ]; then
    echo "📁 迁移专用java目录..."
    java_yaml=$(find tests/java -name "*.yaml" | wc -l)
    java_code=$(find tests/java -name "*.java" | wc -l)
    find tests/java -name "*.yaml" -exec cp {} newtest/testcases/java/basic/ \;
    find tests/java -name "*.java" -exec cp {} newtest/testcases/java/basic/ \;
    echo "  ✅ Java专用目录: $java_yaml yaml, $java_code code 文件"
    total_yaml=$((total_yaml + java_yaml))
    total_code=$((total_code + java_code))
    total_migrated=$((total_migrated + java_yaml + java_code))
fi

# 3. 迁移misc目录
if [ -d "tests/misc" ]; then
    echo "📁 迁移misc目录..."
    misc_yaml=$(find tests/misc -name "*.yaml" | wc -l)
    misc_code=$(find tests/misc -name "*.py" -o -name "*.java" -o -name "*.js" | wc -l)
    find tests/misc -name "*.yaml" -exec cp {} newtest/testcases/misc/basic/ \;
    find tests/misc -name "*.py" -o -name "*.java" -o -name "*.js" | while read file; do
        cp "$file" "newtest/testcases/misc/basic/"
    done
    echo "  ✅ Misc目录: $misc_yaml yaml, $misc_code code 文件"
    total_yaml=$((total_yaml + misc_yaml))
    total_code=$((total_code + misc_code))
    total_migrated=$((total_migrated + misc_yaml + misc_code))
fi

# 4. 迁移e-rules目录
if [ -d "tests/e-rules" ]; then
    echo "📁 迁移e-rules目录..."
    erules_yaml=$(find tests/e-rules -name "*.yaml" | wc -l)
    find tests/e-rules -name "*.yaml" -exec cp {} newtest/testcases/e-rules/rule-validation/ \;
    echo "  ✅ E-rules目录: $erules_yaml yaml 文件"
    total_yaml=$((total_yaml + erules_yaml))
    total_migrated=$((total_migrated + erules_yaml))
fi

# 5. 迁移explanations目录
if [ -d "tests/explanations" ]; then
    echo "📁 迁移explanations目录..."
    exp_yaml=$(find tests/explanations -name "*.yaml" | wc -l)
    find tests/explanations -name "*.yaml" -exec cp {} newtest/testcases/explanations/basic/ \;
    echo "  ✅ Explanations目录: $exp_yaml yaml 文件"
    total_yaml=$((total_yaml + exp_yaml))
    total_migrated=$((total_migrated + exp_yaml))
fi

# 6. 迁移parsing_errors目录
if [ -d "tests/parsing_errors" ]; then
    echo "📁 迁移parsing_errors目录..."
    parse_yaml=$(find tests/parsing_errors -name "*.yaml" | wc -l)
    find tests/parsing_errors -name "*.yaml" -exec cp {} newtest/testcases/parsing_errors/parsing/ \;
    echo "  ✅ Parsing_errors目录: $parse_yaml yaml 文件"
    total_yaml=$((total_yaml + parse_yaml))
    total_migrated=$((total_migrated + parse_yaml))
fi

# 7. 迁移剩余的根目录yaml文件
echo "📁 迁移根目录yaml文件..."
root_yaml=$(find tests -maxdepth 1 -name "*.yaml" | wc -l)
find tests -maxdepth 1 -name "*.yaml" -exec cp {} newtest/testcases/misc/basic/ \;
echo "  ✅ 根目录: $root_yaml yaml 文件"
total_yaml=$((total_yaml + root_yaml))
total_migrated=$((total_migrated + root_yaml))

# 8. 迁移所有脚本文件
echo "📁 迁移所有脚本文件..."
script_count=$(find tests -name "*.sh" | wc -l)
find tests -name "*.sh" -exec cp {} newtest/scripts/validation/ \;
echo "  ✅ 脚本: $script_count 个文件"
total_migrated=$((total_migrated + script_count))

echo ""
echo "📊 迁移完成统计："
echo "原始tests目录yaml文件: $(find tests -name "*.yaml" | wc -l) 个"
echo "迁移到newtest目录yaml文件: $total_yaml 个"
echo "迁移代码文件: $total_code 个"
echo "迁移脚本文件: $script_count 个"
echo "总迁移文件: $total_migrated 个"
echo ""
echo "📁 按语言分布："
for lang in "${languages[@]}"; do
    count=$(find newtest/testcases/$lang -name "*.yaml" 2>/dev/null | wc -l)
    if [ $count -gt 0 ]; then
        echo "  $lang: $count 个yaml文件"
    fi
done

echo ""
echo "✅ 全面迁移完成！"
echo "💡 运行测试: python3 newtest/scripts/runners/comprehensive_test_runner.py --structured-only"