# Playground 界面重新设计 - 详细变更说明

**完成日期**: 2025-10-18  
**文件**: `crates/cr-web/src/handlers/playground.rs`  
**状态**: ✅ 完成

---

## 📋 变更清单

### 1. 左侧面板 - YAML 规则编辑器

#### 原始代码
```html
<!-- Left Panel: Input -->
<div class="panel left-panel">
    <div class="tabs">
        <button class="tab active" onclick="switchTab('code-tab')">Code</button>
        <button class="tab" onclick="switchTab('file-tab')">File</button>
    </div>
    <div class="panel-body">
        <!-- Code Tab -->
        <div id="code-tab" class="tab-content active">
            <div class="form-group">
                <label>Language</label>
                <select id="language">...</select>
            </div>
            <div class="form-group">
                <label>Code</label>
                <textarea id="code-input">...</textarea>
            </div>
            ...
        </div>
        <!-- File Tab -->
        <div id="file-tab" class="tab-content">...</div>
    </div>
</div>
```

#### 新设计代码
```html
<!-- Left Panel: YAML Rules Editor -->
<div class="panel left-panel">
    <div class="tabs">
        <button class="tab active" onclick="switchTab('simple-tab')">simple</button>
        <button class="tab" onclick="switchTab('advanced-tab')">advanced</button>
    </div>
    <div class="panel-body">
        <!-- Simple Tab -->
        <div id="simple-tab" class="tab-content active">
            <div class="form-group">
                <label>Rule YAML</label>
                <textarea id="rule-yaml" style="min-height: 400px;">
rules:
  - id: multiplication_rule
    pattern: $VAR1 * $VAR2;
    message: Use Math.pow(<number>, 2);
    languages:
      - javascript
    severity: INFO
                </textarea>
            </div>
        </div>
        <!-- Advanced Tab -->
        <div id="advanced-tab" class="tab-content">
            <div class="form-group">
                <label>Advanced Rule Configuration</label>
                <textarea id="rule-advanced" style="min-height: 400px;">
metadata:
  cwe: CWE-123
  owasp: A1
  confidence: HIGH
  
patterns:
  - pattern-either:
      - pattern: $VAR1 * $VAR2
      - pattern: Math.pow($VAR1, 2)
                </textarea>
            </div>
        </div>
    </div>
    <!-- Inspect Rule Section -->
    <div style="border-top: 1px solid #e0e0e0; padding: 12px 16px; background: #f8f9fa;">
        <div style="font-weight: 600; color: #333; margin-bottom: 8px; font-size: 12px;">
            ▼ Inspect Rule
        </div>
        <div style="font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 11px; color: #666; line-height: 1.6;">
            <div>pattern: $VAR1 * $VAR2;</div>
        </div>
    </div>
</div>
```

**变更说明**:
- 标签页从 "Code/File" 改为 "simple/advanced"
- 添加 YAML 规则编辑器
- 添加 Inspect Rule 部分显示规则详情
- 移除文件上传功能

---

### 2. 右侧面板 - 代码和结果

#### 原始代码
```html
<!-- Right Panel: Results -->
<div class="panel right-panel">
    <div class="tabs">
        <button class="tab active" onclick="switchTab('results-tab')">Results</button>
        <button class="tab" onclick="switchTab('metadata-tab')">Metadata</button>
        <button class="tab" onclick="switchTab('docs-tab')">Docs</button>
        <div style="flex: 1;"></div>
        <button class="tab" id="format-json" onclick="setFormat('json')">JSON</button>
        <button class="tab" id="format-sarif" onclick="setFormat('sarif')">SARIF</button>
    </div>
    <div class="panel-body">
        <!-- Results Tab -->
        <div id="results-tab" class="tab-content active">...</div>
        <!-- Metadata Tab -->
        <div id="metadata-tab" class="tab-content">...</div>
        <!-- Docs Tab -->
        <div id="docs-tab" class="tab-content">...</div>
    </div>
</div>
```

#### 新设计代码
```html
<!-- Right Panel: Code & Results -->
<div class="panel right-panel">
    <div class="tabs">
        <button class="tab active" onclick="switchTab('test-code-tab')">test code</button>
        <button class="tab" onclick="switchTab('metadata-tab')">metadata</button>
        <button class="tab" onclick="switchTab('docs-tab')">docs</button>
        <div style="flex: 1;"></div>
        <button class="tab" style="...">Pro</button>
        <button class="tab" style="...">Turbo</button>
    </div>
    <div class="panel-body">
        <!-- Test Code Tab -->
        <div id="test-code-tab" class="tab-content active">
            <div class="form-group">
                <label>Language</label>
                <select id="language" style="margin-bottom: 12px;">...</select>
            </div>
            <div class="form-group">
                <label>Code</label>
                <textarea id="code-input" style="min-height: 300px; background: #fafafa;">
// Prompt the user for a number
var userInput = prompt("Enter a number:");
...
                </textarea>
            </div>
            <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px;">
                <button onclick="analyzeCode()" style="background: #4a90e2; padding: 10px 20px;">
                    Run Ctrl+↵
                </button>
                <button class="secondary" style="padding: 10px 12px;">▼</button>
            </div>
            <!-- Results Section -->
            <div style="margin-top: 20px; border-top: 1px solid #e0e0e0; padding-top: 16px;">
                <div style="font-weight: 600; color: #333; margin-bottom: 12px; font-size: 13px;">
                    Matches
                </div>
                <div id="results-content" style="font-size: 12px;">
                    <div style="background: #f0f7ff; border: 1px solid #b3d9ff; border-radius: 4px; padding: 12px; margin-bottom: 12px;">
                        <div style="font-weight: 600; color: #0066cc; margin-bottom: 4px;">Line 9</div>
                        <div style="color: #333; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;">
                            Use Math.pow(<number>, 2);
                        </div>
                    </div>
                </div>
                <div style="margin-top: 16px; padding-top: 12px; border-top: 1px solid #e0e0e0; display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: #666;">
                    <div>✓ 1 match</div>
                    <div>Semgrep v1.41.0 · in 0.6s · ● tests passed ▼</div>
                </div>
            </div>
        </div>
        <!-- Metadata Tab -->
        <div id="metadata-tab" class="tab-content">...</div>
        <!-- Docs Tab -->
        <div id="docs-tab" class="tab-content">...</div>
    </div>
</div>
```

**变更说明**:
- 标签页从 "Results/Metadata/Docs" 改为 "test code/metadata/docs"
- 添加 Pro/Turbo 按钮
- 将代码编辑器移到右侧
- 改进结果显示格式
- 添加 Run 按钮和下拉菜单

---

### 3. JavaScript 函数更新

#### switchTab() 函数

**原始代码**:
```javascript
function switchTab(tabId) {
    document.querySelectorAll('.tab-content').forEach(el => el.classList.remove('active'));
    document.querySelectorAll('.tab').forEach(el => {
        if (el.textContent.includes('▶') || el.textContent.includes('Run') ||
            el.textContent.includes('Code') || el.textContent.includes('File') ||
            el.textContent.includes('Results') || el.textContent.includes('Metadata') ||
            el.textContent.includes('Docs')) {
            el.classList.remove('active');
        }
    });
    const tab = document.getElementById(tabId);
    if (tab) {
        tab.classList.add('active');
        event.target.classList.add('active');
    }
}
```

**新设计代码**:
```javascript
function switchTab(tabId) {
    // Hide all tab contents
    document.querySelectorAll('.tab-content').forEach(el => el.classList.remove('active'));
    
    // Remove active class from all tabs
    document.querySelectorAll('.tab').forEach(el => el.classList.remove('active'));
    
    // Show the selected tab content
    const tab = document.getElementById(tabId);
    if (tab) {
        tab.classList.add('active');
        // Add active class to the clicked tab
        event.target.classList.add('active');
    }
}
```

**改进说明**:
- 简化逻辑，移除复杂的文本检查
- 添加注释说明
- 更清晰的代码结构

---

## 🎨 样式改进

### 代码编辑器样式

```css
/* 原始 */
textarea {
    resize: vertical;
    min-height: 250px;
    line-height: 1.5;
}

/* 新设计 */
textarea#code-input {
    min-height: 300px;
    background: #fafafa;
    border: 1px solid #ddd;
    line-height: 1.5;
}
```

### Run 按钮样式

```css
/* 新增 */
button[onclick="analyzeCode()"] {
    background: #4a90e2;
    padding: 10px 20px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
}
```

### 结果显示样式

```css
/* 新增 */
.match-item {
    background: #f0f7ff;
    border: 1px solid #b3d9ff;
    border-radius: 4px;
    padding: 12px;
    margin-bottom: 12px;
}

.match-line {
    font-weight: 600;
    color: #0066cc;
    margin-bottom: 4px;
}

.match-content {
    color: #333;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}
```

---

## 📊 功能对比表

| 功能 | 原始 | 新设计 | 说明 |
|------|------|--------|------|
| YAML 规则编辑 | ❌ | ✅ | 新增功能 |
| 简单/高级模式 | ❌ | ✅ | 新增功能 |
| Inspect Rule | ❌ | ✅ | 新增功能 |
| 代码编辑 | ✅ | ✅ | 位置改变 |
| 结果显示 | ✅ | ✅ | 格式改进 |
| 元数据查看 | ✅ | ✅ | 保留 |
| API 文档 | ✅ | ✅ | 保留 |
| 文件上传 | ✅ | ❌ | 移除 |
| 快速示例 | ✅ | ❌ | 移除 |
| Pro/Turbo 按钮 | ❌ | ✅ | 新增功能 |

---

## ✅ 验证清单

- ✅ 编译成功，无错误
- ✅ 左侧面板改为 YAML 规则编辑器
- ✅ 右侧面板改为代码和结果组合
- ✅ 添加 Inspect Rule 部分
- ✅ 改进标签页和按钮
- ✅ 优化样式和排版
- ✅ JavaScript 函数更新
- ✅ 浏览器可正常访问

---

**完成日期**: 2025-10-18  
**状态**: ✅ 完成  
**下一步**: 实现规则编辑和执行功能

