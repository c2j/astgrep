# 多语言静态代码分析工具设计文档

## 1. 项目概述

本项目旨在设计和实现一个通用的静态代码分析工具，支持Java、Bash、SQL（包括SQL语句和存储过程）、JavaScript和Python等五种核心编程语言的安全漏洞和代码质量检查。

### 1.1 设计目标

- **语言无关性**: 提供统一的规则定义和执行框架
- **可扩展性**: 支持新语言和新规则类型的快速集成
- **高性能**: 支持大规模代码库的高效分析
- **准确性**: 低误报率和漏报率的精确检测
- **易用性**: 简单直观的规则编写和使用体验

### 1.2 核心特性

- **多语言支持**: Java、Bash、SQL、JavaScript、Python
- **统一规则语法**: 基于YAML的声明式规则定义
- **模式匹配**: 支持语法模式、语义模式和数据流模式
- **污点分析**: 跨函数和跨文件的数据流追踪
- **并行处理**: 多线程/多进程并行分析
- **多种输出**: JSON、SARIF、文本、XML等格式

## 2. 系统架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                    多语言代码分析工具                        │
├─────────────────────────────────────────────────────────────┤
│  用户接口层                                                 │
│  ├── CLI工具        ├── Web界面        ├── IDE插件          │
├─────────────────────────────────────────────────────────────┤
│  规则管理层                                                 │
│  ├── 规则解析器     ├── 规则验证器     ├── 规则优化器       │
├─────────────────────────────────────────────────────────────┤
│  语言抽象层                                                 │
│  ├── 通用AST       ├── 语言适配器     ├── 模式匹配器        │
├─────────────────────────────────────────────────────────────┤
│  分析引擎层                                                 │
│  ├── 语法分析      ├── 语义分析       ├── 数据流分析        │
├─────────────────────────────────────────────────────────────┤
│  语言支持层                                                 │
│  ├── Java解析器    ├── Bash解析器     ├── SQL解析器         │
│  ├── JS解析器      ├── Python解析器   ├── 扩展接口          │
├─────────────────────────────────────────────────────────────┤
│  基础设施层                                                 │
│  ├── 并行处理      ├── 缓存系统       ├── 日志监控          │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 核心组件

#### 2.2.1 规则引擎
- **规则解析器**: 解析YAML格式的规则定义
- **规则验证器**: 验证规则语法和语义正确性
- **规则编译器**: 将规则编译为高效的匹配器
- **规则索引器**: 构建规则索引以提高查找效率

#### 2.2.2 语言处理器
- **词法分析器**: 将源代码转换为token流
- **语法分析器**: 构建抽象语法树(AST)
- **语义分析器**: 进行类型检查和符号解析
- **AST转换器**: 将特定语言AST转换为通用AST

#### 2.2.3 匹配引擎
- **模式匹配器**: 执行语法和语义模式匹配
- **数据流分析器**: 进行污点分析和数据流追踪
- **结果聚合器**: 收集和合并分析结果
- **后处理器**: 应用过滤、去重和修复建议

## 3. 语言支持设计

### 3.1 通用AST设计

```yaml
# 通用AST节点类型定义
ast_node_types:
  # 基础节点
  - identifier: 标识符节点
  - literal: 字面量节点 (字符串、数字、布尔值)
  - comment: 注释节点
  
  # 表达式节点
  - binary_expression: 二元表达式 (a + b, a == b)
  - unary_expression: 一元表达式 (!a, -a)
  - call_expression: 函数调用表达式
  - member_expression: 成员访问表达式 (obj.prop)
  - assignment_expression: 赋值表达式
  
  # 语句节点
  - expression_statement: 表达式语句
  - declaration_statement: 声明语句
  - control_flow_statement: 控制流语句 (if, for, while)
  - return_statement: 返回语句
  - block_statement: 代码块语句
  
  # 声明节点
  - function_declaration: 函数声明
  - variable_declaration: 变量声明
  - class_declaration: 类声明
  - import_declaration: 导入声明
  
  # 特殊节点
  - sql_query: SQL查询节点
  - sql_procedure: SQL存储过程节点
  - shell_command: Shell命令节点
```

### 3.2 语言特定适配器

#### 3.2.1 Java语言适配器
```yaml
java_adapter:
  file_extensions: [".java"]
  parser_type: "antlr4"
  grammar_file: "Java8.g4"
  
  # Java特定AST映射
  ast_mappings:
    - java_class -> class_declaration
    - java_method -> function_declaration
    - java_field -> variable_declaration
    - java_annotation -> annotation_node
    - java_lambda -> lambda_expression
  
  # Java特定模式
  patterns:
    - sql_injection: "Statement.execute($QUERY)"
    - xss_vulnerability: "response.getWriter().write($INPUT)"
    - path_traversal: "new File($PATH)"
```

#### 3.2.2 JavaScript语言适配器
```yaml
javascript_adapter:
  file_extensions: [".js", ".jsx", ".ts", ".tsx"]
  parser_type: "tree_sitter"
  grammar_file: "tree-sitter-javascript"
  
  # JavaScript特定AST映射
  ast_mappings:
    - js_function -> function_declaration
    - js_arrow_function -> lambda_expression
    - js_object -> object_literal
    - js_template_literal -> template_string
  
  # JavaScript特定模式
  patterns:
    - eval_injection: "eval($CODE)"
    - dom_xss: "document.write($INPUT)"
    - prototype_pollution: "$OBJ.__proto__ = $VALUE"
```

#### 3.2.3 Python语言适配器
```yaml
python_adapter:
  file_extensions: [".py", ".pyw"]
  parser_type: "tree_sitter"
  grammar_file: "tree-sitter-python"
  
  # Python特定AST映射
  ast_mappings:
    - python_function_def -> function_declaration
    - python_class_def -> class_declaration
    - python_import -> import_declaration
    - python_list_comprehension -> comprehension_expression
  
  # Python特定模式
  patterns:
    - code_injection: "exec($CODE)"
    - sql_injection: "cursor.execute($QUERY)"
    - pickle_deserialization: "pickle.loads($DATA)"
```

#### 3.2.4 SQL语言适配器
```yaml
sql_adapter:
  file_extensions: [".sql", ".ddl", ".dml"]
  parser_type: "antlr4"
  grammar_file: "SQLite.g4"
  
  # SQL特定AST映射
  ast_mappings:
    - sql_select -> query_statement
    - sql_insert -> insert_statement
    - sql_update -> update_statement
    - sql_delete -> delete_statement
    - sql_procedure -> procedure_declaration
  
  # SQL特定模式
  patterns:
    - sql_injection: "EXECUTE IMMEDIATE $QUERY"
    - privilege_escalation: "GRANT ALL PRIVILEGES"
    - data_exposure: "SELECT * FROM sensitive_table"
```

#### 3.2.5 Bash语言适配器
```yaml
bash_adapter:
  file_extensions: [".sh", ".bash", ".zsh"]
  parser_type: "tree_sitter"
  grammar_file: "tree-sitter-bash"
  
  # Bash特定AST映射
  ast_mappings:
    - bash_function -> function_declaration
    - bash_command -> command_statement
    - bash_pipeline -> pipeline_expression
    - bash_variable -> variable_reference
  
  # Bash特定模式
  patterns:
    - command_injection: "eval $INPUT"
    - path_traversal: "cat $FILE"
    - privilege_escalation: "sudo $COMMAND"
```

## 4. 规则系统设计

### 4.1 统一规则语法

```yaml
# 规则定义模板
rule_template:
  # 基本信息
  id: "rule-identifier"
  name: "规则名称"
  description: "规则描述"
  severity: "ERROR|WARNING|INFO"
  confidence: "HIGH|MEDIUM|LOW"
  
  # 适用语言
  languages: ["java", "javascript", "python", "sql", "bash"]
  
  # 匹配模式
  patterns:
    - pattern: "模式表达式"
    - pattern_either: ["模式1", "模式2"]
    - pattern_not: "排除模式"
    - pattern_inside: "上下文模式"
  
  # 条件约束
  conditions:
    - metavariable_regex:
        variable: "$VAR"
        regex: "正则表达式"
    - metavariable_comparison:
        variable: "$NUM"
        comparison: "$NUM > 10"
  
  # 数据流分析
  dataflow:
    sources: ["source_pattern"]
    sinks: ["sink_pattern"]
    sanitizers: ["sanitizer_pattern"]
  
  # 修复建议
  fix: "修复代码模板"
  fix_regex:
    regex: "查找正则"
    replacement: "替换文本"
  
  # 元数据
  metadata:
    category: "security|performance|style"
    cwe: "CWE-89"
    owasp: "A03:2021"
    references: ["https://example.com"]
```

### 4.2 语言特定规则示例

#### 4.2.1 Java SQL注入检测规则
```yaml
rules:
  - id: java-sql-injection
    name: "Java SQL注入检测"
    description: "检测Java代码中的SQL注入漏洞"
    severity: "ERROR"
    languages: ["java"]
    
    patterns:
      - pattern_inside: |
          class $CLASS {
            ...
            $RETURN $METHOD(...) {
              ...
            }
          }
      - pattern_either:
          - pattern: "Statement.execute($QUERY)"
          - pattern: "Statement.executeQuery($QUERY)"
          - pattern: "Statement.executeUpdate($QUERY)"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern_either:
                - pattern: "$STR + $INPUT"
                - pattern: "String.format($STR, ..., $INPUT, ...)"
                - pattern: "$STR.concat($INPUT)"
    
    dataflow:
      sources:
        - pattern: "request.getParameter($PARAM)"
        - pattern: "request.getHeader($HEADER)"
      sinks:
        - pattern: "Statement.execute($QUERY)"
      sanitizers:
        - pattern: "PreparedStatement.setString($INDEX, $VALUE)"
    
    fix: "使用PreparedStatement: PreparedStatement stmt = conn.prepareStatement($QUERY); stmt.setString(1, $INPUT);"
    
    metadata:
      category: "security"
      cwe: "CWE-89"
      owasp: "A03:2021 - Injection"

#### 4.2.2 JavaScript XSS检测规则
```yaml
rules:
  - id: javascript-dom-xss
    name: "JavaScript DOM XSS检测"
    description: "检测JavaScript代码中的DOM XSS漏洞"
    severity: "ERROR"
    languages: ["javascript", "typescript"]

    patterns:
      - pattern_either:
          - pattern: "document.write($INPUT)"
          - pattern: "document.writeln($INPUT)"
          - pattern: "$ELEMENT.innerHTML = $INPUT"
          - pattern: "$ELEMENT.outerHTML = $INPUT"
      - metavariable_pattern:
          metavariable: "$INPUT"
          patterns:
            - pattern_either:
                - pattern: "location.search"
                - pattern: "location.hash"
                - pattern: "document.URL"
                - pattern: "$STR + $PARAM"

    dataflow:
      sources:
        - pattern: "location.search"
        - pattern: "location.hash"
        - pattern: "document.URL"
        - pattern: "window.name"
      sinks:
        - pattern: "document.write($DATA)"
        - pattern: "$ELEMENT.innerHTML = $DATA"
      sanitizers:
        - pattern: "DOMPurify.sanitize($DATA)"
        - pattern: "escapeHtml($DATA)"

    fix: "使用安全的DOM操作: $ELEMENT.textContent = $INPUT 或 $ELEMENT.innerHTML = DOMPurify.sanitize($INPUT)"

    metadata:
      category: "security"
      cwe: "CWE-79"
      owasp: "A03:2021 - Injection"

#### 4.2.3 Python代码注入检测规则
```yaml
rules:
  - id: python-code-injection
    name: "Python代码注入检测"
    description: "检测Python代码中的代码注入漏洞"
    severity: "ERROR"
    languages: ["python"]

    patterns:
      - pattern_either:
          - pattern: "exec($CODE)"
          - pattern: "eval($CODE)"
          - pattern: "compile($CODE, ...)"
          - pattern: "__import__($MODULE)"
      - metavariable_pattern:
          metavariable: "$CODE"
          patterns:
            - pattern_either:
                - pattern: "$STR + $INPUT"
                - pattern: "$STR.format(..., $INPUT, ...)"
                - pattern: "f\"...$INPUT...\""

    dataflow:
      sources:
        - pattern: "request.args.get($PARAM)"
        - pattern: "request.form.get($PARAM)"
        - pattern: "input($PROMPT)"
        - pattern: "sys.argv[$INDEX]"
      sinks:
        - pattern: "exec($CODE)"
        - pattern: "eval($CODE)"
      sanitizers:
        - pattern: "ast.literal_eval($CODE)"
        - pattern: "json.loads($CODE)"

    fix: "使用安全的替代方案: ast.literal_eval() 用于字面量求值，或使用白名单验证"

    metadata:
      category: "security"
      cwe: "CWE-94"
      owasp: "A03:2021 - Injection"

#### 4.2.4 SQL存储过程安全检测规则
```yaml
rules:
  - id: sql-procedure-injection
    name: "SQL存储过程注入检测"
    description: "检测SQL存储过程中的注入漏洞"
    severity: "ERROR"
    languages: ["sql"]

    patterns:
      - pattern_inside: |
          CREATE PROCEDURE $PROC_NAME(...)
          AS
          BEGIN
            ...
          END
      - pattern_either:
          - pattern: "EXECUTE IMMEDIATE $QUERY"
          - pattern: "EXEC($QUERY)"
          - pattern: "sp_executesql $QUERY"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern_either:
                - pattern: "$STR + $PARAM"
                - pattern: "CONCAT($STR, $PARAM)"
                - pattern: "$STR || $PARAM"

    conditions:
      - metavariable_regex:
          metavariable: "$PARAM"
          regex: "@\\w+"  # 匹配存储过程参数

    fix: "使用参数化查询: EXEC sp_executesql @sql, N'@param NVARCHAR(50)', @param = @input"

    metadata:
      category: "security"
      cwe: "CWE-89"
      owasp: "A03:2021 - Injection"

#### 4.2.5 Bash命令注入检测规则
```yaml
rules:
  - id: bash-command-injection
    name: "Bash命令注入检测"
    description: "检测Bash脚本中的命令注入漏洞"
    severity: "ERROR"
    languages: ["bash"]

    patterns:
      - pattern_either:
          - pattern: "eval $INPUT"
          - pattern: "bash -c $INPUT"
          - pattern: "sh -c $INPUT"
          - pattern: "system($INPUT)"
          - pattern: "`$INPUT`"
          - pattern: "$($INPUT)"
      - metavariable_pattern:
          metavariable: "$INPUT"
          patterns:
            - pattern_either:
                - pattern: "$STR$VAR"
                - pattern: "\"$STR$VAR\""
                - pattern: "'$STR$VAR'"

    dataflow:
      sources:
        - pattern: "$1"  # 命令行参数
        - pattern: "$@"  # 所有参数
        - pattern: "read $VAR"  # 用户输入
      sinks:
        - pattern: "eval $CMD"
        - pattern: "bash -c $CMD"
      sanitizers:
        - pattern: "printf '%q' $VAR"  # Bash引号转义
        - pattern: "shellcheck $VAR"

    fix: "使用数组和引号保护: cmd_array=($SAFE_CMD) && \"${cmd_array[@]}\""

    metadata:
      category: "security"
      cwe: "CWE-78"
      owasp: "A03:2021 - Injection"

## 5. 核心算法设计

### 5.1 通用AST构建算法

```python
class UniversalASTBuilder:
    """通用AST构建器"""

    def __init__(self, language_adapter):
        self.adapter = language_adapter
        self.node_factory = ASTNodeFactory()

    def build_ast(self, source_code):
        """构建通用AST"""
        # 1. 语言特定解析
        language_ast = self.adapter.parse(source_code)

        # 2. 转换为通用AST
        universal_ast = self.convert_to_universal(language_ast)

        # 3. 语义分析增强
        self.enhance_with_semantics(universal_ast)

        return universal_ast

    def convert_to_universal(self, language_ast):
        """将语言特定AST转换为通用AST"""
        mappings = self.adapter.get_ast_mappings()

        def convert_node(node):
            node_type = type(node).__name__
            if node_type in mappings:
                universal_type = mappings[node_type]
                universal_node = self.node_factory.create(universal_type)

                # 复制属性
                for attr, value in node.attributes.items():
                    if isinstance(value, list):
                        universal_node.children = [convert_node(child) for child in value]
                    else:
                        setattr(universal_node, attr, value)

                return universal_node
            else:
                # 保持原始节点
                return node

        return convert_node(language_ast)

    def enhance_with_semantics(self, ast):
        """增强AST的语义信息"""
        # 符号表构建
        symbol_table = SymbolTableBuilder().build(ast)

        # 类型推断
        type_inferrer = TypeInferrer(symbol_table)
        type_inferrer.infer_types(ast)

        # 控制流分析
        cfg_builder = ControlFlowGraphBuilder()
        cfg = cfg_builder.build(ast)

        # 数据流分析
        dataflow_analyzer = DataFlowAnalyzer(cfg)
        dataflow_analyzer.analyze(ast)
```

### 5.2 模式匹配算法

```python
class PatternMatcher:
    """通用模式匹配器"""

    def __init__(self):
        self.metavar_bindings = {}
        self.match_cache = {}

    def match_pattern(self, pattern, ast_node):
        """匹配模式与AST节点"""
        # 缓存检查
        cache_key = (pattern.id, ast_node.id)
        if cache_key in self.match_cache:
            return self.match_cache[cache_key]

        result = self._match_recursive(pattern, ast_node)
        self.match_cache[cache_key] = result
        return result

    def _match_recursive(self, pattern, node):
        """递归匹配算法"""
        if isinstance(pattern, MetaVariable):
            return self._match_metavariable(pattern, node)
        elif isinstance(pattern, Ellipsis):
            return self._match_ellipsis(pattern, node)
        elif isinstance(pattern, ASTPattern):
            return self._match_structure(pattern, node)
        else:
            return pattern == node

    def _match_metavariable(self, metavar, node):
        """匹配元变量"""
        var_name = metavar.name

        if var_name in self.metavar_bindings:
            # 检查一致性
            return self.metavar_bindings[var_name] == node
        else:
            # 新绑定
            if self._validate_metavar_constraints(metavar, node):
                self.metavar_bindings[var_name] = node
                return True
            return False

    def _match_ellipsis(self, ellipsis, node):
        """匹配省略号（可变长度匹配）"""
        # 省略号可以匹配零个或多个节点
        return True

    def _match_structure(self, pattern, node):
        """匹配结构模式"""
        if pattern.node_type != node.node_type:
            return False

        # 匹配子节点
        if len(pattern.children) != len(node.children):
            return False

        for pattern_child, node_child in zip(pattern.children, node.children):
            if not self._match_recursive(pattern_child, node_child):
                return False

        return True

    def _validate_metavar_constraints(self, metavar, node):
        """验证元变量约束"""
        for constraint in metavar.constraints:
            if isinstance(constraint, RegexConstraint):
                if not constraint.regex.match(str(node)):
                    return False
            elif isinstance(constraint, TypeConstraint):
                if not isinstance(node, constraint.expected_type):
                    return False

        return True

### 5.3 数据流分析算法

```python
class DataFlowAnalyzer:
    """数据流分析器"""

    def __init__(self, control_flow_graph):
        self.cfg = control_flow_graph
        self.taint_state = {}
        self.worklist = []

    def analyze_taint_flow(self, sources, sinks, sanitizers):
        """污点分析主算法"""
        # 初始化污点源
        for source in sources:
            source_nodes = self._find_matching_nodes(source)
            for node in source_nodes:
                self._mark_as_tainted(node)
                self.worklist.append(node)

        # 工作列表算法
        while self.worklist:
            current_node = self.worklist.pop(0)

            # 检查是否为净化器
            if self._is_sanitizer(current_node, sanitizers):
                self._mark_as_clean(current_node)
                continue

            # 传播污点
            for successor in self.cfg.get_successors(current_node):
                if self._propagate_taint(current_node, successor):
                    self.worklist.append(successor)

        # 检查污点是否到达汇点
        vulnerabilities = []
        for sink in sinks:
            sink_nodes = self._find_matching_nodes(sink)
            for node in sink_nodes:
                if self._is_tainted(node):
                    path = self._construct_taint_path(node)
                    vulnerabilities.append({
                        'sink': node,
                        'path': path,
                        'severity': 'HIGH'
                    })

        return vulnerabilities

    def _propagate_taint(self, from_node, to_node):
        """传播污点状态"""
        if not self._is_tainted(from_node):
            return False

        # 检查数据依赖关系
        if self._has_data_dependency(from_node, to_node):
            if not self._is_tainted(to_node):
                self._mark_as_tainted(to_node)
                return True

        return False

    def _construct_taint_path(self, sink_node):
        """构建污点传播路径"""
        path = []
        current = sink_node

        while current:
            path.append(current)
            current = self._get_taint_source(current)

        return list(reversed(path))

class LanguageSpecificAnalyzer:
    """语言特定分析器基类"""

    def __init__(self, language):
        self.language = language
        self.built_in_functions = self._load_built_in_functions()
        self.security_patterns = self._load_security_patterns()

    def analyze_security_issues(self, ast):
        """分析安全问题"""
        issues = []

        # 语言特定的安全检查
        issues.extend(self._check_injection_vulnerabilities(ast))
        issues.extend(self._check_authentication_issues(ast))
        issues.extend(self._check_authorization_issues(ast))
        issues.extend(self._check_cryptographic_issues(ast))

        return issues

    def _check_injection_vulnerabilities(self, ast):
        """检查注入漏洞"""
        raise NotImplementedError("子类必须实现此方法")

class JavaAnalyzer(LanguageSpecificAnalyzer):
    """Java语言分析器"""

    def __init__(self):
        super().__init__("java")

    def _check_injection_vulnerabilities(self, ast):
        """检查Java注入漏洞"""
        issues = []

        # SQL注入检查
        sql_patterns = [
            "Statement.execute*",
            "PreparedStatement.execute*",
            "Connection.prepareStatement"
        ]

        for pattern in sql_patterns:
            matches = self._find_pattern_matches(ast, pattern)
            for match in matches:
                if self._is_dynamic_query(match):
                    issues.append({
                        'type': 'SQL_INJECTION',
                        'location': match.location,
                        'severity': 'HIGH',
                        'message': 'Potential SQL injection vulnerability'
                    })

        # LDAP注入检查
        ldap_patterns = ["DirContext.search", "LdapContext.search"]
        for pattern in ldap_patterns:
            matches = self._find_pattern_matches(ast, pattern)
            for match in matches:
                if self._contains_user_input(match):
                    issues.append({
                        'type': 'LDAP_INJECTION',
                        'location': match.location,
                        'severity': 'HIGH',
                        'message': 'Potential LDAP injection vulnerability'
                    })

        return issues

    def _is_dynamic_query(self, node):
        """检查是否为动态构建的查询"""
        # 检查字符串拼接、格式化等
        if hasattr(node, 'arguments'):
            for arg in node.arguments:
                if self._is_string_concatenation(arg):
                    return True
                if self._is_string_format(arg):
                    return True
        return False

class JavaScriptAnalyzer(LanguageSpecificAnalyzer):
    """JavaScript语言分析器"""

    def __init__(self):
        super().__init__("javascript")

    def _check_injection_vulnerabilities(self, ast):
        """检查JavaScript注入漏洞"""
        issues = []

        # XSS检查
        xss_sinks = [
            "document.write",
            "document.writeln",
            "innerHTML",
            "outerHTML"
        ]

        for sink in xss_sinks:
            matches = self._find_pattern_matches(ast, sink)
            for match in matches:
                if self._contains_untrusted_data(match):
                    issues.append({
                        'type': 'XSS',
                        'location': match.location,
                        'severity': 'HIGH',
                        'message': 'Potential XSS vulnerability'
                    })

        # 代码注入检查
        code_injection_patterns = ["eval", "Function", "setTimeout", "setInterval"]
        for pattern in code_injection_patterns:
            matches = self._find_pattern_matches(ast, pattern)
            for match in matches:
                if self._contains_user_input(match):
                    issues.append({
                        'type': 'CODE_INJECTION',
                        'location': match.location,
                        'severity': 'CRITICAL',
                        'message': 'Potential code injection vulnerability'
                    })

        return issues

class PythonAnalyzer(LanguageSpecificAnalyzer):
    """Python语言分析器"""

    def __init__(self):
        super().__init__("python")

    def _check_injection_vulnerabilities(self, ast):
        """检查Python注入漏洞"""
        issues = []

        # 代码注入检查
        dangerous_functions = ["exec", "eval", "compile", "__import__"]
        for func in dangerous_functions:
            matches = self._find_pattern_matches(ast, func)
            for match in matches:
                if self._contains_user_input(match):
                    issues.append({
                        'type': 'CODE_INJECTION',
                        'location': match.location,
                        'severity': 'CRITICAL',
                        'message': f'Dangerous use of {func} with user input'
                    })

        # SQL注入检查（针对Python数据库库）
        sql_patterns = [
            "cursor.execute",
            "cursor.executemany",
            "connection.execute"
        ]

        for pattern in sql_patterns:
            matches = self._find_pattern_matches(ast, pattern)
            for match in matches:
                if self._is_string_formatting_query(match):
                    issues.append({
                        'type': 'SQL_INJECTION',
                        'location': match.location,
                        'severity': 'HIGH',
                        'message': 'Potential SQL injection via string formatting'
                    })

        return issues

class SQLAnalyzer(LanguageSpecificAnalyzer):
    """SQL语言分析器"""

    def __init__(self):
        super().__init__("sql")

    def _check_injection_vulnerabilities(self, ast):
        """检查SQL注入漏洞"""
        issues = []

        # 动态SQL检查
        dynamic_sql_patterns = [
            "EXECUTE IMMEDIATE",
            "EXEC",
            "sp_executesql"
        ]

        for pattern in dynamic_sql_patterns:
            matches = self._find_pattern_matches(ast, pattern)
            for match in matches:
                if self._is_concatenated_query(match):
                    issues.append({
                        'type': 'SQL_INJECTION',
                        'location': match.location,
                        'severity': 'HIGH',
                        'message': 'Dynamic SQL with string concatenation'
                    })

        # 权限提升检查
        privilege_patterns = [
            "GRANT ALL PRIVILEGES",
            "GRANT DBA",
            "ALTER USER.*IDENTIFIED BY"
        ]

        for pattern in privilege_patterns:
            matches = self._find_pattern_matches(ast, pattern)
            for match in matches:
                issues.append({
                    'type': 'PRIVILEGE_ESCALATION',
                    'location': match.location,
                    'severity': 'MEDIUM',
                    'message': 'Potential privilege escalation'
                })

        return issues

class BashAnalyzer(LanguageSpecificAnalyzer):
    """Bash语言分析器"""

    def __init__(self):
        super().__init__("bash")

    def _check_injection_vulnerabilities(self, ast):
        """检查Bash命令注入漏洞"""
        issues = []

        # 命令注入检查
        dangerous_constructs = [
            "eval",
            "bash -c",
            "sh -c",
            "system"
        ]

        for construct in dangerous_constructs:
            matches = self._find_pattern_matches(ast, construct)
            for match in matches:
                if self._contains_variable_expansion(match):
                    issues.append({
                        'type': 'COMMAND_INJECTION',
                        'location': match.location,
                        'severity': 'HIGH',
                        'message': f'Command injection via {construct}'
                    })

        # 路径遍历检查
        file_operations = ["cat", "cp", "mv", "rm", "chmod", "chown"]
        for op in file_operations:
            matches = self._find_pattern_matches(ast, op)
            for match in matches:
                if self._contains_user_controlled_path(match):
                    issues.append({
                        'type': 'PATH_TRAVERSAL',
                        'location': match.location,
                        'severity': 'MEDIUM',
                        'message': f'Potential path traversal in {op}'
                    })

        return issues

## 6. 实现架构设计

### 6.1 技术栈选择

#### 6.1.1 核心引擎技术栈
| 组件 | 技术选择 | 理由 |
|------|----------|------|
| 主要语言 | Java/Kotlin | 跨平台、生态丰富、性能优秀 |
| 解析器框架 | ANTLR4 + Tree-sitter | ANTLR4用于复杂语法，Tree-sitter用于增量解析 |
| 并发处理 | Java并发包 + Kotlin协程 | 高效的并行处理能力 |
| 缓存系统 | Caffeine + Redis | 内存缓存 + 分布式缓存 |
| 配置管理 | YAML + JSON | 人类可读的配置格式 |
| 日志系统 | SLF4J + Logback | 标准化日志接口 |

#### 6.1.2 可选技术栈
| 语言 | 适用场景 | 优势 |
|------|----------|------|
| Rust | 高性能核心组件 | 内存安全、零成本抽象 |
| Go | 微服务架构 | 简单部署、高并发 |
| Python | 快速原型和脚本 | 开发效率高、库丰富 |
| TypeScript | Web界面 | 类型安全的前端开发 |

### 6.2 模块化架构设计

```java
// 核心接口定义
public interface LanguageParser {
    AST parse(String sourceCode) throws ParseException;
    List<String> getSupportedExtensions();
    String getLanguageName();
}

public interface RuleEngine {
    List<Rule> loadRules(String ruleSource) throws RuleLoadException;
    List<Match> executeRules(List<Rule> rules, AST ast);
    void validateRule(Rule rule) throws RuleValidationException;
}

public interface PatternMatcher {
    boolean matches(Pattern pattern, ASTNode node);
    Map<String, ASTNode> getMetaVariableBindings();
    void reset();
}

// 语言解析器实现
public class JavaParser implements LanguageParser {
    private final ANTLRParser antlrParser;

    @Override
    public AST parse(String sourceCode) throws ParseException {
        try {
            // 使用ANTLR解析Java代码
            JavaLexer lexer = new JavaLexer(CharStreams.fromString(sourceCode));
            CommonTokenStream tokens = new CommonTokenStream(lexer);
            JavaParser parser = new JavaParser(tokens);

            ParseTree tree = parser.compilationUnit();

            // 转换为通用AST
            return new JavaToUniversalASTConverter().convert(tree);
        } catch (Exception e) {
            throw new ParseException("Failed to parse Java code", e);
        }
    }

    @Override
    public List<String> getSupportedExtensions() {
        return Arrays.asList(".java");
    }

    @Override
    public String getLanguageName() {
        return "java";
    }
}

// 规则引擎实现
public class DefaultRuleEngine implements RuleEngine {
    private final RuleParser ruleParser;
    private final RuleValidator ruleValidator;
    private final PatternMatcher patternMatcher;

    @Override
    public List<Rule> loadRules(String ruleSource) throws RuleLoadException {
        try {
            List<Rule> rules = ruleParser.parseRules(ruleSource);

            // 验证规则
            for (Rule rule : rules) {
                ruleValidator.validate(rule);
            }

            return rules;
        } catch (Exception e) {
            throw new RuleLoadException("Failed to load rules", e);
        }
    }

    @Override
    public List<Match> executeRules(List<Rule> rules, AST ast) {
        List<Match> matches = new ArrayList<>();

        for (Rule rule : rules) {
            matches.addAll(executeRule(rule, ast));
        }

        return matches;
    }

    private List<Match> executeRule(Rule rule, AST ast) {
        List<Match> matches = new ArrayList<>();

        // 遍历AST节点
        ast.traverse(node -> {
            patternMatcher.reset();

            for (Pattern pattern : rule.getPatterns()) {
                if (patternMatcher.matches(pattern, node)) {
                    Match match = new Match(
                        rule.getId(),
                        node.getLocation(),
                        patternMatcher.getMetaVariableBindings()
                    );
                    matches.add(match);
                }
            }
        });

        return matches;
    }
}
```

### 6.3 数据流分析实现

```java
public class TaintAnalysisEngine {
    private final ControlFlowGraphBuilder cfgBuilder;
    private final DataFlowAnalyzer dataFlowAnalyzer;

    public List<TaintFlow> analyzeTaintFlow(AST ast, TaintRule rule) {
        // 1. 构建控制流图
        ControlFlowGraph cfg = cfgBuilder.build(ast);

        // 2. 识别污点源
        Set<ASTNode> sources = findTaintSources(ast, rule.getSources());

        // 3. 识别污点汇
        Set<ASTNode> sinks = findTaintSinks(ast, rule.getSinks());

        // 4. 识别净化器
        Set<ASTNode> sanitizers = findSanitizers(ast, rule.getSanitizers());

        // 5. 执行污点分析
        return dataFlowAnalyzer.analyze(cfg, sources, sinks, sanitizers);
    }

    private Set<ASTNode> findTaintSources(AST ast, List<Pattern> sourcePatterns) {
        Set<ASTNode> sources = new HashSet<>();

        for (Pattern pattern : sourcePatterns) {
            ast.traverse(node -> {
                if (patternMatcher.matches(pattern, node)) {
                    sources.add(node);
                }
            });
        }

        return sources;
    }

    // 类似的方法用于查找sinks和sanitizers
}

public class DataFlowAnalyzer {
    public List<TaintFlow> analyze(ControlFlowGraph cfg,
                                   Set<ASTNode> sources,
                                   Set<ASTNode> sinks,
                                   Set<ASTNode> sanitizers) {

        Map<ASTNode, TaintState> taintStates = new HashMap<>();
        Queue<ASTNode> worklist = new LinkedList<>();

        // 初始化污点源
        for (ASTNode source : sources) {
            taintStates.put(source, TaintState.TAINTED);
            worklist.add(source);
        }

        // 工作列表算法
        while (!worklist.isEmpty()) {
            ASTNode current = worklist.poll();
            TaintState currentState = taintStates.get(current);

            // 检查是否为净化器
            if (sanitizers.contains(current)) {
                taintStates.put(current, TaintState.CLEAN);
                continue;
            }

            // 传播污点到后继节点
            for (ASTNode successor : cfg.getSuccessors(current)) {
                TaintState newState = propagateTaint(currentState, current, successor);
                TaintState existingState = taintStates.get(successor);

                if (newState != existingState) {
                    taintStates.put(successor, newState);
                    worklist.add(successor);
                }
            }
        }

        // 检查污点是否到达汇点
        List<TaintFlow> flows = new ArrayList<>();
        for (ASTNode sink : sinks) {
            if (taintStates.get(sink) == TaintState.TAINTED) {
                TaintFlow flow = constructTaintFlow(sink, taintStates, cfg);
                flows.add(flow);
            }
        }

        return flows;
    }

    private TaintState propagateTaint(TaintState currentState, ASTNode from, ASTNode to) {
        // 实现污点传播逻辑
        if (currentState == TaintState.TAINTED && hasDataDependency(from, to)) {
            return TaintState.TAINTED;
        }
        return TaintState.CLEAN;
    }
}
```

### 6.4 性能优化策略

#### 6.4.1 解析优化
```java
public class OptimizedParser {
    private final Cache<String, AST> astCache;
    private final ExecutorService parseExecutor;

    public CompletableFuture<AST> parseAsync(String sourceCode, String language) {
        // 检查缓存
        String cacheKey = computeHash(sourceCode);
        AST cachedAST = astCache.getIfPresent(cacheKey);
        if (cachedAST != null) {
            return CompletableFuture.completedFuture(cachedAST);
        }

        // 异步解析
        return CompletableFuture.supplyAsync(() -> {
            LanguageParser parser = getParser(language);
            AST ast = parser.parse(sourceCode);
            astCache.put(cacheKey, ast);
            return ast;
        }, parseExecutor);
    }

    // 增量解析支持
    public AST parseIncremental(String sourceCode, AST previousAST, List<TextEdit> edits) {
        // 使用Tree-sitter的增量解析能力
        TreeSitterParser parser = new TreeSitterParser();
        return parser.parseIncremental(sourceCode, previousAST, edits);
    }
}
```

#### 6.4.2 规则执行优化
```java
public class OptimizedRuleEngine {
    private final RuleIndex ruleIndex;
    private final ParallelExecutor parallelExecutor;

    public List<Match> executeRulesOptimized(List<Rule> rules, AST ast) {
        // 1. 规则预过滤
        List<Rule> relevantRules = ruleIndex.getRelevantRules(ast);

        // 2. 并行执行
        List<CompletableFuture<List<Match>>> futures = relevantRules.stream()
            .map(rule -> CompletableFuture.supplyAsync(
                () -> executeRule(rule, ast),
                parallelExecutor
            ))
            .collect(Collectors.toList());

        // 3. 收集结果
        return futures.stream()
            .map(CompletableFuture::join)
            .flatMap(List::stream)
            .collect(Collectors.toList());
    }
}

public class RuleIndex {
    private final Map<String, List<Rule>> rulesByLanguage;
    private final Map<String, List<Rule>> rulesByKeyword;
    private final BloomFilter<String> ruleFilter;

    public List<Rule> getRelevantRules(AST ast) {
        String language = ast.getLanguage();
        Set<String> keywords = extractKeywords(ast);

        List<Rule> candidates = rulesByLanguage.get(language);
        if (candidates == null) {
            return Collections.emptyList();
        }

        return candidates.stream()
            .filter(rule -> isRelevant(rule, keywords))
            .collect(Collectors.toList());
    }

    private boolean isRelevant(Rule rule, Set<String> keywords) {
        // 使用布隆过滤器快速排除不相关规则
        for (String keyword : rule.getKeywords()) {
            if (ruleFilter.mightContain(keyword) && keywords.contains(keyword)) {
                return true;
            }
        }
        return false;
    }
}
```

## 7. 部署和集成设计

### 7.1 部署架构

#### 7.1.1 单机部署
```yaml
# docker-compose.yml
version: '3.8'
services:
  code-analyzer:
    image: code-analyzer:latest
    ports:
      - "8080:8080"
    environment:
      - JAVA_OPTS=-Xmx4g -Xms2g
      - ANALYZER_THREADS=8
      - CACHE_SIZE=1000
    volumes:
      - ./rules:/app/rules
      - ./cache:/app/cache
      - ./logs:/app/logs
    command: ["java", "-jar", "code-analyzer.jar", "--server"]

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

volumes:
  redis_data:
```

#### 7.1.2 分布式部署
```yaml
# kubernetes部署配置
apiVersion: apps/v1
kind: Deployment
metadata:
  name: code-analyzer
spec:
  replicas: 3
  selector:
    matchLabels:
      app: code-analyzer
  template:
    metadata:
      labels:
        app: code-analyzer
    spec:
      containers:
      - name: analyzer
        image: code-analyzer:latest
        ports:
        - containerPort: 8080
        env:
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        - name: ANALYZER_MODE
          value: "distributed"
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5

---
apiVersion: v1
kind: Service
metadata:
  name: code-analyzer-service
spec:
  selector:
    app: code-analyzer
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

### 7.2 CI/CD集成

#### 7.2.1 GitHub Actions集成
```yaml
# .github/workflows/code-analysis.yml
name: Code Security Analysis

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  security-scan:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Setup Code Analyzer
      run: |
        wget https://releases.example.com/code-analyzer/latest/code-analyzer.jar
        chmod +x code-analyzer.jar

    - name: Run Security Analysis
      run: |
        java -jar code-analyzer.jar \
          --rules security-rules.yml \
          --target . \
          --format sarif \
          --output security-results.sarif \
          --exclude "test/**,docs/**"

    - name: Upload SARIF results
      uses: github/codeql-action/upload-sarif@v2
      with:
        sarif_file: security-results.sarif

    - name: Check for critical issues
      run: |
        CRITICAL_COUNT=$(jq '.runs[0].results | map(select(.level == "error")) | length' security-results.sarif)
        if [ "$CRITICAL_COUNT" -gt 0 ]; then
          echo "Found $CRITICAL_COUNT critical security issues"
          exit 1
        fi
```

### 7.3 IDE集成

#### 7.3.1 VS Code扩展
```typescript
// VS Code扩展主文件
import * as vscode from 'vscode';
import { CodeAnalyzerClient } from './analyzer-client';

export function activate(context: vscode.ExtensionContext) {
    const client = new CodeAnalyzerClient();

    // 注册命令
    const analyzeCommand = vscode.commands.registerCommand(
        'codeAnalyzer.analyze',
        async () => {
            const activeEditor = vscode.window.activeTextEditor;
            if (!activeEditor) {
                return;
            }

            const document = activeEditor.document;
            const results = await client.analyzeDocument(document);

            // 显示诊断信息
            const diagnostics = results.map(result => {
                const range = new vscode.Range(
                    result.location.start.line,
                    result.location.start.column,
                    result.location.end.line,
                    result.location.end.column
                );

                const diagnostic = new vscode.Diagnostic(
                    range,
                    result.message,
                    getSeverity(result.severity)
                );

                diagnostic.source = 'Code Analyzer';
                diagnostic.code = result.ruleId;

                return diagnostic;
            });

            const collection = vscode.languages.createDiagnosticCollection('codeAnalyzer');
            collection.set(document.uri, diagnostics);
        }
    );

    context.subscriptions.push(analyzeCommand);
}
```

## 8. 测试策略

### 8.1 单元测试框架

#### 8.1.1 规则测试
```java
@TestMethodOrder(OrderAnnotation.class)
public class RuleTestFramework {

    @Test
    @Order(1)
    public void testJavaSQLInjectionRule() {
        // 准备测试数据
        String vulnerableCode = """
            public void getUserData(String userId) {
                String query = "SELECT * FROM users WHERE id = " + userId;
                Statement stmt = connection.createStatement();
                ResultSet rs = stmt.executeQuery(query);
            }
            """;

        String safeCode = """
            public void getUserData(String userId) {
                String query = "SELECT * FROM users WHERE id = ?";
                PreparedStatement stmt = connection.prepareStatement(query);
                stmt.setString(1, userId);
                ResultSet rs = stmt.executeQuery();
            }
            """;

        // 加载规则
        Rule sqlInjectionRule = loadRule("java-sql-injection.yml");

        // 测试漏洞代码
        List<Match> vulnerableMatches = analyzeCode(vulnerableCode, sqlInjectionRule);
        assertThat(vulnerableMatches).hasSize(1);
        assertThat(vulnerableMatches.get(0).getRuleId()).isEqualTo("java-sql-injection");

        // 测试安全代码
        List<Match> safeMatches = analyzeCode(safeCode, sqlInjectionRule);
        assertThat(safeMatches).isEmpty();
    }

    @Test
    @Order(2)
    public void testJavaScriptXSSRule() {
        String vulnerableCode = """
            function displayUserInput(input) {
                document.getElementById('output').innerHTML = input;
            }

            const userInput = location.search.substring(1);
            displayUserInput(userInput);
            """;

        String safeCode = """
            function displayUserInput(input) {
                document.getElementById('output').textContent = input;
            }

            const userInput = location.search.substring(1);
            displayUserInput(DOMPurify.sanitize(userInput));
            """;

        Rule xssRule = loadRule("javascript-xss.yml");

        List<Match> vulnerableMatches = analyzeCode(vulnerableCode, xssRule);
        assertThat(vulnerableMatches).hasSize(1);

        List<Match> safeMatches = analyzeCode(safeCode, xssRule);
        assertThat(safeMatches).isEmpty();
    }

    private List<Match> analyzeCode(String code, Rule rule) {
        try {
            LanguageParser parser = getParser(rule.getLanguage());
            AST ast = parser.parse(code);
            RuleEngine engine = new DefaultRuleEngine();
            return engine.executeRules(Arrays.asList(rule), ast);
        } catch (Exception e) {
            throw new RuntimeException("Analysis failed", e);
        }
    }
}
```

### 8.2 性能测试

#### 8.2.1 基准测试
```java
@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.MILLISECONDS)
@State(Scope.Benchmark)
public class PerformanceBenchmark {

    private RuleEngine ruleEngine;
    private List<Rule> rules;
    private AST largeAST;

    @Setup
    public void setup() {
        ruleEngine = new DefaultRuleEngine();
        rules = loadAllRules();
        largeAST = createLargeAST(10000); // 10K节点的AST
    }

    @Benchmark
    public List<Match> benchmarkRuleExecution() {
        return ruleEngine.executeRules(rules, largeAST);
    }

    @Benchmark
    public AST benchmarkJavaParsing() {
        String largeJavaFile = generateLargeJavaFile(1000); // 1000行Java代码
        JavaParser parser = new JavaParser();
        return parser.parse(largeJavaFile);
    }

    @Benchmark
    public List<TaintFlow> benchmarkTaintAnalysis() {
        TaintAnalysisEngine engine = new TaintAnalysisEngine();
        TaintRule taintRule = loadTaintRule("sql-injection-taint.yml");
        return engine.analyzeTaintFlow(largeAST, taintRule);
    }
}
```

## 9. 监控和运维

### 9.1 关键性能指标

| 指标类别 | 指标名称 | 描述 | 目标值 |
|----------|----------|------|--------|
| 吞吐量 | files_per_second | 每秒分析文件数 | >50 |
| 吞吐量 | lines_per_second | 每秒分析代码行数 | >10000 |
| 延迟 | avg_analysis_time | 平均分析时间 | <2s |
| 延迟 | p99_analysis_time | 99%分位分析时间 | <10s |
| 资源 | memory_usage | 内存使用率 | <80% |
| 资源 | cpu_usage | CPU使用率 | <70% |
| 准确性 | false_positive_rate | 误报率 | <5% |
| 可用性 | uptime | 系统可用时间 | >99.9% |

### 9.2 监控实现

```java
@Component
public class MetricsCollector {
    private final MeterRegistry meterRegistry;
    private final Timer analysisTimer;
    private final Counter filesProcessed;
    private final Gauge memoryUsage;

    public MetricsCollector(MeterRegistry meterRegistry) {
        this.meterRegistry = meterRegistry;
        this.analysisTimer = Timer.builder("analysis.time")
            .description("Time taken to analyze files")
            .register(meterRegistry);
        this.filesProcessed = Counter.builder("files.processed")
            .description("Number of files processed")
            .register(meterRegistry);
        this.memoryUsage = Gauge.builder("memory.usage")
            .description("Memory usage percentage")
            .register(meterRegistry, this, MetricsCollector::getMemoryUsage);
    }

    public void recordAnalysisTime(Duration duration) {
        analysisTimer.record(duration);
    }

    public void incrementFilesProcessed() {
        filesProcessed.increment();
    }

    private double getMemoryUsage() {
        Runtime runtime = Runtime.getRuntime();
        long totalMemory = runtime.totalMemory();
        long freeMemory = runtime.freeMemory();
        return ((double) (totalMemory - freeMemory) / totalMemory) * 100;
    }
}
```

## 10. 规则库设计

### 10.1 规则分类体系

```yaml
# 规则分类结构
rule_categories:
  security:
    - injection:
        - sql_injection
        - code_injection
        - command_injection
        - ldap_injection
    - xss:
        - reflected_xss
        - stored_xss
        - dom_xss
    - authentication:
        - weak_passwords
        - missing_authentication
        - session_management
    - authorization:
        - privilege_escalation
        - access_control
        - path_traversal
    - cryptography:
        - weak_encryption
        - insecure_random
        - certificate_validation

  performance:
    - inefficient_algorithms
    - resource_leaks
    - unnecessary_computations
    - database_performance

  maintainability:
    - code_complexity
    - naming_conventions
    - documentation
    - design_patterns
```

### 10.2 语言特定规则集

#### 10.2.1 Java规则集
```yaml
# java-security-rules.yml
rules:
  - id: java-sql-injection-statement
    name: "SQL注入 - Statement"
    severity: "HIGH"
    languages: ["java"]
    patterns:
      - pattern: "$STMT.execute($QUERY)"
      - pattern: "$STMT.executeQuery($QUERY)"
      - pattern: "$STMT.executeUpdate($QUERY)"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern_either:
                - pattern: "$STR + $VAR"
                - pattern: "String.format($STR, ..., $VAR, ...)"
    message: "SQL注入漏洞：使用动态构建的SQL查询"
    fix: "使用PreparedStatement: PreparedStatement stmt = conn.prepareStatement(\"SELECT * FROM table WHERE id = ?\"); stmt.setString(1, userInput);"

  - id: java-deserialization-vulnerability
    name: "不安全的反序列化"
    severity: "CRITICAL"
    languages: ["java"]
    patterns:
      - pattern_either:
          - pattern: "ObjectInputStream.readObject()"
          - pattern: "XMLDecoder.readObject()"
          - pattern: "Yaml.load($INPUT)"
    message: "不安全的反序列化可能导致远程代码执行"
    fix: "使用安全的序列化库或实现自定义的安全检查"
```

#### 10.2.2 JavaScript规则集
```yaml
# javascript-security-rules.yml
rules:
  - id: javascript-eval-injection
    name: "代码注入 - eval"
    severity: "CRITICAL"
    languages: ["javascript", "typescript"]
    patterns:
      - pattern: "eval($CODE)"
      - metavariable_pattern:
          metavariable: "$CODE"
          patterns:
            - pattern_either:
                - pattern: "$STR + $INPUT"
                - pattern: "`$STR${$INPUT}$STR2`"
    message: "使用eval执行动态代码可能导致代码注入"
    fix: "避免使用eval，使用JSON.parse()解析数据或Function构造器"

  - id: javascript-prototype-pollution
    name: "原型污染"
    severity: "HIGH"
    languages: ["javascript", "typescript"]
    patterns:
      - pattern: "$OBJ.__proto__ = $VALUE"
      - pattern: "$OBJ['__proto__'] = $VALUE"
      - pattern: "$OBJ.constructor.prototype = $VALUE"
    message: "原型污染可能导致应用程序逻辑被篡改"
    fix: "使用Object.create(null)创建无原型对象，或验证属性名"
```

#### 10.2.3 Python规则集
```yaml
# python-security-rules.yml
rules:
  - id: python-pickle-deserialization
    name: "不安全的Pickle反序列化"
    severity: "CRITICAL"
    languages: ["python"]
    patterns:
      - pattern: "pickle.loads($DATA)"
      - pattern: "pickle.load($FILE)"
      - pattern: "cPickle.loads($DATA)"
    message: "Pickle反序列化不受信任的数据可能导致代码执行"
    fix: "使用json.loads()或其他安全的序列化格式"

  - id: python-sql-injection-format
    name: "SQL注入 - 字符串格式化"
    severity: "HIGH"
    languages: ["python"]
    patterns:
      - pattern: "$CURSOR.execute($QUERY)"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern_either:
                - pattern: "$STR % $VARS"
                - pattern: "$STR.format(...)"
                - pattern: "f\"...$VAR...\""
    message: "使用字符串格式化构建SQL查询可能导致SQL注入"
    fix: "使用参数化查询: cursor.execute(\"SELECT * FROM table WHERE id = %s\", (user_id,))"
```

#### 10.2.4 SQL规则集
```yaml
# sql-security-rules.yml
rules:
  - id: sql-dynamic-query-execution
    name: "动态SQL执行"
    severity: "HIGH"
    languages: ["sql"]
    patterns:
      - pattern: "EXECUTE IMMEDIATE $QUERY"
      - pattern: "EXEC($QUERY)"
      - pattern: "sp_executesql $QUERY"
      - metavariable_pattern:
          metavariable: "$QUERY"
          patterns:
            - pattern_either:
                - pattern: "$STR + $VAR"
                - pattern: "CONCAT($STR, $VAR)"
    message: "动态SQL执行可能导致SQL注入"
    fix: "使用参数化查询或存储过程参数"

  - id: sql-privilege-escalation
    name: "权限提升"
    severity: "MEDIUM"
    languages: ["sql"]
    patterns:
      - pattern: "GRANT ALL PRIVILEGES TO $USER"
      - pattern: "GRANT DBA TO $USER"
      - pattern: "ALTER USER $USER IDENTIFIED BY $PASSWORD"
    message: "检测到可能的权限提升操作"
    fix: "遵循最小权限原则，只授予必要的权限"
```

#### 10.2.5 Bash规则集
```yaml
# bash-security-rules.yml
rules:
  - id: bash-command-injection-eval
    name: "命令注入 - eval"
    severity: "HIGH"
    languages: ["bash"]
    patterns:
      - pattern: "eval $INPUT"
      - pattern: "bash -c $INPUT"
      - pattern: "sh -c $INPUT"
      - metavariable_pattern:
          metavariable: "$INPUT"
          patterns:
            - pattern_either:
                - pattern: "$STR$VAR"
                - pattern: "\"$STR$VAR\""
    message: "使用eval执行动态命令可能导致命令注入"
    fix: "使用数组和适当的引号: cmd_array=(\"$safe_command\" \"$safe_arg\") && \"${cmd_array[@]}\""

  - id: bash-path-traversal
    name: "路径遍历"
    severity: "MEDIUM"
    languages: ["bash"]
    patterns:
      - pattern_either:
          - pattern: "cat $FILE"
          - pattern: "cp $SRC $DEST"
          - pattern: "mv $SRC $DEST"
          - pattern: "rm $FILE"
      - metavariable_pattern:
          metavariable: "$FILE"
          patterns:
            - pattern: "$VAR"
            - metavariable_regex:
                metavariable: "$VAR"
                regex: ".*\\$.*"  # 包含变量的路径
    message: "使用用户控制的路径可能导致路径遍历"
    fix: "验证和清理文件路径，使用realpath进行路径规范化"
```

---

本设计文档提供了一个完整的多语言静态代码分析工具的设计方案，涵盖了从架构设计到具体实现的各个方面。通过模块化的设计和统一的规则语法，该工具能够有效支持Java、Bash、SQL、JavaScript和Python等多种编程语言的安全漏洞检测，为开发团队提供强大的代码质量保障。

### 关键特性总结

1. **统一的规则语法**: 基于YAML的声明式规则定义，支持复杂的模式匹配和数据流分析
2. **模块化架构**: 清晰的分层设计，便于扩展和维护
3. **高性能**: 并行处理、缓存优化和增量分析
4. **多语言支持**: 通过语言适配器支持不同编程语言
5. **完整的工具链**: 从开发、测试到部署的完整解决方案
```
